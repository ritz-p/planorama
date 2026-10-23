use crate::{plan, semantic};
use serde_json::{Value, json};
const FIXTURE: &str = include_str!("../../fixtures/cloudtrail-plan.json");

#[test]
fn cloudtrail_keeps_home_region_without_replicating_organization_or_multiregion_trails() {
    for multi in [false, true] {
        for organization in [false, true] {
            let mut value: Value = serde_json::from_str(FIXTURE).unwrap();
            value["resource_changes"][4]["change"]["after"]["is_multi_region_trail"] = json!(multi);
            value["resource_changes"][4]["change"]["after"]["is_organization_trail"] =
                json!(organization);
            let graph = semantic::transform(&plan::parse(&value.to_string()).unwrap());
            assert_eq!(graph.coverage.selected.resources, 5);
            let trails: Vec<_> = graph
                .nodes
                .iter()
                .filter(|n| n.resource_type == "aws_cloudtrail")
                .collect();
            assert_eq!(trails.len(), 1);
            assert!(!trails[0].entity.scope.as_ref().unwrap().global);
            assert_eq!(
                trails[0]
                    .entity
                    .scope
                    .as_ref()
                    .unwrap()
                    .region
                    .as_ref()
                    .unwrap()
                    .0,
                "us-east-1"
            );
            assert_eq!(
                trails[0].references.iter().filter(|r| r.complete).count(),
                4
            );
        }
    }
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
        assert_eq!(
            output.contains("planorama-icon-aws-cloudtrail"),
            cfg!(feature = "icon-artwork")
        );
        assert!(!output.contains("PRIVATE_"));
        assert!(!output.contains("arn:aws:"));
    }
}

#[test]
fn cloudtrail_log_group_reference_requires_full_known_value_and_exact_endpoint() {
    for case in [
        "unknown",
        "source-unknown",
        "missing-arn",
        "suffix",
        "ambiguous",
        "wrong-provider",
        "wrong-type",
        "delete",
        "deposed",
        "data",
    ] {
        let mut value: Value = serde_json::from_str(FIXTURE).unwrap();
        match case {
            "unknown" => {
                value["resource_changes"][4]["change"]["after_unknown"] =
                    json!({"cloud_watch_logs_group_arn":true})
            }
            "source-unknown" => {
                value["resource_changes"][1]["change"]["after_unknown"] = json!({"arn":true})
            }
            "missing-arn" => {
                value["resource_changes"][1]["change"]["after"]["arn"] = Value::Null;
                value["resource_changes"][4]["change"]["after"]["cloud_watch_logs_group_arn"] =
                    json!("invalid");
            }
            "suffix" => {
                value["resource_changes"][4]["change"]["after"]["cloud_watch_logs_group_arn"] =
                    json!("arn:aws:logs:us-east-1:123456789012:log-group:fixture:other:*")
            }
            "ambiguous" => {
                value["configuration"]["root_module"]["resources"][4]["expressions"]["cloud_watch_logs_group_arn"] = json!({"references":["aws_cloudwatch_log_group.audit.arn","aws_iam_role.audit.arn"]})
            }
            "wrong-provider" => {
                value["resource_changes"][1]["provider_name"] = json!("example.com/foreign/aws")
            }
            "wrong-type" => value["resource_changes"][1]["type"] = json!("aws_sqs_queue"),
            "delete" => value["resource_changes"][1]["change"]["actions"] = json!(["delete"]),
            "deposed" => value["resource_changes"][1]["deposed"] = json!("old"),
            "data" => value["resource_changes"][1]["mode"] = json!("data"),
            _ => unreachable!(),
        }
        let parsed = plan::parse(&value.to_string()).unwrap();
        assert!(
            parsed
                .attributes
                .iter()
                .filter(|r| r.attribute == "cloud_watch_logs_group_arn")
                .all(|r| !r.service_reference_verified),
            "{case}"
        );
    }
}

#[test]
fn cloudtrail_keeps_filtered_indexed_and_named_state_identity() {
    let indexed = FIXTURE.replace(
        "aws_cloudwatch_log_group.audit",
        "aws_cloudwatch_log_group.audit[0]",
    );
    let graph = semantic::transform(&plan::parse(&indexed).unwrap());
    let selected = crate::view::single(
        &graph,
        &crate::view::Options {
            focus: Some("aws_cloudtrail.audit".into()),
            depth: Some(0),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(selected.coverage.selected.resources, 1);
    assert!(
        crate::svg::render(&selected, &crate::layout::Layout::new(&selected))
            .contains("Unavailable")
    );
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
}
