use crate::{model::EdgeKind, plan, semantic, view};
use serde_json::{Value, json};

const FIXTURE: &str = include_str!("../../fixtures/mq-plan.json");

fn complete(value: &Value, field: &str) -> bool {
    let graph = semantic::transform(&plan::parse(&value.to_string()).unwrap());
    graph
        .nodes
        .iter()
        .find(|n| n.address == "aws_mq_broker.primary")
        .unwrap()
        .references
        .iter()
        .any(|r| {
            r.attribute == field
                && r.complete
                && super::label("aws_mq_broker", field) == Some(r.label)
        })
}

fn parent(value: &Value) -> Option<String> {
    let graph = semantic::transform(&plan::parse(&value.to_string()).unwrap());
    graph
        .edges
        .iter()
        .find(|e| {
            e.kind == EdgeKind::Containment && graph.nodes[e.to].address == "aws_mq_broker.primary"
        })
        .map(|e| graph.nodes[e.from].address.clone())
}

#[test]
fn mq_shared_configuration_and_multi_subnet_vpc_preserve_identity_and_safe_output() {
    let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
    for engine in ["ActiveMQ", "RabbitMQ"] {
        for i in [5, 6, 7] {
            v["resource_changes"][i]["change"]["after"]["engine_type"] = json!(engine);
        }
        assert_eq!(parent(&v).as_deref(), Some("aws_vpc.main"));
        for field in ["subnet_ids", "security_groups", "configuration.id"] {
            assert!(complete(&v, field), "{engine} {field}");
        }
        let graph = semantic::transform(&plan::parse(&v.to_string()).unwrap());
        assert_eq!(graph.coverage.selected.resources, 8);
        assert_eq!(
            graph
                .nodes
                .iter()
                .filter(|n| n.resource_type == "aws_mq_configuration")
                .count(),
            1
        );
        assert!(graph.nodes.iter().all(|n| n.member_owner.is_none()));
        for format in [crate::OutputFormat::Svg, crate::OutputFormat::Html] {
            let out = crate::generate(
                &v.to_string(),
                &crate::Options {
                    format,
                    ..Default::default()
                },
            )
            .unwrap()
            .content;
            assert!(!out.contains("PRIVATE_"));
            assert!(out.contains("MQ configuration"));
        }
    }
}

#[test]
fn mq_configuration_requires_explicit_id_revision_and_compatible_engine() {
    for case in [
        "literal",
        "revision-literal",
        "revision-mismatch",
        "revision-unknown",
        "source-unknown",
        "id-mismatch",
        "wrong-field",
        "ambiguous",
        "wrong-type",
        "wrong-provider",
        "scope",
        "deleted",
        "deposed",
        "data",
        "engine",
        "engine-unknown",
        "block-unknown",
        "block-multiple",
    ] {
        let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
        match case {
            "literal" => {
                v["configuration"]["root_module"]["resources"][6]["expressions"]["configuration"]
                    [0]["id"] = json!({"constant_value":"c-shared"})
            }
            "revision-literal" => {
                v["configuration"]["root_module"]["resources"][6]["expressions"]["configuration"]
                    [0]["revision"] = json!({"constant_value":3})
            }
            "revision-mismatch" => {
                v["resource_changes"][6]["change"]["after"]["configuration"][0]["revision"] =
                    json!(2)
            }
            "revision-unknown" => {
                v["resource_changes"][6]["change"]["after_unknown"] =
                    json!({"configuration":[{"revision":true}]})
            }
            "source-unknown" => {
                v["resource_changes"][5]["change"]["after_unknown"] =
                    json!({"latest_revision":true})
            }
            "id-mismatch" => {
                v["resource_changes"][6]["change"]["after"]["configuration"][0]["id"] =
                    json!("c-other")
            }
            "wrong-field" => {
                v["configuration"]["root_module"]["resources"][6]["expressions"]["configuration"]
                    [0]["id"] = json!({"references":["aws_mq_configuration.shared.name"]})
            }
            "ambiguous" => {
                v["configuration"]["root_module"]["resources"][6]["expressions"]["configuration"]
                    [0]["id"] = json!({"references":["var.dynamic"]})
            }
            "wrong-type" => v["resource_changes"][5]["type"] = json!("aws_msk_configuration"),
            "wrong-provider" => {
                v["resource_changes"][5]["provider_name"] = json!("example.org/foreign/aws")
            }
            "scope" => {
                let mut config = v["configuration"]["provider_config"]["aws"].clone();
                config["expressions"]["region"] = json!({"constant_value":"us-west-2"});
                v["configuration"]["provider_config"]["aws.other"] = config;
                v["configuration"]["root_module"]["resources"][5]["provider_config_key"] =
                    json!("aws.other");
            }
            "deleted" => v["resource_changes"][5]["change"]["actions"] = json!(["delete"]),
            "deposed" => v["resource_changes"][5]["deposed"] = json!("old"),
            "data" => v["resource_changes"][5]["mode"] = json!("data"),
            "engine" => {
                v["resource_changes"][5]["change"]["after"]["engine_type"] = json!("RabbitMQ")
            }
            "engine-unknown" => {
                v["resource_changes"][5]["change"]["after_unknown"] = json!({"engine_type":true})
            }
            "block-unknown" => {
                v["resource_changes"][6]["change"]["after_unknown"] = json!({"configuration":true})
            }
            "block-multiple" => {
                v["resource_changes"][6]["change"]["after"]["configuration"] =
                    json!([{"id":"c-shared","revision":3},{"id":"c-shared","revision":3}])
            }
            _ => unreachable!(),
        }
        assert!(!complete(&v, "configuration.id"), "{case}");
    }
}

