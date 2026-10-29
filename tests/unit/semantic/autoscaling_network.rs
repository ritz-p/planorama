use crate::{
    model::{EdgeKind, Placement},
    plan, semantic,
};
use serde_json::{Value, json};

const FIXTURE: &str = include_str!("../../fixtures/autoscaling-network-plan.json");
fn fixture() -> Value {
    serde_json::from_str(FIXTURE).unwrap()
}
fn graph(v: &Value) -> crate::model::ArchitectureGraph {
    semantic::transform(&plan::parse(&v.to_string()).unwrap())
}
fn complete(v: &Value, address: &str, attribute: &str) -> bool {
    graph(v)
        .nodes
        .iter()
        .find(|n| n.address == address)
        .unwrap()
        .references
        .iter()
        .any(|r| {
            r.attribute == attribute
                && r.complete
                && r.label == super::label(address.split('.').next().unwrap(), attribute).unwrap()
        })
}
fn parent(v: &Value) -> Option<String> {
    let g = graph(v);
    g.edges
        .iter()
        .find(|e| {
            e.kind == EdgeKind::Containment && g.nodes[e.to].address == "aws_autoscaling_group.app"
        })
        .map(|e| g.nodes[e.from].address.clone())
}

#[test]
fn verified_network_launch_and_traffic_keep_resource_identities() {
    let v = fixture();
    for (address, fields) in [
        (
            "aws_autoscaling_group.app",
            vec![
                "vpc_zone_identifier",
                "launch_configuration",
                "load_balancers",
                "target_group_arns",
                "traffic_source.identifier",
            ],
        ),
        (
            "aws_launch_configuration.app",
            vec![
                "image_id",
                "security_groups",
                "iam_instance_profile",
                "key_name",
            ],
        ),
        (
            "aws_autoscaling_attachment.classic",
            vec!["autoscaling_group_name", "elb"],
        ),
        (
            "aws_autoscaling_attachment.target",
            vec!["autoscaling_group_name", "lb_target_group_arn"],
        ),
        (
            "aws_autoscaling_traffic_source_attachment.app",
            vec!["autoscaling_group_name", "traffic_source.identifier"],
        ),
    ] {
        for field in fields {
            assert!(complete(&v, address, field), "{address}: {field}");
        }
    }
    assert_eq!(parent(&v).as_deref(), Some("aws_vpc.main"));
    let g = graph(&v);
    assert_eq!(g.coverage.selected.resources, 15);
    let lc = g
        .nodes
        .iter()
        .find(|n| n.address == "aws_launch_configuration.app")
        .unwrap();
    assert_eq!(lc.placement, Placement::Logical);
    assert!(lc.member_owner.is_none());
    assert!(!g.nodes.iter().any(|n| n.resource_type == "aws_instance"));
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
        assert!(!out.contains("PRIVATE_BOOTSTRAP"));
        assert!(out.contains("Launch configuration"));
    }
}

#[test]
fn incomplete_or_incompatible_subnet_evidence_cannot_place_group() {
    for case in 0..12 {
        let mut v = fixture();
        match case {
            0 => v["resource_changes"][11]["change"]["after"]["vpc_zone_identifier"] = json!([]),
            1 => {
                v["resource_changes"][11]["change"]["after"]["vpc_zone_identifier"] =
                    json!(["subnet-a"])
            }
            2 => {
                v["resource_changes"][11]["change"]["after"]["vpc_zone_identifier"] =
                    json!(["subnet-a", "subnet-a"])
            }
            3 => {
                v["resource_changes"][11]["change"]["after_unknown"] =
                    json!({"vpc_zone_identifier":[false,true]})
            }
            4 => v["resource_changes"][2]["change"]["actions"] = json!(["delete"]),
            5 => v["resource_changes"][2]["deposed"] = json!("old"),
            6 => v["resource_changes"][2]["mode"] = json!("data"),
            7 => v["resource_changes"][2]["change"]["after_unknown"] = json!({"id":true}),
            8 => {
                v["configuration"]["root_module"]["resources"][11]["expressions"]["vpc_zone_identifier"] =
                    json!({"constant_value":["subnet-a","subnet-b"]})
            }
            9 => {
                v["configuration"]["root_module"]["resources"][11]["expressions"]["vpc_zone_identifier"] =
                    json!({"references":["aws_subnet.a.arn","aws_subnet.b.arn"]})
            }
            10 => {
                v["configuration"]["provider_config"]["aws.other"] = json!({"full_name":"registry.terraform.io/hashicorp/aws","expressions":{"region":{"constant_value":"us-west-2"}}});
                v["configuration"]["root_module"]["resources"][2]["provider_config_key"] =
                    json!("aws.other");
            }
            _ => v["resource_changes"][11]["change"]["actions"] = json!(["delete"]),
        }
        assert!(
            !complete(&v, "aws_autoscaling_group.app", "vpc_zone_identifier"),
            "case {case}"
        );
        assert_eq!(parent(&v), None, "case {case}");
    }
    let mut v = fixture();
    v["resource_changes"][2]["change"]["after"]["vpc_id"] = json!("vpc-other");
    assert_eq!(parent(&v), None);
}

