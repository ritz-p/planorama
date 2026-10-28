use crate::{
    model::{EdgeKind, Placement},
    plan, semantic, view,
};
use serde_json::{Value, json};

const FIXTURE: &str = include_str!("../../fixtures/batch-plan.json");

fn fixture() -> Value {
    serde_json::from_str(FIXTURE).unwrap()
}
fn graph(v: &Value) -> crate::model::ArchitectureGraph {
    semantic::transform(&plan::parse(&v.to_string()).unwrap())
}
fn complete(v: &Value, target: usize, field: &str) -> bool {
    graph(v)
        .nodes
        .iter()
        .find(|n| n.address == v["resource_changes"][target]["address"].as_str().unwrap())
        .unwrap()
        .references
        .iter()
        .any(|r| {
            r.attribute == field
                && r.complete
                && Some(r.label)
                    == super::label(
                        v["resource_changes"][target]["type"].as_str().unwrap(),
                        field,
                    )
        })
}
fn parent(v: &Value) -> Option<String> {
    let g = graph(v);
    g.edges
        .iter()
        .find(|e| {
            e.kind == EdgeKind::Containment
                && g.nodes[e.to].address == "aws_batch_compute_environment.ec2"
        })
        .map(|e| g.nodes[e.from].address.clone())
}

#[test]
fn batch_preserves_independent_environments_and_logical_queue_policy() {
    let v = fixture();
    for (i, f) in [
        (4, super::SUBNETS),
        (4, super::GROUPS),
        (5, super::SUBNETS),
        (7, super::ENVIRONMENTS),
        (7, "scheduling_policy_arn"),
    ] {
        assert!(complete(&v, i, f), "{i} {f}");
    }
    assert_eq!(parent(&v).as_deref(), Some("aws_vpc.main"));
    let g = graph(&v);
    assert_eq!(g.coverage.selected.resources, 9);
    for n in &g.nodes {
        assert!(n.member_owner.is_none());
        if n.resource_type == "aws_batch_compute_environment" {
            assert_eq!(n.placement, Placement::Spatial);
        }
        if matches!(
            n.resource_type.as_str(),
            "aws_batch_job_queue" | "aws_batch_scheduling_policy" | "aws_batch_job_definition"
        ) {
            assert_eq!(n.placement, Placement::Logical);
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
            "Compute environments",
            "Scheduling policy",
            "Configured subnets",
            "Configured security groups",
        ] {
            assert!(out.contains(label), "{label}");
        }
        assert!(!out.contains("PRIVATE_"));
        assert!(!out.contains("arn:aws:batch"));
    }
}

#[test]
fn batch_requires_complete_known_current_typed_compatible_references() {
    for (target, source, field) in [
        (4, 1, super::SUBNETS),
        (4, 3, super::GROUPS),
        (7, 4, super::ENVIRONMENTS),
        (7, 6, "scheduling_policy_arn"),
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
            let config = &mut v["configuration"]["root_module"]["resources"][target]["expressions"];
            let expression = match field {
                super::SUBNETS => &mut config["compute_resources"][0]["subnets"],
                super::GROUPS => &mut config["compute_resources"][0]["security_group_ids"],
                super::ENVIRONMENTS => {
                    &mut config["compute_environment_order"][0]["compute_environment"]
                }
                _ => &mut config[field],
            };
            match case {
                "literal" => *expression = json!({"constant_value":"literal"}),
                "dynamic" => *expression = json!({"references":["var.dynamic"]}),
                "wrong-output" => {
                    *expression = json!({"references":["aws_batch_compute_environment.ec2.name"]})
                }
                "mismatch" => {
                    let a = &mut v["resource_changes"][target]["change"]["after"];
                    match field {
                        super::SUBNETS => {
                            a["compute_resources"][0]["subnets"][0] = json!("different")
                        }
                        super::GROUPS => {
                            a["compute_resources"][0]["security_group_ids"][0] = json!("different")
                        }
                        super::ENVIRONMENTS => {
                            a["compute_environment_order"][0]["compute_environment"] =
                                json!("different")
                        }
                        _ => a[field] = json!("different"),
                    }
                }
                "unknown" => v["resource_changes"][target]["change"]["after_unknown"] = json!(true),
                "source-unknown" => {
                    v["resource_changes"][source]["change"]["after_unknown"] =
                        json!({"id":true,"arn":true})
                }
                "wrong-type" => {
                    v["resource_changes"][source]["type"] = json!("aws_batch_job_definition")
                }
                "delete" => v["resource_changes"][source]["change"]["actions"] = json!(["delete"]),
                "consumer-delete" => {
                    v["resource_changes"][target]["change"]["actions"] = json!(["delete"])
                }
                "deposed" => v["resource_changes"][source]["deposed"] = json!("old"),
                "consumer-deposed" => v["resource_changes"][target]["deposed"] = json!("old"),
                "data" => v["resource_changes"][source]["mode"] = json!("data"),
                "consumer-data" => v["resource_changes"][target]["mode"] = json!("data"),
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
            assert!(!complete(&v, target, field), "{field} {case}");
            if target == 4 {
                assert!(parent(&v).is_none(), "{field} {case}");
            }
        }
    }
}

