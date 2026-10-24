use crate::{layout::Layout, model::Placement, plan, semantic, svg, view};
use serde_json::{Value, json};

const FIXTURE: &str = include_str!("../../fixtures/log-groups-plan.json");

#[test]
fn mixed_references_expose_only_log_group_targets_without_losing_provenance() {
    let mut value: Value = serde_json::from_str(FIXTURE).unwrap();
    value["configuration"]["root_module"]["resources"][1]["expressions"] = json!({"tags":{"references":["aws_cloudwatch_log_group.app.name", "aws_s3_bucket.tagged.id"]}});
    value["configuration"]["root_module"]["resources"][1]["depends_on"] =
        json!(["aws_cloudwatch_log_group.app", "aws_s3_bucket.tagged"]);
    let graph = semantic::transform(&plan::parse(&value.to_string()).unwrap());
    let consumer = graph
        .nodes
        .iter()
        .find(|n| n.address == "aws_lambda_function.app")
        .unwrap();
    let rows = view::references::rows(consumer);
    assert_eq!(rows.len(), 2);
    assert!(
        rows.iter()
            .all(|r| r.target.unwrap().id.address == "aws_cloudwatch_log_group.app")
    );
    for dependency in [false, true] {
        let targets: Vec<_> = consumer
            .references
            .iter()
            .filter(|r| r.dependency == dependency)
            .flat_map(|r| &r.targets)
            .map(|t| t.id.address.as_str())
            .collect();
        assert_eq!(targets.len(), 2);
        assert!(targets.contains(&"aws_s3_bucket.tagged"));
        assert!(targets.contains(&"aws_cloudwatch_log_group.app"));
    }
}

#[test]
fn log_group_cards_preserve_reference_meaning_scope_and_selection() {
    let parsed = plan::parse(FIXTURE).unwrap();
    let graph = semantic::transform(&parsed);
    assert_eq!(graph.coverage.selected.resources, 6);
    assert!(
        parsed
            .attributes
            .iter()
            .all(|r| !r.service_reference_verified)
    );
    for node in &graph.nodes {
        if node.resource_type == "aws_cloudwatch_log_group" {
            assert_eq!(node.placement, Placement::Spatial);
            assert!(node.member_owner.is_none());
            assert_eq!(
                node.entity
                    .scope
                    .as_ref()
                    .and_then(|s| s.region.as_ref())
                    .is_some(),
                node.address != "aws_cloudwatch_log_group.unknown"
            );
        }
    }
    assert!(
        graph
            .edges
            .iter()
            .all(|e| e.kind == crate::model::EdgeKind::Dependency)
    );
    for consumer in [
        "aws_lambda_function.app",
        "aws_ecs_task_definition.app",
        "aws_s3_bucket.tagged",
    ] {
        let node = graph.nodes.iter().find(|n| n.address == consumer).unwrap();
        let rows = view::references::rows(node);
        assert!(!rows.is_empty());
        assert!(
            rows.iter()
                .filter(|r| r.reference.kind != crate::model::ReferenceKind::Information)
                .all(|r| matches!(r.reference.label, "References" | "Dependency"))
        );
        let selected = view::single(
            &graph,
            &view::Options {
                focus: Some(consumer.into()),
                depth: Some(0),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(selected.coverage.selected.resources, 1);
        let output = svg::render(&selected, &Layout::new(&selected));
        assert!(output.contains("Unavailable"));
    }
    for case in ["partial", "ambiguous", "deposed"] {
        let mut value: Value = serde_json::from_str(FIXTURE).unwrap();
        match case {
            "partial" => {
                value["configuration"]["root_module"]["resources"][1]["expressions"]["logging_config"]
                    ["log_group"] =
                    json!({"references":["aws_cloudwatch_log_group.app.name", "var.missing"]})
            }
            "ambiguous" => {
                value["resource_changes"][0]["address"] = json!("aws_cloudwatch_log_group.app[0]");
                let mut other = value["resource_changes"][0].clone();
                other["address"] = json!("aws_cloudwatch_log_group.app[1]");
                value["resource_changes"]
                    .as_array_mut()
                    .unwrap()
                    .push(other);
            }
            _ => value["resource_changes"][1]["deposed"] = json!("old"),
        }
        let graph = semantic::transform(&plan::parse(&value.to_string()).unwrap());
        let consumer = graph
            .nodes
            .iter()
            .find(|n| n.address == "aws_lambda_function.app")
            .unwrap();
        if case == "deposed" {
            assert!(consumer.references.is_empty());
        } else {
            let rows = view::references::rows(consumer);
            assert!(!rows.is_empty());
            assert!(rows.iter().all(|row| row.target.is_none()), "{case}");
        }
    }
    let input = FIXTURE.replace(
        "aws_cloudwatch_log_group.app",
        "module.logs.aws_cloudwatch_log_group.app",
    );
    let graph = semantic::transform(&plan::parse(&input).unwrap());
    let selected = view::single(
        &graph,
        &view::Options {
            modules: vec!["module.logs".into()],
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(selected.coverage.selected.resources, 1);
    assert_eq!(selected.nodes.len(), 1);
    assert_eq!(selected.nodes[0].placement, Placement::Spatial);
}

#[test]
fn log_group_long_lifecycle_identity_survives_named_state_exports() {
    let address = format!("aws_cloudwatch_log_group.app[\"{}<&\"]", "long".repeat(60));
    let mut value: Value = serde_json::from_str(&FIXTURE.replace(
        "aws_cloudwatch_log_group.app",
        &address.replace('"', "\\\""),
    ))
    .unwrap();
    value["resource_changes"][0]["previous_address"] = json!("aws_cloudwatch_log_group.old");
    value["resource_changes"][0]["change"]["importing"] = json!({"id":"PRIVATE_IMPORT"});
    let mut old = value["resource_changes"][0].clone();
    old["deposed"] = json!("old");
    old["change"]["actions"] = json!(["delete"]);
    value["resource_changes"].as_array_mut().unwrap().push(old);
    let graph = semantic::transform(&plan::parse(&value.to_string()).unwrap());
    assert_eq!(graph.coverage.selected.resources, 7);
    let architecture = crate::model::architecture::Architecture {
        states: ["one", "two"]
            .into_iter()
            .map(|s| (crate::model::StateId::new(s).unwrap(), graph.clone()))
            .collect(),
        relationships: Vec::new(),
    };
    let selected = view::apply(&architecture, &Default::default()).unwrap();
    let output = svg::render_states(&selected, svg::Options::default());
    let doc = roxmltree::Document::parse(&output).unwrap();
    assert_eq!(
        doc.descendants()
            .filter(|n| n.attribute("data-terraform-address") == Some(address.as_str()))
            .count(),
        4
    );
    for href in doc.descendants().filter_map(|n| n.attribute("href")) {
        assert!(
            doc.descendants()
                .any(|n| n.attribute("id") == href.strip_prefix('#'))
        );
    }
    let html = crate::html::render_states(&selected, svg::Options::default());
    assert!(!html.contains("PRIVATE_"));
    for format in [crate::OutputFormat::Svg, crate::OutputFormat::Html] {
        let output = crate::generate(
            &value.to_string(),
            &crate::Options {
                format,
                icon_mode: crate::IconMode::ServiceName,
                ..Default::default()
            },
        )
        .unwrap()
        .content;
        assert!(!output.contains("PRIVATE_"));
        assert!(output.contains("aws_cloudwatch_log_group.old"));
    }
}
