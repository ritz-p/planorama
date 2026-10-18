use crate::{model::*, plan, semantic};
use serde_json::{Value, json};

fn fixture() -> Value {
    serde_json::from_str(include_str!("../../fixtures/ebs-plan.json")).unwrap()
}

fn graph(value: &Value) -> ArchitectureGraph {
    semantic::transform(&plan::parse(&value.to_string()).unwrap())
}

#[test]
fn ebs_attachment_keeps_identity_actions_scope_and_lifecycle_provenance() {
    for (actions, action) in [
        (json!(["create"]), Action::Create),
        (json!(["update"]), Action::Update),
        (json!(["delete", "create"]), Action::Replace),
        (json!(["no-op"]), Action::Unchanged),
    ] {
        let mut value = fixture();
        value["resource_changes"][2]["change"]["actions"] = actions;
        value["resource_changes"][2]["previous_address"] = json!("aws_volume_attachment.old");
        value["resource_changes"][2]["change"]["importing"] = json!({"id":"PRIVATE_IMPORT"});
        let result = graph(&value);
        assert_eq!(result.coverage.selected.resources, 5);
        assert_eq!(result.nodes.len(), 4);
        let edge = result
            .edges
            .iter()
            .find(|e| e.kind == EdgeKind::Association)
            .unwrap();
        assert_eq!(result.nodes[edge.from].resource_type, "aws_instance");
        assert_eq!(result.nodes[edge.to].resource_type, "aws_ebs_volume");
        assert_eq!(edge.directionality(), Directionality::Undirected);
        let change = edge.change.as_ref().unwrap();
        assert_eq!(change.address, "aws_volume_attachment.data");
        assert_eq!(change.action, action);
        assert_eq!(
            change.previous_address.as_deref(),
            Some("aws_volume_attachment.old")
        );
        assert!(result.relationships.iter().any(|r| r.provenance.iter().any(|p| matches!(p,
            RelationshipProvenance::Resource { change: c, deployment: Some(d), .. }
                if c == change && d.scope.as_ref().is_some_and(|s| !s.global && s.region.as_ref().is_some_and(|r| r.0 == "us-east-1"))
        ))));
        assert!(
            !result
                .edges
                .iter()
                .any(|e| result.nodes[e.to].resource_type == "aws_ebs_volume"
                    && e.kind == EdgeKind::Containment)
        );
        for format in [crate::OutputFormat::Svg, crate::OutputFormat::Html] {
            let output = crate::generate(
                &value.to_string(),
                &crate::Options {
                    format,
                    ..Default::default()
                },
            )
            .unwrap()
            .content;
            assert!(output.contains("aws_volume_attachment.data"));
            assert!(output.contains("aws_volume_attachment.old"));
            assert!(output.contains("data-import"));
            assert!(!output.contains("PRIVATE_IMPORT"));
        }
    }
}

#[test]
fn ebs_requires_both_complete_current_typed_id_proofs_and_preserves_fallbacks() {
    for (endpoint, attribute) in [(0, "instance_id"), (1, "volume_id")] {
        for case in 0..14 {
            let mut value = fixture();
            let source = value["resource_changes"][endpoint]["address"]
                .as_str()
                .unwrap()
                .to_owned();
            match case {
                0 => {
                    assert!(
                        value["configuration"]["root_module"]["resources"][2]["expressions"]
                            .as_object_mut()
                            .unwrap()
                            .remove(attribute)
                            .is_some()
                    );
                }
                1 => {
                    value["configuration"]["root_module"]["resources"][2]["expressions"]
                        [attribute] = json!({"constant_value":"literal"})
                }
                2 => {
                    value["configuration"]["root_module"]["resources"][2]["expressions"]
                        [attribute] = json!({"references":[format!("{source}.id"), "var.selector"]})
                }
                3 => {
                    value["resource_changes"][endpoint]["provider_name"] =
                        json!("example.com/foreign/aws")
                }
                4 => value["resource_changes"][endpoint]["type"] = json!("aws_s3_bucket"),
                5 => value["resource_changes"][endpoint]["change"]["after_unknown"] = json!(true),
                6 => {
                    value["resource_changes"][2]["change"]["after"][attribute] = json!("different")
                }
                7 => value["resource_changes"][endpoint]["mode"] = json!("data"),
                8 => value["resource_changes"][endpoint]["deposed"] = json!("old"),
                9 => value["resource_changes"][endpoint]["change"]["actions"] = json!(["delete"]),
                10 => value["resource_changes"][2]["change"]["actions"] = json!(["delete"]),
                11 => value["resource_changes"][2]["deposed"] = json!("old"),
                12 => {
                    value["resource_changes"][2]["provider_name"] = json!("example.com/foreign/aws")
                }
                _ => value["resource_changes"][2]["mode"] = json!("data"),
            }
            let parsed = plan::parse(&value.to_string()).unwrap();
            let result = semantic::transform(&parsed);
            assert!(
                !result.edges.iter().any(|e| e.kind == EdgeKind::Association),
                "{attribute}: {case}"
            );
            assert!(
                result
                    .nodes
                    .iter()
                    .any(|n| n.address == "aws_volume_attachment.data")
            );
            assert_eq!(result.coverage.selected.resources, 5);
            if case < 11 {
                assert!(
                    !semantic::diagnostics::collect(&parsed).is_empty(),
                    "{attribute}: {case}"
                );
            }
        }
    }
}

