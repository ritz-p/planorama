use crate::{
    model::{EdgeKind, Placement},
    plan, semantic, view,
};
use serde_json::{Value, json};

const FIXTURE: &str = include_str!("../../fixtures/fsx-lustre-plan.json");
fn fixture() -> Value {
    serde_json::from_str(FIXTURE).unwrap()
}
fn graph(v: &Value) -> crate::model::ArchitectureGraph {
    semantic::transform(&plan::parse(&v.to_string()).unwrap())
}
fn complete(v: &Value, field: &str) -> bool {
    graph(v)
        .nodes
        .iter()
        .find(|n| n.address == "aws_fsx_lustre_file_system.verified")
        .unwrap()
        .references
        .iter()
        .any(|r| {
            r.attribute == field
                && r.complete
                && Some(r.label) == super::label("aws_fsx_lustre_file_system", field)
        })
}
fn parent(v: &Value) -> Option<String> {
    let g = graph(v);
    g.edges
        .iter()
        .find(|e| {
            e.kind == EdgeKind::Containment
                && g.nodes[e.to].address == "aws_fsx_lustre_file_system.verified"
        })
        .map(|e| g.nodes[e.from].address.clone())
}

#[test]
fn lustre_keeps_independent_card_with_verified_vpc_and_private_value_exclusions() {
    let v = fixture();
    for field in ["subnet_ids", "security_group_ids", "kms_key_id"] {
        assert!(complete(&v, field), "{field}");
    }
    assert_eq!(parent(&v).as_deref(), Some("aws_vpc.main"));
    let g = graph(&v);
    assert_eq!(g.coverage.selected.resources, 8);
    for n in g
        .nodes
        .iter()
        .filter(|n| n.resource_type == "aws_fsx_lustre_file_system")
    {
        assert_eq!(n.placement, Placement::Spatial);
        assert!(n.member_owner.is_none());
        if n.address != "aws_fsx_lustre_file_system.verified" {
            assert!(!n.references.iter().any(
                |r| r.complete && super::label(&n.resource_type, &r.attribute) == Some(r.label)
            ));
            assert!(
                !g.edges
                    .iter()
                    .any(|e| e.kind == EdgeKind::Containment && g.nodes[e.to].address == n.address)
            );
        }
    }
    for format in [crate::OutputFormat::Svg, crate::OutputFormat::Html] {
        let out = crate::generate(
            FIXTURE,
            &crate::Options {
                format,
                ..Default::default()
            },
        )
        .unwrap()
        .content;
        for label in [
            "Configured subnets",
            "Configured security groups",
            "Encryption key reference",
            "Amazon FSx",
        ] {
            assert!(out.contains(label), "{label}");
        }
        assert!(!out.contains("PRIVATE_"));
        assert!(!out.contains("arn:aws:fsx"));
        assert!(!out.contains("arn:aws:kms"));
    }
}

#[test]
fn lustre_rejects_unproven_values_types_lifecycle_and_scope() {
    for (field, source, output) in [
        ("subnet_ids", 1, "id"),
        ("security_group_ids", 2, "id"),
        ("kms_key_id", 4, "arn"),
    ] {
        for case in [
            "literal",
            "dynamic",
            "wrong-output",
            "mismatch",
            "unknown",
            "source-unknown",
            "wrong-type",
            "delete",
            "consumer-delete",
            "deposed",
            "consumer-deposed",
            "data",
            "consumer-data",
            "provider",
            "binding",
            "region",
            "account-conflict",
            "missing",
        ] {
            let mut v = fixture();
            match case {
                "literal" => {
                    v["configuration"]["root_module"]["resources"][5]["expressions"][field] =
                        json!({"constant_value":"literal"})
                }
                "dynamic" => {
                    v["configuration"]["root_module"]["resources"][5]["expressions"][field] =
                        json!({"references":["var.dynamic"]})
                }
                "wrong-output" => {
                    let addr = v["resource_changes"][source]["address"].as_str().unwrap();
                    v["configuration"]["root_module"]["resources"][5]["expressions"][field] = json!({"references":[format!("{addr}.{}",if output=="arn"{"id"}else{"arn"})]});
                }
                "mismatch" => {
                    v["resource_changes"][source]["change"]["after"][output] = json!("different")
                }
                "unknown" => {
                    v["resource_changes"][5]["change"]["after_unknown"] = json!({field:true})
                }
                "source-unknown" => {
                    v["resource_changes"][source]["change"]["after_unknown"] = json!({output:true})
                }
                "wrong-type" => v["resource_changes"][source]["type"] = json!("aws_s3_bucket"),
                "delete" => v["resource_changes"][source]["change"]["actions"] = json!(["delete"]),
                "consumer-delete" => {
                    v["resource_changes"][5]["change"]["actions"] = json!(["delete"])
                }
                "deposed" => v["resource_changes"][source]["deposed"] = json!("old"),
                "consumer-deposed" => v["resource_changes"][5]["deposed"] = json!("old"),
                "data" => v["resource_changes"][source]["mode"] = json!("data"),
                "consumer-data" => v["resource_changes"][5]["mode"] = json!("data"),
                "provider" => {
                    v["resource_changes"][source]["provider_name"] =
                        json!("example.org/foreign/aws")
                }
                "binding" | "region" | "account-conflict" => {
                    v["configuration"]["provider_config"]["other"] = json!({"full_name":"registry.terraform.io/hashicorp/aws","expressions":{"region":{"constant_value":if case=="region"{"us-west-2"}else{"us-east-1"}}}});
                    v["configuration"]["root_module"]["resources"][source]["provider_config_key"] =
                        json!("other");
                    if case == "account-conflict" {
                        v["configuration"]["provider_config"]["other"]["expressions"]["allowed_account_ids"] =
                            json!({"constant_value":["999999999999"]});
                    }
                }
                "missing" => v["resource_changes"][source]["address"] = json!("aws_vpc.unrelated"),
                _ => unreachable!(),
            }
            assert!(!complete(&v, field), "{field} {case}");
            if field != "kms_key_id" {
                assert!(parent(&v).is_none(), "{field} {case}");
            }
        }
    }
}

