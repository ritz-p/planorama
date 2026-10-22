use crate::{
    model::{EdgeKind, Placement, ResourceRole},
    plan, semantic,
};
use serde_json::{Value, json};

const FIXTURE: &str = include_str!("../../fixtures/athena-plan.json");

#[test]
fn athena_is_regional_spatial_and_only_preserves_explicit_configuration_references() {
    let parsed = plan::parse(FIXTURE).unwrap();
    let graph = semantic::transform(&parsed);
    assert_eq!(graph.coverage.selected.resources, 9);
    assert_eq!(graph.edges.len(), 2);
    assert!(graph.edges.iter().all(|e| e.kind == EdgeKind::Dependency));
    assert!(
        parsed
            .attributes
            .iter()
            .all(|r| !r.service_reference_verified)
    );
    for node in graph
        .nodes
        .iter()
        .filter(|n| n.resource_type.starts_with("aws_athena_"))
    {
        assert_eq!(node.role, ResourceRole::Node);
        assert_eq!(node.placement, Placement::Spatial);
        assert!(node.member_owner.is_none());
        assert!(node.entity.scope.as_ref().is_some_and(|s| !s.global && s.region.as_ref().is_some_and(|r| r.0 == "us-east-1")));
        if node.address.ends_with(".analytics") {
            assert!(
                node.references
                    .iter()
                    .all(|r| r.label == "References" && r.complete)
            );
            for attribute in [
                "configuration.result_configuration.output_location",
                "configuration.result_configuration.encryption_configuration.kms_key_arn",
            ] {
                assert!(
                    node.references
                        .iter()
                        .any(|r| r.attribute == attribute && r.targets.len() == 1)
                );
            }
        } else {
            assert!(node.references.is_empty());
        }
    }
    let catalog = graph
        .nodes
        .iter()
        .position(|n| n.address == "aws_athena_data_catalog.lake")
        .unwrap();
    assert!(
        graph
            .edges
            .iter()
            .all(|e| e.from != catalog && e.to != catalog)
    );
    assert!(!format!("{parsed:?}").contains("PRIVATE_"));
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
        assert!(output.contains("planorama-icon-aws-athena"));
        assert!(!output.contains("PRIVATE_"));
        assert!(!output.contains("arn:aws:"));
        assert!(!output.contains("s3://"));
        let svg = &output[output.find("<svg ").unwrap()..output.rfind("</svg>").unwrap() + 6];
        let doc = roxmltree::Document::parse(svg).unwrap();
        assert_eq!(
            doc.descendants()
                .filter(|n| n.attribute("data-terraform-address").is_some())
                .count(),
            9
        );
    }
    let layout = crate::layout::Layout::new(&graph);
    let cards: Vec<_> = graph
        .nodes
        .iter()
        .enumerate()
        .filter(|(_, n)| n.resource_type.starts_with("aws_athena_"))
        .map(|(index, _)| layout.bounds[index])
        .collect();
    assert_eq!(cards.len(), 6);
    for (index, card) in cards.iter().enumerate() {
        assert!(card.width > 0 && card.height > 0);
        for other in &cards[index + 1..] {
            assert!(
                card.right() <= other.origin.x
                    || other.right() <= card.origin.x
                    || card.origin.y + card.height <= other.origin.y
                    || other.origin.y + other.height <= card.origin.y
            );
        }
    }
}

#[test]
fn athena_failures_and_unknown_values_keep_only_terraform_provenance() {
    for case in 0..9 {
        let mut value: Value = serde_json::from_str(FIXTURE).unwrap();
        let reference = &mut value["configuration"]["root_module"]["resources"][2]["expressions"]["configuration"]
            [0]["result_configuration"][0]["output_location"];
        match case {
            0 => *reference = json!({"references":["var.destination"]}),
            1 => *reference = json!({"references":["aws_s3_bucket.missing.bucket"]}),
            2 => {
                let mut second = value["resource_changes"][0].clone();
                value["resource_changes"][0]["address"] = json!("aws_s3_bucket.results[0]");
                second["address"] = json!("aws_s3_bucket.results[1]");
                value["resource_changes"]
                    .as_array_mut()
                    .unwrap()
                    .push(second);
            }
            3 => value["resource_changes"][0]["provider_name"] = json!("example.com/foreign/aws"),
            4 => value["resource_changes"][0]["type"] = json!("aws_instance"),
            5 => value["resource_changes"][2]["change"]["after_unknown"] = json!(true),
            6 => value["resource_changes"][0]["deposed"] = json!("old"),
            7 => value["resource_changes"][2]["change"]["actions"] = json!(["delete"]),
            _ => value["resource_changes"][0]["change"]["actions"] = json!(["delete"]),
        }
        let parsed = plan::parse(&value.to_string()).unwrap();
        let graph = semantic::transform(&parsed);
        assert!(graph.edges.iter().all(|e| e.kind == EdgeKind::Dependency));
        assert!(
            parsed
                .attributes
                .iter()
                .all(|r| !r.service_reference_verified)
        );
        assert_eq!(
            graph.coverage.selected.resources,
            value["resource_changes"].as_array().unwrap().len()
        );
        if case <= 2 {
            let node = graph
                .nodes
                .iter()
                .find(|n| n.address == "aws_athena_workgroup.analytics")
                .unwrap();
            assert!(
                node.references
                    .iter()
                    .filter(|r| r.attribute.ends_with("output_location"))
                    .all(|r| !r.complete || !r.issues.is_empty())
            );
            assert!(!semantic::diagnostics::collect(&parsed).is_empty());
        }
    }
    for index in [2, 3] {
        for case in 0..3 {
            let mut value: Value = serde_json::from_str(FIXTURE).unwrap();
            match case {
                0 => {
                    value["resource_changes"][index]["provider_name"] =
                        json!("example.com/foreign/aws")
                }
                1 => value["resource_changes"][index]["type"] = json!("aws_athena_named_query"),
                _ => {
                    value["configuration"]["root_module"]["resources"][index]["expressions"]["region"] =
                        json!({"references":["var.region"]})
                }
            }
            let graph = semantic::transform(&plan::parse(&value.to_string()).unwrap());
            let address = value["resource_changes"][index]["address"]
                .as_str()
                .unwrap();
            let node = graph.nodes.iter().find(|n| n.address == address).unwrap();
            if case == 0 {
                assert_eq!(node.role, ResourceRole::Unknown);
            } else if case == 1 {
                assert_eq!(node.role, ResourceRole::Controller);
                assert!(
                    node.references
                        .iter()
                        .all(|reference| reference.label == "References")
                );
            } else {
                assert!(
                    node.entity
                        .scope
                        .as_ref()
                        .is_none_or(|s| s.region.is_none())
                );
            }
        }
    }
}