#[test]
fn batch_rejects_partial_collections_masks_duplicate_orders_and_unmanaged_network() {
    for case in [
        "partial-subnet",
        "duplicate-subnet",
        "unknown-subnet",
        "unknown-block",
        "sg-absent",
        "unmanaged",
        "unknown-type",
        "wrong-type",
        "multi-block",
        "partial-queue",
        "duplicate-queue",
        "duplicate-order",
        "unknown-queue",
        "unknown-order",
        "missing-order",
        "parent-mismatch",
    ] {
        let mut v = fixture();
        let after = &mut v["resource_changes"][4]["change"]["after"];
        match case {
            "partial-subnet" => after["compute_resources"][0]["subnets"]
                .as_array_mut()
                .unwrap()
                .push(json!("literal")),
            "duplicate-subnet" => after["compute_resources"][0]["subnets"][1] = json!("subnet-a"),
            "sg-absent" => {
                after["compute_resources"][0]
                    .as_object_mut()
                    .unwrap()
                    .remove("security_group_ids");
            }
            "unmanaged" => after["type"] = json!("UNMANAGED"),
            "wrong-type" => after["compute_resources"][0]["type"] = json!("UNKNOWN"),
            "multi-block" => {
                let b = after["compute_resources"][0].clone();
                after["compute_resources"].as_array_mut().unwrap().push(b);
            }
            "unknown-type" => {
                v["resource_changes"][4]["change"]["after_unknown"] =
                    json!({"compute_resources":[{"type":true}]})
            }
            "unknown-block" => {
                v["resource_changes"][4]["change"]["after_unknown"] =
                    json!({"compute_resources":true})
            }
            "unknown-subnet" => {
                v["resource_changes"][4]["change"]["after_unknown"] =
                    json!({"compute_resources":[{"subnets":[false,true]}]})
            }
            "parent-mismatch" => {
                v["resource_changes"][3]["change"]["after"]["vpc_id"] = json!("other")
            }
            "partial-queue" => {
                v["resource_changes"][7]["change"]["after"]["compute_environment_order"]
                    .as_array_mut()
                    .unwrap()
                    .push(json!({"order":3,"compute_environment":"literal"}))
            }
            "duplicate-queue" => {
                let a = v["resource_changes"][7]["change"]["after"]["compute_environment_order"][0]
                    ["compute_environment"]
                    .clone();
                v["resource_changes"][7]["change"]["after"]["compute_environment_order"][1]["compute_environment"] =
                    a;
            }
            "duplicate-order" => {
                v["resource_changes"][7]["change"]["after"]["compute_environment_order"][1]["order"] =
                    json!(1)
            }
            "unknown-queue" => {
                v["resource_changes"][7]["change"]["after_unknown"] =
                    json!({"compute_environment_order":[{}, {"compute_environment":true}]})
            }
            "unknown-order" => {
                v["resource_changes"][7]["change"]["after_unknown"] =
                    json!({"compute_environment_order":[{}, {"order":true}]})
            }
            "missing-order" => {
                v["resource_changes"][7]["change"]["after"]["compute_environment_order"][1]["order"] =
                    Value::Null
            }
            _ => unreachable!(),
        }
        if case.contains("queue") || case.contains("order") {
            assert!(!complete(&v, 7, super::ENVIRONMENTS), "{case}");
        } else {
            assert!(parent(&v).is_none(), "{case}");
        }
    }
}

