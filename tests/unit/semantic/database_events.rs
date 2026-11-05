use crate::{plan, semantic};
use serde_json::{Value, json};
const FIXTURE: &str = include_str!("../../fixtures/database-events-plan.json");

#[test]
fn event_subscriptions_preserve_named_state_identity_without_event_payloads() {
    for format in [crate::OutputFormat::Svg, crate::OutputFormat::Html] {
        let result = crate::generate_named(
            &[
                crate::NamedInput {
                    state: "one".into(),
                    json: FIXTURE.into(),
                },
                crate::NamedInput {
                    state: "two".into(),
                    json: FIXTURE.into(),
                },
            ],
            &crate::Options {
                format,
                ..Default::default()
            },
        )
        .unwrap();
        assert!(result.content.contains("data-state-id=\"one\""));
        assert!(result.content.contains("data-state-id=\"two\""));
        assert!(!result.content.contains("PRIVATE_CATEGORY"));
    }
}

#[test]
fn event_sources_require_complete_nonempty_sets_and_known_service_discriminators() {
    for target in [2, 4, 6] {
        for case in [
            "valid",
            "empty",
            "extra",
            "wrong-type",
            "unknown-type",
            "arn",
            "unknown-member",
        ] {
            let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
            match case {
                "empty" => {
                    v["resource_changes"][target]["change"]["after"]["source_ids"] = json!([])
                }
                "extra" => v["resource_changes"][target]["change"]["after"]["source_ids"]
                    .as_array_mut()
                    .unwrap()
                    .push(json!("unproven")),
                "wrong-type" => {
                    v["resource_changes"][target]["change"]["after"]["source_type"] =
                        json!("db-instance")
                }
                "unknown-type" => {
                    v["resource_changes"][target]["change"]["after_unknown"] =
                        json!({"source_type":true})
                }
                "unknown-member" => {
                    v["resource_changes"][target]["change"]["after_unknown"] =
                        json!({"source_ids":[true]})
                }
                "arn" => {
                    let address = v["resource_changes"][target - 1]["address"]
                        .as_str()
                        .unwrap()
                        .to_owned();
                    v["resource_changes"][target - 1]["change"]["after"]["arn"] =
                        json!("arn:aws:rds:us-east-1:123456789012:cluster:example");
                    v["resource_changes"][target]["change"]["after"]["source_ids"] =
                        json!(["arn:aws:rds:us-east-1:123456789012:cluster:example"]);
                    v["configuration"]["root_module"]["resources"][target]["expressions"]["source_ids"] =
                        json!({"references":[format!("{address}.arn")]});
                }
                _ => {}
            }
            let graph = semantic::transform(&plan::parse(&v.to_string()).unwrap());
            let address = v["resource_changes"][target]["address"].as_str().unwrap();
            let node = graph.nodes.iter().find(|n| n.address == address).unwrap();
            assert_eq!(
                node.references
                    .iter()
                    .any(|r| r.label == "Event sources" && r.complete),
                case == "valid",
                "{target} {case}"
            );
            assert!(
                node.references
                    .iter()
                    .any(|r| r.label == "Event topic" && r.complete)
            );
            assert!(node.member_owner.is_none());
            assert_eq!(graph.coverage.selected.resources, 7);
        }
    }
}