#[test]
fn athena_preserves_indexed_data_lifecycle_module_filter_and_state_identity() {
    let indexed = FIXTURE.replace("aws_s3_bucket.results", "aws_s3_bucket.results[0]");
    let graph = semantic::transform(&plan::parse(&indexed).unwrap());
    assert_eq!(graph.edges.len(), 2);
    let filtered = crate::view::single(
        &graph,
        &crate::view::Options {
            focus: Some("aws_athena_workgroup.analytics".into()),
            depth: Some(0),
            ..Default::default()
        },
    )
    .unwrap();
    assert!(
        crate::svg::render(&filtered, &crate::layout::Layout::new(&filtered))
            .contains("Unavailable")
    );
    let mut value: Value = serde_json::from_str(FIXTURE).unwrap();
    value["resource_changes"][2]["previous_address"] = json!("aws_athena_workgroup.old");
    value["resource_changes"][2]["change"]["importing"] = json!({"id":"PRIVATE_IMPORT"});
    let mut old = value["resource_changes"][2].clone();
    old["deposed"] = json!("old");
    old["change"]["before"] = old["change"]["after"].take();
    old["change"]["actions"] = json!(["delete"]);
    value["resource_changes"].as_array_mut().unwrap().push(old);
    let mut data = value["resource_changes"][2].clone();
    data["address"] = json!("data.aws_athena_workgroup.lookup");
    data["mode"] = json!("data");
    data["change"]["actions"] = json!(["read"]);
    value["resource_changes"].as_array_mut().unwrap().push(data);
    let graph = semantic::transform(&plan::parse(&value.to_string()).unwrap());
    assert_eq!(graph.coverage.selected.resources, 11);
    assert!(
        graph
            .nodes
            .iter()
            .filter(|n| n.deposed_key.is_some())
            .all(|n| n.references.is_empty())
    );
    let current = graph
        .nodes
        .iter()
        .find(|n| n.address == "aws_athena_workgroup.analytics" && n.deposed_key.is_none())
        .unwrap();
    assert_eq!(
        current.previous_address.as_deref(),
        Some("aws_athena_workgroup.old")
    );
    assert!(current.metadata.import.is_some());
    assert!(
        graph
            .nodes
            .iter()
            .any(|n| n.mode == crate::model::EntityMode::Data && n.placement == Placement::Spatial)
    );
    let mut modular: Value = serde_json::from_str(FIXTURE).unwrap();
    let resources = modular["configuration"]["root_module"]["resources"].take();
    modular["configuration"]["root_module"]["module_calls"] =
        json!({"analytics":{"module":{"resources":resources}}});
    for node in modular["resource_changes"].as_array_mut().unwrap() {
        node["address"] = json!(format!(
            "module.analytics.{}",
            node["address"].as_str().unwrap()
        ));
        node["module_address"] = json!("module.analytics");
    }
    let output = crate::generate(
        &modular.to_string(),
        &crate::Options {
            modules: vec!["module.analytics".into()],
            ..Default::default()
        },
    )
    .unwrap()
    .content;
    assert!(output.contains("9 resources"));
    let plans = plan::parse_inputs(
        ["one", "two"]
            .into_iter()
            .map(|state| crate::model::PlanInput {
                state_id: crate::model::StateId::new(state).unwrap(),
                plan_json: FIXTURE.into(),
            })
            .collect(),
    )
    .unwrap();
    assert!(semantic::cross_state::resolve(&plans, &[]).edges.is_empty());
    for parsed in plans.states.values() {
        let graph = semantic::transform(parsed);
        assert_eq!(graph.coverage.selected.resources, 9);
        assert!(
            graph
                .nodes
                .iter()
                .flat_map(|n| &n.references)
                .flat_map(|r| &r.targets)
                .all(|t| t.state.is_none())
        );
    }
}