#[test]
fn traffic_source_type_and_complete_collection_are_required() {
    for case in 0..5 {
        let mut v = fixture();
        match case {
            0 => {
                v["resource_changes"][11]["change"]["after"]["traffic_source"][0]["type"] =
                    json!("elbv2")
            }
            1 => {
                v["resource_changes"][11]["change"]["after_unknown"] =
                    json!({"traffic_source":[{"type":true}]})
            }
            2 => {
                v["resource_changes"][11]["change"]["after"]["traffic_source"][0]["identifier"] =
                    json!("unknown")
            }
            3 => {
                let entry =
                    v["resource_changes"][11]["change"]["after"]["traffic_source"][0].clone();
                v["resource_changes"][11]["change"]["after"]["traffic_source"] =
                    json!([entry, entry]);
            }
            _ => {
                v["resource_changes"][11]["change"]["after"]["traffic_source"][0]["type"] =
                    Value::Null
            }
        }
        assert!(
            !complete(&v, "aws_autoscaling_group.app", "traffic_source.identifier"),
            "case {case}"
        );
    }
}

#[test]
fn launch_and_attachment_proofs_reject_wrong_values_outputs_and_lifecycle() {
    for (consumer, field, source) in [
        (7, "image_id", 4),
        (7, "iam_instance_profile", 5),
        (7, "key_name", 6),
        (11, "launch_configuration", 7),
        (12, "autoscaling_group_name", 11),
        (12, "elb", 8),
        (13, "lb_target_group_arn", 9),
    ] {
        for case in 0..8 {
            let mut v = fixture();
            let address = v["resource_changes"][consumer]["address"]
                .as_str()
                .unwrap()
                .to_owned();
            match case {
                0 => v["resource_changes"][consumer]["change"]["after"][field] = json!("unmatched"),
                1 => {
                    v["resource_changes"][consumer]["change"]["after_unknown"][field] = json!(true)
                }
                2 => v["resource_changes"][source]["change"]["actions"] = json!(["delete"]),
                3 => v["resource_changes"][source]["deposed"] = json!("old"),
                4 => v["resource_changes"][source]["mode"] = json!("data"),
                5 => {
                    v["resource_changes"][source]["provider_name"] = json!("example.com/custom/aws")
                }
                6 => {
                    v["configuration"]["root_module"]["resources"][consumer]["expressions"][field] =
                        json!({"constant_value":"literal"})
                }
                _ => v["resource_changes"][consumer]["mode"] = json!("data"),
            }
            assert!(
                !complete(&v, &address, field),
                "{address} {field} case {case}"
            );
        }
    }
    let mut v = fixture();
    v["configuration"]["root_module"]["resources"][12]["expressions"]["autoscaling_group_name"] =
        json!({"references":["aws_autoscaling_group.app.id"]});
    assert!(complete(
        &v,
        "aws_autoscaling_attachment.classic",
        "autoscaling_group_name"
    ));
}

#[test]
fn mixed_traffic_sources_require_matching_types_for_every_endpoint() {
    let mut v = fixture();
    let arn = v["resource_changes"][9]["change"]["after"]["arn"].clone();
    let lattice = v["resource_changes"][10]["change"]["after"]["arn"].clone();
    v["resource_changes"][11]["change"]["after"]["traffic_source"] = json!([
        {"identifier":lattice,"type":"vpc-lattice"}, {"identifier":arn,"type":"elbv2"}
    ]);
    v["configuration"]["root_module"]["resources"][11]["expressions"]["traffic_source"] = json!([
        {"identifier":{"references":["aws_vpclattice_target_group.app.arn"]}},
        {"identifier":{"references":["aws_lb_target_group.app.arn"]}}
    ]);
    assert!(complete(
        &v,
        "aws_autoscaling_group.app",
        "traffic_source.identifier"
    ));
    v["resource_changes"][11]["change"]["after"]["traffic_source"][1]["type"] =
        json!("vpc-lattice");
    assert!(!complete(
        &v,
        "aws_autoscaling_group.app",
        "traffic_source.identifier"
    ));
}

