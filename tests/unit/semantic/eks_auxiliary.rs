use crate::{plan, semantic};
use serde_json::{Value, json};

fn input(kind: &str, field: &str) -> Value {
    let collection = field == "subnet_ids";
    let endpoint = if field == "network_acl_id" {
        "aws_network_acl"
    } else {
        "aws_subnet"
    };
    json!({"format_version":"1.2", "resource_changes":[
        {"address":format!("{endpoint}.peer"),"type":endpoint,"mode":"managed","provider_name":"registry.terraform.io/hashicorp/aws","change":{"actions":["create"],"after":{"id":"endpoint","vpc_id":"vpc-one"}}},
        {"address":format!("{kind}.main"),"type":kind,"mode":"managed","provider_name":"registry.terraform.io/hashicorp/aws","change":{"actions":["create"],"after":{field:if collection {json!(["endpoint"])} else {json!("endpoint")},"vpc_id":"vpc-one","rule_number":100,"egress":false,"protocol":"tcp","rule_action":"allow"}}}
    ],"configuration":{"provider_config":{"aws":{"name":"aws","full_name":"registry.terraform.io/hashicorp/aws","expressions":{"region":{"constant_value":"us-east-1"}}}},"root_module":{"resources":[
        {"address":format!("{endpoint}.peer"),"type":endpoint,"mode":"managed","provider_config_key":"aws","expressions":{}},
        {"address":format!("{kind}.main"),"type":kind,"mode":"managed","provider_config_key":"aws","expressions":{field:{"references":[format!("{endpoint}.peer.id")]}}}
    ]}}})
}

fn verified(v: &Value, field: &str) -> bool {
    plan::parse(&v.to_string())
        .unwrap()
        .attributes
        .iter()
        .any(|r| r.attribute == field && r.service_reference_verified)
}

#[test]
fn auxiliary_cluster_and_role_references_require_current_explicit_matching_outputs() {
    for (kind, field, endpoint, output) in [
        (
            "aws_eks_identity_provider_config",
            "cluster_name",
            "aws_eks_cluster",
            "name",
        ),
        (
            "aws_eks_access_policy_association",
            "cluster_name",
            "aws_eks_cluster",
            "name",
        ),
        (
            "aws_eks_pod_identity_association",
            "cluster_name",
            "aws_eks_cluster",
            "name",
        ),
        (
            "aws_eks_capability",
            "cluster_name",
            "aws_eks_cluster",
            "name",
        ),
        ("aws_eks_capability", "role_arn", "aws_iam_role", "arn"),
        (
            "aws_eks_pod_identity_association",
            "role_arn",
            "aws_iam_role",
            "arn",
        ),
        (
            "aws_eks_pod_identity_association",
            "target_role_arn",
            "aws_iam_role",
            "arn",
        ),
    ] {
        let mut v = input(kind, field);
        v["resource_changes"][0]["type"] = json!(endpoint);
        v["resource_changes"][0]["address"] = json!(format!("{endpoint}.peer"));
        v["resource_changes"][0]["change"]["after"] = json!({output:"endpoint"});
        v["configuration"]["root_module"]["resources"][0]["type"] = json!(endpoint);
        v["configuration"]["root_module"]["resources"][0]["address"] =
            json!(format!("{endpoint}.peer"));
        v["configuration"]["root_module"]["resources"][1]["expressions"][field] =
            json!({"references":[format!("{endpoint}.peer.{output}")]});
        assert!(verified(&v, field), "{kind} {field}");
        assert_eq!(
            semantic::transform(&plan::parse(&v.to_string()).unwrap())
                .coverage
                .selected
                .resources,
            2
        );
        for scenario in 0..8 {
            let mut bad = v.clone();
            match scenario {
                0 => bad["resource_changes"][0]["change"]["after"][output] = json!("mismatch"),
                1 => bad["resource_changes"][0]["change"]["after_unknown"][output] = json!(true),
                2 => bad["resource_changes"][1]["change"]["after_unknown"][field] = json!(true),
                3 => {
                    bad["configuration"]["root_module"]["resources"][1]["expressions"][field] =
                        json!({"constant_value":"endpoint"})
                }
                4 => bad["resource_changes"][0]["mode"] = json!("data"),
                5 => bad["resource_changes"][0]["deposed"] = json!("old"),
                6 => bad["resource_changes"][0]["change"]["actions"] = json!(["delete"]),
                _ => {
                    bad["configuration"]["root_module"]["resources"][1]["provider_config_key"] =
                        json!("aws.other")
                }
            }
            assert!(!verified(&bad, field), "{kind} {field}: {scenario}");
        }
    }
}

#[test]
fn qualified_access_template_and_addon_collections_preserve_identity() {
    let v: Value =
        serde_json::from_str(include_str!("../../fixtures/eks-auxiliary-plan.json")).unwrap();
    for field in [
        "principal_arn",
        "pod_identity_association.role_arn",
        "launch_template.id",
    ] {
        assert!(verified(&v, field), "{field}");
    }
    assert_eq!(
        semantic::transform(&plan::parse(&v.to_string()).unwrap())
            .coverage
            .selected
            .resources,
        9
    );
    for scenario in 0..6 {
        let mut bad = v.clone();
        let field = match scenario {
            0 => {
                bad["resource_changes"][2]["change"]["after"]["cluster_name"] = json!("other");
                "principal_arn"
            }
            1 => {
                bad["resource_changes"][6]["change"]["after"]["launch_template"][0]["version"] =
                    json!("$Latest");
                "launch_template.id"
            }
            2 => {
                bad["resource_changes"][5]["change"]["after"]["latest_version"] = json!(4);
                "launch_template.id"
            }
            3 => {
                bad["resource_changes"][4]["change"]["after"]["pod_identity_association"]
                    .as_array_mut()
                    .unwrap()
                    .push(json!({"role_arn":"missing"}));
                "pod_identity_association.role_arn"
            }
            4 => {
                bad["resource_changes"][4]["change"]["after_unknown"]["pod_identity_association"] =
                    json!(true);
                "pod_identity_association.role_arn"
            }
            _ => {
                bad["configuration"]["root_module"]["resources"][6]["expressions"]["launch_template"]
                    [0]["version"] = json!({"constant_value":"3"});
                "launch_template.id"
            }
        };
        let raw = plan::parse(&bad.to_string()).unwrap();
        let target = if field == "principal_arn" {
            "aws_eks_access_policy_association"
        } else if field == "launch_template.id" {
            "aws_eks_node_group"
        } else {
            "aws_eks_addon"
        };
        assert!(
            !raw.attributes
                .iter()
                .any(|r| raw.nodes[r.target].resource_type == target
                    && r.attribute == field
                    && r.service_reference_verified),
            "{scenario}"
        );
    }
    for format in [crate::OutputFormat::Svg, crate::OutputFormat::Html] {
        let out = crate::generate_named(
            &[
                crate::NamedInput {
                    state: "one".into(),
                    json: v.to_string(),
                },
                crate::NamedInput {
                    state: "two".into(),
                    json: v.to_string(),
                },
            ],
            &crate::Options {
                format,
                ..Default::default()
            },
        )
        .unwrap()
        .content;
        assert!(!out.contains("PRIVATE_POLICY"));
        assert!(!out.contains("PRIVATE_SERVICE_ACCOUNT"));
        assert!(out.contains("data-state-id=\"one\""));
        assert!(out.contains("data-state-id=\"two\""));
    }
}
