use crate::{
    model::{Placement, ResourceRole},
    plan, semantic,
};
use serde_json::{Value, json};

const FIXTURE: &str = include_str!("../../fixtures/glue-plan.json");

#[test]
fn glue_role_proofs_preserve_fallback_and_lifecycle() {
    for consumer in [4, 5] {
        let attribute = if consumer == 4 { "role" } else { "role_arn" };
        for case in 0..8 {
            let mut value: Value = serde_json::from_str(FIXTURE).unwrap();
            match case {
                1 => {
                    value["resource_changes"][2]["provider_name"] = json!("example.com/foreign/aws")
                }
                2 => value["resource_changes"][2]["type"] = json!("aws_iam_user"),
                3 => value["resource_changes"][consumer]["change"]["actions"] = json!(["delete"]),
                4 => value["resource_changes"][2]["deposed"] = json!("old"),
                5 => value["resource_changes"][consumer]["change"]["after_unknown"] = json!(true),
                6 => {
                    value["configuration"]["root_module"]["resources"][consumer]["expressions"]
                        [attribute] = json!({"references":["aws_iam_role.etl.arn", "var.selector"]})
                }
                7 => value["resource_changes"][2]["mode"] = json!("data"),
                _ => {}
            }
            let graph = semantic::transform(&plan::parse(&value.to_string()).unwrap());
            let address = value["resource_changes"][consumer]["address"]
                .as_str()
                .unwrap();
            let node = graph.nodes.iter().find(|n| n.address == address).unwrap();
            assert_eq!(
                node.references
                    .iter()
                    .any(|r| r.label == "Execution role reference" && r.complete),
                case == 0
            );
            assert_eq!(graph.coverage.selected.resources, 7);
        }
    }
    let mut value: Value = serde_json::from_str(FIXTURE).unwrap();
    value["resource_changes"][1]["previous_address"] = json!("aws_glue_catalog_table.old");
    value["resource_changes"][1]["change"]["importing"] = json!({"id":"PRIVATE_IMPORT"});
    let mut old = value["resource_changes"][1].clone();
    old["deposed"] = json!("old");
    old["change"]["before"] = old["change"]["after"].take();
    old["change"]["actions"] = json!(["delete"]);
    value["resource_changes"].as_array_mut().unwrap().push(old);
    let graph = semantic::transform(&plan::parse(&value.to_string()).unwrap());
    assert_eq!(graph.coverage.selected.resources, 8);
    let current = graph
        .nodes
        .iter()
        .find(|n| n.address == "aws_glue_catalog_table.events" && n.deposed_key.is_none())
        .unwrap();
    assert_eq!(
        current.previous_address.as_deref(),
        Some("aws_glue_catalog_table.old")
    );
    assert!(current.metadata.import.is_some());
    assert!(
        graph
            .nodes
            .iter()
            .filter(|n| n.deposed_key.is_some())
            .all(|n| n.references.iter().all(|r| !r.complete))
    );
    let mut modular: Value = serde_json::from_str(FIXTURE).unwrap();
    let resources = modular["configuration"]["root_module"]["resources"].take();
    modular["configuration"]["root_module"]["module_calls"] =
        json!({"lake":{"module":{"resources":resources}}});
    for node in modular["resource_changes"].as_array_mut().unwrap() {
        node["address"] = json!(format!("module.lake.{}", node["address"].as_str().unwrap()));
        node["module_address"] = json!("module.lake");
    }
    let output = crate::generate(
        &modular.to_string(),
        &crate::Options {
            modules: vec!["module.lake".into()],
            ..Default::default()
        },
    )
    .unwrap()
    .content;
    assert!(output.contains("7 resources"));
    assert!(output.contains("Catalog database"));
}

#[test]
fn glue_catalog_requires_explicit_current_typed_and_matching_catalog_region() {
    super::super::service_references::tests::check_service_fixture(
        FIXTURE,
        "Catalog database",
        "database_name",
    );
    for case in 0..6 {
        let mut value: Value = serde_json::from_str(FIXTURE).unwrap();
        match case {
            0 => {
                value["resource_changes"][1]["change"]["after"]["catalog_id"] =
                    json!("999999999999")
            }
            1 => {
                value["resource_changes"][0]["change"]["after_unknown"] = json!({"catalog_id":true})
            }
            2 => {
                value["configuration"]["provider_config"]["aws"]["expressions"]["region"] =
                    json!({"references":["var.region"]})
            }
            3 => {
                let mut provider = value["configuration"]["provider_config"]["aws"].clone();
                provider["expressions"]["region"]["constant_value"] = json!("eu-west-1");
                value["configuration"]["provider_config"]["aws.other"] = provider;
                value["configuration"]["root_module"]["resources"][1]["provider_config_key"] =
                    json!("aws.other");
            }
            4 => value["resource_changes"][1]["deposed"] = json!("old"),
            _ => value["resource_changes"][1]["provider_name"] = json!("example.com/foreign/aws"),
        }
        let parsed = plan::parse(&value.to_string()).unwrap();
        let graph = semantic::transform(&parsed);
        assert!(
            !graph
                .nodes
                .iter()
                .flat_map(|n| &n.references)
                .any(|r| r.label == "Catalog database" && r.complete),
            "case {case}"
        );
        assert_eq!(graph.coverage.selected.resources, 7);
    }
}

#[test]
fn glue_keeps_safe_references_identity_and_omits_payloads() {
    let parsed = plan::parse(FIXTURE).unwrap();
    assert!(!format!("{parsed:?}").contains("PRIVATE_"));
    let graph = semantic::transform(&parsed);
    for node in graph
        .nodes
        .iter()
        .filter(|n| n.resource_type.starts_with("aws_glue_"))
    {
        assert_eq!(node.role, ResourceRole::Node);
        assert_eq!(node.placement, Placement::Logical);
        assert!(
            node.entity
                .scope
                .as_ref()
                .is_some_and(|s| !s.global && s.region.is_some())
        );
    }
    assert_eq!(
        graph
            .nodes
            .iter()
            .flat_map(|n| &n.references)
            .filter(|r| r.label == "Execution role reference" && r.complete)
            .count(),
        2
    );
    let literal = graph
        .nodes
        .iter()
        .find(|n| n.address == "aws_glue_job.literal")
        .unwrap();
    assert!(literal.references.is_empty());
    for format in [crate::OutputFormat::Svg, crate::OutputFormat::Html] {
        let output = crate::generate(
            FIXTURE,
            &crate::Options {
                format,
                ..Default::default()
            },
        )
        .unwrap()
        .content;
        assert!(!output.contains("PRIVATE_"));
        assert!(!output.contains("arn:aws:"));
        assert!(output.contains("planorama-icon-aws-glue"));
        assert!(output.contains("Catalog database"));
    }
}