#[test]
fn event_source_endpoint_contracts_are_service_specific() {
    for (service, discriminator, kind, field) in [
        ("db", "db-instance", "aws_db_instance", "identifier"),
        (
            "db",
            "db-instance",
            "aws_rds_cluster_instance",
            "identifier",
        ),
        ("db", "db-parameter-group", "aws_db_parameter_group", "name"),
        (
            "db",
            "db-snapshot",
            "aws_db_snapshot",
            "db_snapshot_identifier",
        ),
        (
            "db",
            "db-snapshot",
            "aws_db_snapshot_copy",
            "target_db_snapshot_identifier",
        ),
        (
            "db",
            "db-cluster-snapshot",
            "aws_db_cluster_snapshot",
            "db_cluster_snapshot_identifier",
        ),
        (
            "db",
            "db-cluster-snapshot",
            "aws_rds_cluster_snapshot_copy",
            "target_db_cluster_snapshot_identifier",
        ),
        (
            "docdb",
            "db-instance",
            "aws_docdb_cluster_instance",
            "identifier",
        ),
        (
            "docdb",
            "db-parameter-group",
            "aws_docdb_cluster_parameter_group",
            "name",
        ),
        (
            "docdb",
            "db-cluster-snapshot",
            "aws_docdb_cluster_snapshot",
            "db_cluster_snapshot_identifier",
        ),
        (
            "neptune",
            "db-instance",
            "aws_neptune_cluster_instance",
            "identifier",
        ),
        (
            "neptune",
            "db-parameter-group",
            "aws_neptune_parameter_group",
            "name",
        ),
        (
            "neptune",
            "db-cluster-snapshot",
            "aws_neptune_cluster_snapshot",
            "db_cluster_snapshot_identifier",
        ),
    ] {
        for valid in [true, false] {
            let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
            let address = format!("{kind}.endpoint");
            v["resource_changes"][1]["address"] = json!(address);
            v["resource_changes"][1]["type"] = json!(kind);
            v["resource_changes"][1]["change"]["after"] = json!({field:"source"});
            v["configuration"]["root_module"]["resources"][1]["address"] = json!(address);
            v["configuration"]["root_module"]["resources"][1]["type"] = json!(kind);
            let subscription = format!(
                "aws_{}_event_subscription",
                if valid { service } else { "wrong" }
            );
            v["resource_changes"][2]["type"] = json!(subscription);
            v["configuration"]["root_module"]["resources"][2]["type"] = json!(subscription);
            v["resource_changes"][2]["change"]["after"]["source_type"] = json!(discriminator);
            v["resource_changes"][2]["change"]["after"]["source_ids"] = json!(["source"]);
            v["configuration"]["root_module"]["resources"][2]["expressions"]["source_ids"] =
                json!({"references":[format!("{address}.{field}")]});
            let graph = semantic::transform(&plan::parse(&v.to_string()).unwrap());
            let node = graph
                .nodes
                .iter()
                .find(|n| n.address == "aws_db_event_subscription.example")
                .unwrap();
            assert_eq!(
                node.references
                    .iter()
                    .any(|r| r.label == "Event sources" && r.complete),
                valid,
                "{service} {kind} {valid}"
            );
        }
    }
}
#[test]
fn database_events_require_exact_current_reference_proof() {
    let original: Value = serde_json::from_str(FIXTURE).unwrap();
    for target in 0..original["resource_changes"].as_array().unwrap().len() {
        for (field, expression) in
            original["configuration"]["root_module"]["resources"][target]["expressions"]
                .as_object()
                .unwrap()
        {
            let reference = expression["references"][0].as_str().unwrap();
            let address = reference.rsplit_once('.').unwrap().0;
            let source = original["resource_changes"]
                .as_array()
                .unwrap()
                .iter()
                .position(|r| r["address"] == address)
                .unwrap();
            for case in [
                "valid",
                "unknown",
                "wrong-output",
                "value",
                "deleted",
                "deposed",
                "data",
                "type",
                "region",
                "filtered",
            ] {
                let mut v = original.clone();
                match case {
                    "unknown" => {
                        v["resource_changes"][target]["change"]["after_unknown"] =
                            json!({field:true})
                    }
                    "wrong-output" => {
                        v["configuration"]["root_module"]["resources"][target]["expressions"]
                            [field] = json!({"references":[format!("{address}.unrelated")]})
                    }
                    "value" => {
                        v["resource_changes"][target]["change"]["after"][field] = json!("wrong")
                    }
                    "deleted" => {
                        v["resource_changes"][source]["change"]["actions"] = json!(["delete"])
                    }
                    "deposed" => v["resource_changes"][source]["deposed"] = json!("old"),
                    "data" => v["resource_changes"][source]["mode"] = json!("data"),
                    "type" => v["resource_changes"][source]["type"] = json!("aws_unrelated"),
                    "region" => {
                        v["configuration"]["provider_config"]["other"] = json!({"full_name":"registry.terraform.io/hashicorp/aws","expressions":{"region":{"constant_value":"us-west-2"}}});
                        v["configuration"]["root_module"]["resources"][source]["provider_config_key"] =
                            json!("other");
                    }
                    "filtered" => {
                        v["resource_changes"].as_array_mut().unwrap().remove(source);
                    }
                    _ => {}
                }
                let g = semantic::transform(&plan::parse(&v.to_string()).unwrap());
                let node = g
                    .nodes
                    .iter()
                    .find(|n| {
                        n.address
                            == original["resource_changes"][target]["address"]
                                .as_str()
                                .unwrap()
                    })
                    .unwrap();
                let label = super::label(&node.resource_type, field).unwrap();
                assert_eq!(
                    node.references
                        .iter()
                        .any(|r| r.attribute == *field && r.label == label && r.complete),
                    case == "valid",
                    "{target} {field} {case}"
                );
            }
        }
    }
}
