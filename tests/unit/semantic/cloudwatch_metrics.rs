use crate::{plan, semantic};
use serde_json::{Value, json};

fn fixture() -> Value {
    serde_json::from_str(include_str!("../../fixtures/cloudwatch-metrics-plan.json")).unwrap()
}
fn verified(v: &Value, target: &str, field: &str) -> bool {
    let raw = plan::parse(&v.to_string()).unwrap();
    semantic::transform(&raw)
        .nodes
        .iter()
        .find(|n| n.address == target)
        .unwrap()
        .references
        .iter()
        .any(|r| {
            r.attribute == field
                && r.complete
                && (r.label.starts_with("Configured") || r.label == "Metric stream role reference")
        })
}

#[test]
fn metric_configuration_requires_unique_typed_current_known_endpoints() {
    for (target, source, field) in [
        (4, 0, "firehose_arn"),
        (4, 1, "role_arn"),
        (5, 2, "resource_arn"),
        (6, 3, "resource_arn"),
    ] {
        for case in [
            "valid",
            "literal",
            "unknown",
            "target_unknown",
            "sensitive",
            "mismatch",
            "wrong_type",
            "data",
            "deleted",
            "deposed",
            "provider",
            "alias",
            "region",
            "wrong_output",
            "ambiguous",
            "wrong_arn",
        ] {
            let mut v = fixture();
            let address = v["resource_changes"][target]["address"]
                .as_str()
                .unwrap()
                .to_owned();
            match case {
                "literal" => v["configuration"]["root_module"]["resources"][target]["expressions"] = json!({}),
                "unknown" => v["resource_changes"][source]["change"]["after_unknown"] = json!({"arn":true}),
                "target_unknown" => v["resource_changes"][target]["change"]["after_unknown"] = json!({field:true}),
                "sensitive" => v["resource_changes"][target]["change"]["after_sensitive"] = json!({field:true}),
                "mismatch" => v["resource_changes"][source]["change"]["after"]["arn"] = json!("other"),
                "wrong_type" => v["resource_changes"][source]["type"] = json!("aws_s3_bucket"),
                "data" => v["resource_changes"][source]["mode"] = json!("data"),
                "deleted" => {v["resource_changes"][source]["change"]["actions"] = json!(["delete"]);v["resource_changes"][source]["change"]["before"] = v["resource_changes"][source]["change"]["after"].take();},
                "deposed" => v["resource_changes"][source]["deposed"] = json!("old"),
                "provider" => v["resource_changes"][source]["provider_name"] = json!("example.invalid/custom/aws"),
                "alias" | "region" => {v["configuration"]["provider_config"]["aws.other"] = json!({"name":"aws","full_name":"registry.terraform.io/hashicorp/aws","alias":"other","expressions":{"region":{"constant_value":if case=="region" {"eu-west-1"}else{"us-east-1"}}}});v["configuration"]["root_module"]["resources"][source]["provider_config_key"] = json!("aws.other");},
                "wrong_output" => {let source_address=v["resource_changes"][source]["address"].as_str().unwrap();v["configuration"]["root_module"]["resources"][target]["expressions"][field] = json!({"references":[format!("{source_address}.id")]});},
                "ambiguous" => v["configuration"]["root_module"]["resources"][target]["expressions"][field]["references"].as_array_mut().unwrap().push(json!("aws_s3_bucket.missing.arn")),
                "wrong_arn" => {v["resource_changes"][source]["change"]["after"]["arn"] = json!("arn:aws:s3:us-east-1:123456789012:bucket");v["resource_changes"][target]["change"]["after"][field] = v["resource_changes"][source]["change"]["after"]["arn"].clone();},
                _ => {}
            }
            assert_eq!(
                verified(&v, &address, field),
                case == "valid",
                "{address} {field} {case}"
            );
        }
    }
}

#[test]
fn insight_templates_and_detailed_metric_target_types_are_exact() {
    for name in [
        "VpcEndpointService-ActiveConnectionsByEndpointId-v1",
        "VpcEndpointService-BytesByEndpointId-v1",
        "VpcEndpointService-NewConnectionsByEndpointId-v1",
        "VpcEndpointService-RstPacketsByEndpointId-v1",
        "VpcEndpointService-BytesByEndpointId-v2",
        "arbitrary",
        "",
    ] {
        let mut v = fixture();
        v["resource_changes"][5]["change"]["after"]["template_name"] = json!(name);
        assert_eq!(
            verified(
                &v,
                "aws_cloudwatch_contributor_managed_insight_rule.insights",
                "resource_arn"
            ),
            name.ends_with("-v1"),
            "{name}"
        );
    }
    for mask in ["after_unknown", "after_sensitive"] {
        let mut v = fixture();
        v["resource_changes"][5]["change"][mask] = json!({"template_name":true});
        assert!(!verified(
            &v,
            "aws_cloudwatch_contributor_managed_insight_rule.insights",
            "resource_arn"
        ));
    }
    for kind in [
        "aws_elasticache_serverless_cache",
        "aws_elasticache_cluster",
    ] {
        let mut v = fixture();
        v["resource_changes"][3]["type"] = json!(kind);
        assert!(!verified(
            &v,
            "aws_cloudwatch_resource_metrics_configuration.detailed",
            "resource_arn"
        ));
    }
    let mut v = fixture();
    v["resource_changes"][4]["change"]["after_unknown"] = json!({"include_filter":true});
    v["resource_changes"][6]["change"]["after_unknown"] = json!({"metric_selection":true});
    assert!(verified(
        &v,
        "aws_cloudwatch_metric_stream.metrics",
        "firehose_arn"
    ));
    assert!(verified(
        &v,
        "aws_cloudwatch_resource_metrics_configuration.detailed",
        "resource_arn"
    ));
}

#[test]
fn metric_helpers_keep_identity_coverage_and_state_selection_boundaries() {
    let input = fixture().to_string();
    let plans = plan::parse_inputs(
        ["first", "second"]
            .into_iter()
            .map(|state| crate::model::PlanInput {
                state_id: crate::model::StateId::new(state).unwrap(),
                plan_json: input.clone(),
            })
            .collect(),
    )
    .unwrap();
    assert!(semantic::cross_state::resolve(&plans, &[]).edges.is_empty());
    for raw in plans.states.values() {
        let graph = semantic::transform(raw);
        assert_eq!(graph.coverage.selected.resources, 7);
        assert!(
            graph
                .nodes
                .iter()
                .all(|n| n.action == crate::model::Action::Create)
        );
        for focus in [None, Some("aws_cloudwatch_metric_stream.metrics".into())] {
            let graph = crate::generation::project(
                raw,
                &crate::view::Options {
                    depth: focus.as_ref().map(|_| 0),
                    focus,
                    ..Default::default()
                },
            )
            .unwrap();
            let svg = crate::svg::render(&graph, &crate::layout::Layout::new(&graph));
            assert!(!svg.contains("PRIVATE_"));
            let doc = roxmltree::Document::parse(&svg).unwrap();
            for href in doc
                .descendants()
                .filter_map(|n| n.attribute("href"))
                .filter_map(|s| s.strip_prefix('#'))
            {
                assert!(doc.descendants().any(|n| n.attribute("id") == Some(href)));
            }
        }
    }
}