#[test]
fn lustre_requires_complete_unique_network_and_persistent_key_contract() {
    for case in [
        "extra-subnet",
        "duplicate-subnet",
        "partial-groups",
        "duplicate-groups",
        "empty-groups",
        "missing-groups",
        "unknown-element",
        "bad-mask",
        "parent-mismatch",
        "parent-delete",
        "kms-scratch",
        "kms-unknown-mode",
        "kms-missing-mode",
        "alias",
    ] {
        let mut v = fixture();
        let after = &mut v["resource_changes"][5]["change"]["after"];
        match case {
            "extra-subnet" => after["subnet_ids"]
                .as_array_mut()
                .unwrap()
                .push(json!("literal")),
            "duplicate-subnet" => after["subnet_ids"]
                .as_array_mut()
                .unwrap()
                .push(json!("subnet-app")),
            "partial-groups" => after["security_group_ids"]
                .as_array_mut()
                .unwrap()
                .push(json!("literal")),
            "duplicate-groups" => after["security_group_ids"][1] = json!("sg-second"),
            "empty-groups" => after["security_group_ids"] = json!([]),
            "missing-groups" => {
                after.as_object_mut().unwrap().remove("security_group_ids");
            }
            "unknown-element" => {
                v["resource_changes"][5]["change"]["after_unknown"] =
                    json!({"security_group_ids":[false,true]})
            }
            "bad-mask" => {
                v["resource_changes"][5]["change"]["after_unknown"] =
                    json!({"security_group_ids":[false]})
            }
            "parent-mismatch" => {
                v["resource_changes"][2]["change"]["after"]["vpc_id"] = json!("vpc-other")
            }
            "parent-delete" => v["resource_changes"][0]["change"]["actions"] = json!(["delete"]),
            "kms-scratch" => after["deployment_type"] = json!("SCRATCH_2"),
            "kms-unknown-mode" => {
                v["resource_changes"][5]["change"]["after_unknown"] =
                    json!({"deployment_type":true})
            }
            "kms-missing-mode" => {
                after.as_object_mut().unwrap().remove("deployment_type");
            }
            "alias" => v["resource_changes"][4]["type"] = json!("aws_kms_alias"),
            _ => unreachable!(),
        }
        if case.starts_with("kms-") || case == "alias" {
            assert!(!complete(&v, "kms_key_id"), "{case}");
            assert!(parent(&v).is_some());
        } else {
            assert!(parent(&v).is_none(), "{case}");
        }
    }
    for variant in [
        "aws_fsx_windows_file_system",
        "aws_fsx_openzfs_file_system",
        "aws_fsx_ontap_file_system",
    ] {
        let mut v = fixture();
        v["resource_changes"][5]["type"] = json!(variant);
        assert!(parent(&v).is_none());
        assert!(super::label(variant, "kms_key_id").is_none());
    }
    let mut v = fixture();
    v["resource_changes"][5]["change"]["after"]["deployment_type"] = json!("PERSISTENT_1");
    assert!(complete(&v, "kms_key_id"));
}

