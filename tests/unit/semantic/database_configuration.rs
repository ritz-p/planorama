use crate::{plan, semantic};
use serde_json::{Value, json};

const FIXTURE: &str = include_str!("../../fixtures/database-configuration-plan.json");

#[test]
fn database_iam_roles_are_global_but_require_matching_accounts() {
    for region in ["us-east-1", "us-west-2"] {
        for account in ["123456789012", "999999999999"] {
            let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
            v["configuration"]["provider_config"]["aws"]["expressions"]["assume_role"] =
                json!([{"role_arn":{"constant_value":"arn:aws:iam::123456789012:role/Test"}}]);
            v["configuration"]["provider_config"]["aws.other"] = json!({"full_name":"registry.terraform.io/hashicorp/aws","alias":"other","expressions":{"region":{"constant_value":region},"assume_role":[{"role_arn":{"constant_value":format!("arn:aws:iam::{account}:role/Test")}}]}});
            v["configuration"]["root_module"]["resources"][0]["provider_config_key"] =
                json!("aws.other");
            v["configuration"]["root_module"]["resources"][6]["provider_config_key"] =
                json!("aws.other");
            let parsed = crate::plan::evidence::tests::parse_fixture(&v.to_string()).unwrap();
            let g = semantic::transform(&parsed);
            let db = g
                .nodes
                .iter()
                .find(|n| n.address == "aws_db_instance.main")
                .unwrap();
            assert_eq!(
                db.references
                    .iter()
                    .any(|r| r.label == "Monitoring role" && r.complete),
                account == "123456789012"
            );
            assert_eq!(
                db.references
                    .iter()
                    .any(|r| r.label == "Parameter group" && r.complete),
                account == "123456789012" && region == "us-east-1"
            );
        }
    }
}

#[test]
fn database_role_collection_requires_complete_set_and_named_states_stay_separate() {
    let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
    v["resource_changes"][8]["change"]["after"]["iam_roles"] =
        json!(["arn:aws:iam::123456789012:role/monitor", "unproven"]);
    let g = semantic::transform(&plan::parse(&v.to_string()).unwrap());
    let cluster = g
        .nodes
        .iter()
        .find(|n| n.address == "aws_rds_cluster.main")
        .unwrap();
    assert!(
        !cluster
            .references
            .iter()
            .any(|r| r.label == "Associated IAM roles" && r.complete)
    );
    for format in [crate::OutputFormat::Svg, crate::OutputFormat::Html] {
        let output = crate::generate_named(
            &[
                crate::NamedInput {
                    state: "one".into(),
                    json: FIXTURE.into(),
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
        .unwrap();
        assert!(output.content.contains("data-state-id=\"one\""));
        assert!(output.content.contains("data-state-id=\"two\""));
        assert!(!output.content.contains("PRIVATE_"));
    }
}

#[test]
fn database_configuration_requires_exact_current_reference_proof() {
    let original: Value = serde_json::from_str(FIXTURE).unwrap();
    for target in 7..15 {
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
                            [field] = json!({"references":[format!("{address}.id")]})
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

#[test]
fn database_configuration_is_shared_and_does_not_create_network_placement() {
    let g = semantic::transform(&plan::parse(FIXTURE).unwrap());
    assert_eq!(g.coverage.selected.resources, 15);
    assert!(
        !g.edges
            .iter()
            .any(|e| e.kind == crate::model::EdgeKind::Containment)
    );
    for kind in [
        "aws_db_parameter_group",
        "aws_db_option_group",
        "aws_rds_cluster_parameter_group",
        "aws_docdb_cluster_parameter_group",
        "aws_neptune_parameter_group",
        "aws_neptune_cluster_parameter_group",
    ] {
        let nodes: Vec<_> = g.nodes.iter().filter(|n| n.resource_type == kind).collect();
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].placement, crate::model::Placement::Logical);
        assert!(nodes[0].member_owner.is_none());
    }
    for format in [crate::OutputFormat::Svg, crate::OutputFormat::Html] {
        let result = crate::generate(
            FIXTURE,
            &crate::Options {
                format,
                ..Default::default()
            },
        )
        .unwrap();
        assert!(!result.content.contains("PRIVATE_"));
    }
}