#[test]
fn mq_placement_requires_all_subnets_and_groups_to_prove_the_same_vpc() {
    for case in [
        "literal",
        "missing",
        "unknown",
        "partial",
        "duplicate",
        "subnet-vpc",
        "group-vpc",
        "wrong-provider",
        "deleted",
    ] {
        let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
        match case {
            "literal" => {
                v["configuration"]["root_module"]["resources"][6]["expressions"]["subnet_ids"] =
                    json!({"constant_value":["subnet-a","subnet-b"]})
            }
            "missing" => {
                v["configuration"]["root_module"]["resources"][6]["expressions"]
                    .as_object_mut()
                    .unwrap()
                    .remove("security_groups");
            }
            "unknown" => {
                v["resource_changes"][6]["change"]["after_unknown"] =
                    json!({"subnet_ids":[false,true]})
            }
            "partial" => {
                v["resource_changes"][6]["change"]["after"]["subnet_ids"] = json!(["subnet-a"])
            }
            "duplicate" => {
                v["resource_changes"][6]["change"]["after"]["subnet_ids"] =
                    json!(["subnet-a", "subnet-a"])
            }
            "subnet-vpc" | "group-vpc" => {
                let i = if case == "subnet-vpc" { 3 } else { 4 };
                v["resource_changes"][i]["change"]["after"]["vpc_id"] = json!("vpc-other");
                v["configuration"]["root_module"]["resources"][i]["expressions"]["vpc_id"] =
                    json!({"references":["aws_vpc.other.id"]});
            }
            "wrong-provider" => {
                v["resource_changes"][4]["provider_name"] = json!("example.org/foreign/aws")
            }
            "deleted" => v["resource_changes"][2]["change"]["actions"] = json!(["delete"]),
            _ => unreachable!(),
        }
        assert!(parent(&v).is_none(), "{case}");
        assert!(
            crate::provider::diagnostics(&plan::parse(&v.to_string()).unwrap())
                .iter()
                .any(|d| d.address == "aws_mq_broker.primary"),
            "{case}"
        );
    }
}

#[test]
fn mq_filtering_lifecycle_and_named_states_do_not_borrow_proofs() {
    let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
    v["resource_changes"][6]["previous_address"] = json!("aws_mq_broker.old");
    v["resource_changes"][6]["change"]["actions"] = json!(["update"]);
    v["resource_changes"][6]["change"]["importing"] = json!({"id":"PRIVATE_IMPORT"});
    let mut old = v["resource_changes"][6].clone();
    old["deposed"] = json!("old");
    old["change"]["actions"] = json!(["delete"]);
    v["resource_changes"].as_array_mut().unwrap().push(old);
    let parsed = plan::parse(&v.to_string()).unwrap();
    let graph = semantic::transform(&parsed);
    assert_eq!(graph.coverage.selected.resources, 9);
    for source in &parsed.nodes {
        let nodes = graph
            .nodes
            .iter()
            .filter(|n| n.address == source.address && n.deposed_key == source.deposed_key)
            .collect::<Vec<_>>();
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].metadata, source.metadata);
        assert_eq!(nodes[0].action, source.action);
        assert_eq!(nodes[0].previous_address, source.previous_address);
    }
    let selected = view::single(
        &semantic::transform(&plan::parse(FIXTURE).unwrap()),
        &view::Options {
            focus: Some("aws_mq_broker.primary".into()),
            depth: Some(0),
            ..Default::default()
        },
    )
    .unwrap();
    assert!(
        !selected
            .edges
            .iter()
            .any(|e| e.kind == EdgeKind::Containment)
    );
    assert_eq!(selected.coverage.selected.resources, 1);
    assert!(
        crate::svg::render(&selected, &crate::layout::Layout::new(&selected))
            .contains("Unavailable")
    );
    let mut missing = v.clone();
    missing["resource_changes"]
        .as_array_mut()
        .unwrap()
        .remove(5);
    assert!(!complete(&missing, "configuration.id"));
    for format in [crate::OutputFormat::Svg, crate::OutputFormat::Html] {
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
            &crate::Options {
                format,
                ..Default::default()
            },
        )
        .unwrap()
        .content;
        assert!(!out.contains("PRIVATE_"));
        assert!(!out.contains("<metadata id=\"cross-state-relationships\">"));
        assert!(out.contains("data-state-id=\"one\""));
        assert!(out.contains("data-state-id=\"two\""));
    }
}
