use crate::{plan, semantic};
use serde_json::{Value, json};

const FIXTURE: &str = include_str!("../../fixtures/kinesis-owners-plan.json");

#[test]
fn consumer_and_policy_owners_require_explicit_matching_arn_and_scope() {
    let fixture: Value = serde_json::from_str(FIXTURE).unwrap();
    for (target, source, path, label) in [
        (1, 0, "stream_arn", "Consumer stream"),
        (2, 0, "resource_arn", "Kinesis policy resource"),
        (3, 1, "resource_arn", "Kinesis policy resource"),
    ] {
        let mut pair = fixture.clone();
        pair["resource_changes"] = json!([
            fixture["resource_changes"][source],
            fixture["resource_changes"][target]
        ]);
        pair["configuration"]["root_module"]["resources"] = json!([
            fixture["configuration"]["root_module"]["resources"][source],
            fixture["configuration"]["root_module"]["resources"][target]
        ]);
        super::super::service_references::tests::check_current_scalar_fixture(
            &pair.to_string(),
            label,
            path,
            "arn",
        );
    }
}

#[test]
fn policies_and_consumers_keep_independent_identity_without_arn_ownership_inference() {
    let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
    let graph = semantic::transform(&plan::parse(FIXTURE).unwrap());
    assert_eq!(graph.coverage.selected.resources, 4);
    assert!(graph.nodes.iter().all(|n| n.member_owner.is_none()));
    assert_eq!(
        graph
            .nodes
            .iter()
            .filter(|n| n.placement == crate::model::Placement::Logical)
            .count(),
        3
    );
    v["configuration"]["root_module"]["resources"][1]["expressions"] = json!({});
    let graph = semantic::transform(&plan::parse(&v.to_string()).unwrap());
    let consumer = graph
        .nodes
        .iter()
        .find(|n| n.resource_type == "aws_kinesis_stream_consumer")
        .unwrap();
    assert!(
        !consumer
            .references
            .iter()
            .any(|r| r.label == "Consumer stream" && r.complete)
    );
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
        assert!(result.content.contains("Consumer stream"));
        assert!(result.content.contains("Kinesis policy resource"));
    }
}
