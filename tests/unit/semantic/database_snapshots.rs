use crate::{plan, semantic};
use serde_json::{Value, json};
const FIXTURE: &str = include_str!("../../fixtures/database-snapshots-plan.json");

#[test]
fn copies_require_cross_region_arn_and_account_proof() {
    for case in ["valid", "identifier", "account", "arn-region", "unknown"] {
        let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
        let provider = json!({"full_name":"registry.terraform.io/hashicorp/aws","expressions":{"region":{"constant_value":"us-east-1"},"assume_role":[{"role_arn":{"constant_value":"arn:aws:iam::123456789012:role/Test"}}]}});
        v["configuration"]["provider_config"]["aws"] = provider.clone();
        v["configuration"]["provider_config"]["aws.west"] = provider;
        v["configuration"]["provider_config"]["aws.west"]["expressions"]["region"]["constant_value"] =
            json!("us-west-2");
        let copy = 8;
        v["configuration"]["root_module"]["resources"][copy]["provider_config_key"] =
            json!("aws.west");
        match case {
            "identifier" => {
                v["resource_changes"][1]["change"]["after"]["id"] = json!("snapshot");
                v["resource_changes"][copy]["change"]["after"]["source_db_snapshot_identifier"] =
                    json!("snapshot");
                v["configuration"]["root_module"]["resources"][copy]["expressions"]["source_db_snapshot_identifier"]
                    ["references"] = json!(["aws_db_snapshot.example.id"]);
            }
            "account" => {
                v["configuration"]["provider_config"]["aws.west"]["expressions"]["assume_role"][0]
                    ["role_arn"]["constant_value"] = json!("arn:aws:iam::999999999999:role/Test")
            }
            "arn-region" => {
                let arn = json!("arn:aws:rds:eu-west-1:123456789012:snapshot:example");
                v["resource_changes"][1]["change"]["after"]["db_snapshot_arn"] = arn.clone();
                v["resource_changes"][copy]["change"]["after"]["source_db_snapshot_identifier"] =
                    arn;
            }
            "unknown" => {
                v["resource_changes"][copy]["change"]["after_unknown"] =
                    json!({"source_db_snapshot_identifier":true})
            }
            _ => {}
        }
        let parsed = crate::plan::evidence::tests::parse_fixture(&v.to_string()).unwrap();
        let graph = semantic::transform(&parsed);
        let node = graph
            .nodes
            .iter()
            .find(|n| n.resource_type == "aws_db_snapshot_copy")
            .unwrap();
        assert_eq!(
            node.references
                .iter()
                .any(|r| r.label == "Source snapshot" && r.complete),
            case == "valid",
            "{case}"
        );
        assert_eq!(
            node.entity
                .scope
                .as_ref()
                .unwrap()
                .region
                .as_ref()
                .unwrap()
                .0,
            "us-west-2"
        );
    }
}

#[test]
fn snapshots_remain_independent_with_exactly_once_coverage() {
    let graph = semantic::transform(&plan::parse(FIXTURE).unwrap());
    assert_eq!(graph.coverage.selected.resources, 14);
    assert!(
        !graph
            .edges
            .iter()
            .any(|e| e.kind == crate::model::EdgeKind::Containment)
    );
    for node in &graph.nodes {
        assert!(node.member_owner.is_none());
        assert_eq!(node.action, crate::model::Action::Create);
    }
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
    }
}
#[test]
fn database_snapshots_require_exact_current_reference_proof() {
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