#[test]
fn ebs_indexed_ambiguous_parallel_and_deposed_attachments_remain_distinct() {
    let input = include_str!("../../fixtures/ebs-plan.json");
    let indexed = input
        .replace("aws_instance.app", "aws_instance.app[0]")
        .replace("aws_ebs_volume.data", "aws_ebs_volume.data[0]");
    let result = semantic::transform(&plan::parse(&indexed).unwrap());
    assert_eq!(
        result
            .edges
            .iter()
            .filter(|e| e.kind == EdgeKind::Association)
            .count(),
        1
    );
    let mut ambiguous = fixture();
    ambiguous["resource_changes"][1]["address"] = json!("aws_ebs_volume.data[0]");
    let mut other = ambiguous["resource_changes"][1].clone();
    other["address"] = json!("aws_ebs_volume.data[1]");
    ambiguous["resource_changes"]
        .as_array_mut()
        .unwrap()
        .push(other);
    assert!(
        !graph(&ambiguous)
            .edges
            .iter()
            .any(|e| e.kind == EdgeKind::Association)
    );
    let mut parallel = fixture();
    let mut other = parallel["resource_changes"][2].clone();
    other["address"] = json!("aws_volume_attachment.second");
    parallel["resource_changes"]
        .as_array_mut()
        .unwrap()
        .push(other);
    let mut config = parallel["configuration"]["root_module"]["resources"][2].clone();
    config["address"] = json!("aws_volume_attachment.second");
    parallel["configuration"]["root_module"]["resources"]
        .as_array_mut()
        .unwrap()
        .push(config);
    let mut old = parallel["resource_changes"][2].clone();
    old["deposed"] = json!("old");
    old["change"]["actions"] = json!(["delete"]);
    parallel["resource_changes"]
        .as_array_mut()
        .unwrap()
        .push(old);
    let result = graph(&parallel);
    assert_eq!(result.coverage.selected.resources, 7);
    assert_eq!(
        result
            .edges
            .iter()
            .filter(|e| e.kind == EdgeKind::Association)
            .count(),
        2
    );
    assert!(
        result
            .nodes
            .iter()
            .any(|n| n.address == "aws_volume_attachment.data" && n.deposed_key.is_some())
    );
}

#[test]
fn ebs_keeps_inline_blocks_with_instance_without_inventing_volumes() {
    let mut value = fixture();
    value["resource_changes"][0]["change"]["after"]["ebs_block_device"] = json!([{"volume_id":"vol-synthetic","device_name":"PRIVATE_DEVICE","snapshot_id":"PRIVATE_SNAPSHOT","encrypted":true}]);
    value["resource_changes"][1]["change"]["after"]["snapshot_id"] = json!("PRIVATE_SNAPSHOT");
    let result = graph(&value);
    assert_eq!(result.coverage.selected.resources, 5);
    assert_eq!(
        result
            .nodes
            .iter()
            .filter(|n| n.resource_type == "aws_ebs_volume")
            .count(),
        1
    );
    for format in [crate::OutputFormat::Svg, crate::OutputFormat::Html] {
        assert!(
            !crate::generate(
                &value.to_string(),
                &crate::Options {
                    format,
                    ..Default::default()
                }
            )
            .unwrap()
            .content
            .contains("PRIVATE_")
        );
    }
    value["configuration"]["root_module"]["resources"][2]["expressions"] = json!({});
    assert!(
        !graph(&value)
            .edges
            .iter()
            .any(|e| e.kind == EdgeKind::Association)
    );
}

#[test]
fn ebs_multi_state_and_filtered_endpoints_keep_coverage_and_safe_links() {
    let input = include_str!("../../fixtures/ebs-plan.json");
    let plans = plan::parse_inputs(
        ["one", "two"]
            .into_iter()
            .map(|state| PlanInput {
                state_id: StateId::new(state).unwrap(),
                plan_json: input.into(),
            })
            .collect(),
    )
    .unwrap();
    assert!(semantic::cross_state::resolve(&plans, &[]).edges.is_empty());
    for parsed in plans.states.values() {
        let result = semantic::transform(parsed);
        assert_eq!(result.coverage.selected.resources, 5);
        assert_eq!(
            result
                .edges
                .iter()
                .filter(|e| e.kind == EdgeKind::Association)
                .count(),
            1
        );
    }
    let result = graph(&fixture());
    let selected = crate::view::single(
        &result,
        &crate::view::Options {
            focus: Some("aws_instance.app".into()),
            depth: Some(0),
            ..Default::default()
        },
    )
    .unwrap();
    assert!(
        !selected
            .edges
            .iter()
            .any(|e| e.kind == EdgeKind::Association)
    );
    let mut fallback = fixture();
    fallback["resource_changes"]
        .as_array_mut()
        .unwrap()
        .push(json!({"address":"test.consumer","type":"test"}));
    fallback["configuration"]["root_module"]["resources"].as_array_mut().unwrap().push(json!({"address":"test.consumer","expressions":{"input":{"references":["aws_volume_attachment.data.id"]}}}));
    let result = graph(&fallback);
    assert!(
        result
            .nodes
            .iter()
            .any(|n| n.address == "aws_volume_attachment.data")
    );
    let selected = crate::view::single(
        &result,
        &crate::view::Options {
            focus: Some("aws_volume_attachment.data".into()),
            depth: Some(0),
            ..Default::default()
        },
    )
    .unwrap();
    let output = crate::svg::render(&selected, &crate::layout::Layout::new(&selected));
    assert!(output.contains("Unavailable"));
}