#[test]
fn lustre_preserves_account_lifecycle_filtering_and_named_state_identity() {
    let mut v = fixture();
    v["configuration"]["provider_config"]["aws"]["expressions"]["assume_role"] =
        json!([{"role_arn":{"constant_value":"arn:aws:iam::123456789012:role/Test"}}]);
    let parsed = crate::plan::evidence::tests::parse_fixture(&v.to_string()).unwrap();
    let g = semantic::transform(&parsed);
    for n in g
        .nodes
        .iter()
        .filter(|n| n.resource_type == "aws_fsx_lustre_file_system")
    {
        assert_eq!(
            n.entity
                .scope
                .as_ref()
                .unwrap()
                .account
                .as_ref()
                .unwrap()
                .as_str(),
            "123456789012"
        );
    }
    let selected = view::single(
        &g,
        &view::Options {
            focus: Some("aws_fsx_lustre_file_system.verified".into()),
            depth: Some(0),
            ..Default::default()
        },
    )
    .unwrap();
    assert!(
        !selected
            .nodes
            .iter()
            .any(|n| n.address == "aws_fsx_lustre_file_system.literal")
    );
    assert!(
        selected
            .edges
            .iter()
            .all(|e| e.from < selected.nodes.len() && e.to < selected.nodes.len())
    );
    v["resource_changes"][5]["previous_address"] = json!("aws_fsx_lustre_file_system.old");
    v["resource_changes"][5]["change"]["actions"] = json!(["delete", "create"]);
    v["resource_changes"][5]["change"]["importing"] = json!({"id":"PRIVATE_IMPORT"});
    let mut old = v["resource_changes"][5].clone();
    old["deposed"] = json!("old");
    old["change"]["actions"] = json!(["delete"]);
    v["resource_changes"].as_array_mut().unwrap().push(old);
    let parsed = plan::parse(&v.to_string()).unwrap();
    let g = semantic::transform(&parsed);
    assert_eq!(g.coverage.selected.resources, 9);
    for source in &parsed.nodes {
        let nodes = g
            .nodes
            .iter()
            .filter(|n| n.address == source.address && n.deposed_key == source.deposed_key)
            .collect::<Vec<_>>();
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].action, source.action);
        assert_eq!(nodes[0].metadata, source.metadata);
        assert_eq!(nodes[0].previous_address, source.previous_address);
    }
    assert!(
        !g.edges
            .iter()
            .any(|e| e.kind == EdgeKind::Containment && g.nodes[e.to].deposed_key.is_some())
    );
    let mut missing = fixture();
    missing["resource_changes"]
        .as_array_mut()
        .unwrap()
        .remove(1);
    assert!(parent(&missing).is_none());
    let out = crate::generate_named(
        &[
            crate::NamedInput {
                state: "one".into(),
                json: v.to_string(),
            },
            crate::NamedInput {
                state: "two".into(),
                json: missing.to_string(),
            },
        ],
        &crate::Options::default(),
    )
    .unwrap()
    .content;
    assert!(out.contains("data-state-id=\"one\""));
    assert!(out.contains("data-state-id=\"two\""));
    assert!(!out.contains("<metadata id=\"cross-state-relationships\">"));
    assert!(!out.contains("PRIVATE_"));
}

#[test]
fn lustre_group_order_is_not_identity_and_different_vpcs_do_not_combine() {
    let original = fixture();
    let mut v = original.clone();
    v["resource_changes"][5]["change"]["after"]["security_group_ids"]
        .as_array_mut()
        .unwrap()
        .reverse();
    v["resource_changes"].as_array_mut().unwrap().reverse();
    v["configuration"]["root_module"]["resources"]
        .as_array_mut()
        .unwrap()
        .reverse();
    assert_eq!(
        crate::generate(&original.to_string(), &crate::Options::default())
            .unwrap()
            .content,
        crate::generate(&v.to_string(), &crate::Options::default())
            .unwrap()
            .content
    );
    let mut v = original;
    let mut other = v["resource_changes"][0].clone();
    other["address"] = json!("aws_vpc.other");
    other["change"]["after"]["id"] = json!("vpc-other");
    v["resource_changes"].as_array_mut().unwrap().push(other);
    let mut config = v["configuration"]["root_module"]["resources"][0].clone();
    config["address"] = json!("aws_vpc.other");
    v["configuration"]["root_module"]["resources"]
        .as_array_mut()
        .unwrap()
        .push(config);
    v["resource_changes"][2]["change"]["after"]["vpc_id"] = json!("vpc-other");
    v["configuration"]["root_module"]["resources"][2]["expressions"]["vpc_id"] =
        json!({"references":["aws_vpc.other.id"]});
    assert!(complete(&v, "security_group_ids"));
    assert!(parent(&v).is_none());
}