#[test]
fn batch_preserves_scope_lifecycle_selection_and_state_identity() {
    let mut v = fixture();
    v["configuration"]["provider_config"]["aws"]["expressions"]["assume_role"] =
        json!([{"role_arn":{"constant_value":"arn:aws:iam::123456789012:role/Test"}}]);
    let parsed = crate::plan::evidence::tests::parse_fixture(&v.to_string()).unwrap();
    let g = semantic::transform(&parsed);
    for n in g.nodes.iter().filter(|n| {
        matches!(
            n.resource_type.as_str(),
            "aws_batch_compute_environment" | "aws_batch_job_queue" | "aws_batch_scheduling_policy"
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
    let selected = view::single(
        &g,
        &view::Options {
            focus: Some("aws_batch_compute_environment.ec2".into()),
            depth: Some(0),
            ..Default::default()
        },
    )
    .unwrap();
    assert!(
        !selected
            .nodes
            .iter()
            .any(|n| n.address == "aws_batch_compute_environment.fargate")
    );
    assert!(
        selected
            .edges
            .iter()
            .all(|e| e.from < selected.nodes.len() && e.to < selected.nodes.len())
    );
    v["resource_changes"][4]["previous_address"] = json!("aws_batch_compute_environment.old");
    v["resource_changes"][4]["change"]["actions"] = json!(["delete", "create"]);
    v["resource_changes"][4]["change"]["importing"] = json!({"id":"PRIVATE_IMPORT"});
    let mut old = v["resource_changes"][4].clone();
    old["deposed"] = json!("old");
    old["change"]["actions"] = json!(["delete"]);
    v["resource_changes"].as_array_mut().unwrap().push(old);
    let parsed = plan::parse(&v.to_string()).unwrap();
    let g = semantic::transform(&parsed);
    assert_eq!(g.coverage.selected.resources, 10);
    for source in &parsed.nodes {
        let nodes = g
            .nodes
            .iter()
            .filter(|n| n.address == source.address && n.deposed_key == source.deposed_key)
            .collect::<Vec<_>>();
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].metadata, source.metadata);
        assert_eq!(nodes[0].action, source.action);
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
fn batch_network_requires_one_vpc_and_queue_order_is_not_list_position() {
    let original = fixture();
    let mut reordered = original.clone();
    reordered["resource_changes"][7]["change"]["after"]["compute_environment_order"]
        .as_array_mut()
        .unwrap()
        .reverse();
    reordered["configuration"]["root_module"]["resources"][7]["expressions"]["compute_environment_order"].as_array_mut().unwrap().reverse();
    assert!(complete(&reordered, 7, super::ENVIRONMENTS));
    reordered["resource_changes"]
        .as_array_mut()
        .unwrap()
        .reverse();
    reordered["configuration"]["root_module"]["resources"]
        .as_array_mut()
        .unwrap()
        .reverse();
    assert_eq!(
        crate::generate(&original.to_string(), &crate::Options::default())
            .unwrap()
            .content,
        crate::generate(&reordered.to_string(), &crate::Options::default())
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
    v["resource_changes"][3]["change"]["after"]["vpc_id"] = json!("vpc-other");
    v["configuration"]["root_module"]["resources"][3]["expressions"]["vpc_id"] =
        json!({"references":["aws_vpc.other.id"]});
    assert!(complete(&v, 4, super::SUBNETS));
    assert!(complete(&v, 4, super::GROUPS));
    assert!(parent(&v).is_none());
}