#[test]
fn additional_configuration_refs_do_not_create_instances_or_owners() {
    let cases = [
        (
            "placement_group",
            "aws_placement_group",
            "name",
            json!("placement"),
            json!("placement"),
        ),
        (
            "service_linked_role_arn",
            "aws_iam_service_linked_role",
            "arn",
            json!(
                "arn:aws:iam::123456789012:role/aws-service-role/autoscaling.amazonaws.com/AWSServiceRoleForAutoScaling"
            ),
            json!(
                "arn:aws:iam::123456789012:role/aws-service-role/autoscaling.amazonaws.com/AWSServiceRoleForAutoScaling"
            ),
        ),
        (
            "termination_policies",
            "aws_lambda_function",
            "arn",
            json!("arn:aws:lambda:us-east-1:123456789012:function:termination"),
            json!(["arn:aws:lambda:us-east-1:123456789012:function:termination"]),
        ),
        (
            "mixed_instances_policy.launch_template.override.launch_template_specification.launch_template_id",
            "aws_launch_template",
            "id",
            json!("lt-override"),
            json!([{ "launch_template":[{"override":[{"launch_template_specification":[{"launch_template_id":"lt-override"}]}]}]}]),
        ),
    ];
    for (field, kind, output, id, value) in cases {
        let mut v = fixture();
        let mut after = json!({});
        after[output] = id;
        let address = format!("{kind}.extra");
        v["resource_changes"].as_array_mut().unwrap().push(json!({"address":address,"mode":"managed","type":kind,"provider_name":"registry.terraform.io/hashicorp/aws","change":{"actions":["create"],"after":after}}));
        v["configuration"]["root_module"]["resources"].as_array_mut().unwrap().push(json!({"address":address,"mode":"managed","type":kind,"name":"extra","provider_config_key":"aws","expressions":{}}));
        v["resource_changes"][11]["change"]["after"][field.split('.').next().unwrap()] = value;
        let reference = json!({"references":[format!("{address}.{output}")]});
        if field.starts_with("mixed_instances_policy.") {
            v["configuration"]["root_module"]["resources"][11]["expressions"]["mixed_instances_policy"] = json!([{"launch_template":[{"override":[{"launch_template_specification":[{"launch_template_id":reference}]}]}]}]);
        } else {
            v["configuration"]["root_module"]["resources"][11]["expressions"][field] = reference;
        }
        assert!(complete(&v, "aws_autoscaling_group.app", field), "{field}");
        assert!(
            !graph(&v)
                .nodes
                .iter()
                .any(|n| n.resource_type == "aws_instance"
                    || n.member_owner.is_some() && n.address == address)
        );
        v["resource_changes"][15]["change"]["after_unknown"][output] = json!(true);
        assert!(!complete(&v, "aws_autoscaling_group.app", field), "{field}");
    }
}

#[test]
fn filtering_lifecycle_and_named_states_preserve_coverage() {
    let mut v = fixture();
    v["resource_changes"][11]["previous_address"] = json!("aws_autoscaling_group.old");
    v["resource_changes"][11]["change"]["actions"] = json!(["delete", "create"]);
    let mut old = v["resource_changes"][11].clone();
    old["deposed"] = json!("old");
    old["change"]["actions"] = json!(["delete"]);
    v["resource_changes"].as_array_mut().unwrap().push(old);
    let parsed = plan::parse(&v.to_string()).unwrap();
    let g = semantic::transform(&parsed);
    assert_eq!(g.coverage.selected.resources, 16);
    for source in &parsed.nodes {
        let nodes = g
            .nodes
            .iter()
            .filter(|n| n.address == source.address && n.deposed_key == source.deposed_key)
            .collect::<Vec<_>>();
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].action, source.action);
        assert_eq!(nodes[0].previous_address, source.previous_address);
    }
    let selected = crate::view::single(
        &g,
        &crate::view::Options {
            focus: Some("aws_launch_configuration.app".into()),
            depth: Some(0),
            ..Default::default()
        },
    )
    .unwrap();
    assert!(
        !selected
            .nodes
            .iter()
            .any(|n| n.address == "aws_autoscaling_attachment.classic")
    );
    assert!(
        selected
            .edges
            .iter()
            .all(|e| e.from < selected.nodes.len() && e.to < selected.nodes.len())
    );
    let mut missing = fixture();
    missing["resource_changes"]
        .as_array_mut()
        .unwrap()
        .remove(1);
    assert_eq!(parent(&missing), None);
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
fn account_evidence_is_preserved_and_conflicting_endpoint_is_unverified() {
    let mut v = fixture();
    v["configuration"]["provider_config"]["aws"]["expressions"]["assume_role"] =
        json!([{"role_arn":{"constant_value":"arn:aws:iam::123456789012:role/Test"}}]);
    let mut parsed = crate::plan::evidence::tests::parse_fixture(&v.to_string()).unwrap();
    let g = semantic::transform(&parsed);
    for n in g.nodes.iter().filter(|n| {
        matches!(
            n.resource_type.as_str(),
            "aws_launch_configuration"
                | "aws_autoscaling_attachment"
                | "aws_autoscaling_traffic_source_attachment"
        )
    }) {
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
    let subnet = parsed
        .nodes
        .iter_mut()
        .find(|n| n.address == "aws_subnet.b")
        .unwrap();
    subnet
        .provider_configuration
        .as_mut()
        .unwrap()
        .account_conflict = true;
    let g = semantic::transform(&parsed);
    assert!(
        !g.edges.iter().any(|e| e.kind == EdgeKind::Containment
            && g.nodes[e.to].address == "aws_autoscaling_group.app")
    );
}
