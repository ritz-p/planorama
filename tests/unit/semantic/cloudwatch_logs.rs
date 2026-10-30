use crate::{plan, semantic};
use serde_json::json;

#[test]
fn log_helpers_require_explicit_known_typed_current_group() {
    for kind in [
        "aws_cloudwatch_log_stream",
        "aws_cloudwatch_log_metric_filter",
        "aws_cloudwatch_log_index_policy",
        "aws_cloudwatch_log_data_protection_policy",
    ] {
        for case in [
            "valid", "unknown", "literal", "mismatch", "deleted", "data", "provider", "deposed",
        ] {
            let address = format!("{kind}.helper");
            let mut v = json!({
                "format_version":"1.2", "terraform_version":"1.9.8",
                "resource_changes":[
                    {"address":"aws_cloudwatch_log_group.group","mode":"managed","type":"aws_cloudwatch_log_group","provider_name":"registry.terraform.io/hashicorp/aws","change":{"actions":["create"],"after":{"name":"synthetic","arn":"arn:aws:logs:us-east-1:123456789012:log-group:synthetic"}}},
                    {"address":address,"mode":"managed","type":kind,"provider_name":"registry.terraform.io/hashicorp/aws","change":{"actions":["create"],"after":{"log_group_name":"synthetic"}}}
                ],
                "configuration":{"provider_config":{"aws":{"name":"aws","full_name":"registry.terraform.io/hashicorp/aws","expressions":{"region":{"constant_value":"us-east-1"}}}},"root_module":{"resources":[
                    {"address":"aws_cloudwatch_log_group.group","mode":"managed","type":"aws_cloudwatch_log_group","name":"group","provider_config_key":"aws","expressions":{}},
                    {"address":address,"mode":"managed","type":kind,"name":"helper","provider_config_key":"aws","expressions":{"log_group_name":{"references":["aws_cloudwatch_log_group.group.name"]}}}
                ]}}
            });
            match case {
                "unknown" => {
                    v["resource_changes"][0]["change"]["after_unknown"] = json!({"name":true})
                }
                "literal" => {
                    v["configuration"]["root_module"]["resources"][1]["expressions"] = json!({})
                }
                "mismatch" => {
                    v["resource_changes"][1]["change"]["after"]["log_group_name"] = json!("other")
                }
                "deleted" => {
                    v["resource_changes"][0]["change"]["actions"] = json!(["delete"]);
                    v["resource_changes"][0]["change"]["before"] =
                        v["resource_changes"][0]["change"]["after"].take();
                }
                "data" => v["resource_changes"][0]["mode"] = json!("data"),
                "provider" => {
                    v["resource_changes"][0]["provider_name"] = json!("example.invalid/custom/aws")
                }
                "deposed" => v["resource_changes"][0]["deposed"] = json!("old"),
                _ => {}
            }
            let raw = plan::parse(&v.to_string()).unwrap();
            let graph = semantic::transform(&raw);
            let helper = graph.nodes.iter().find(|n| n.address == address).unwrap();
            assert_eq!(
                helper
                    .references
                    .iter()
                    .any(|r| r.label == "Configured log group" && r.complete),
                case == "valid",
                "{kind} {case}"
            );
        }
    }
}

fn pair(
    kind: &str,
    field: &str,
    endpoint: &str,
    output: &str,
    collection: bool,
) -> serde_json::Value {
    let target = format!("{kind}.helper");
    let source = format!("{endpoint}.source");
    let value = if output == "arn" {
        "arn:aws:logs:us-east-1:123456789012:log-group:synthetic"
    } else {
        "synthetic"
    };
    json!({"format_version":"1.2","resource_changes":[
        {"address":source,"mode":"managed","type":endpoint,"provider_name":"registry.terraform.io/hashicorp/aws","change":{"actions":["create"],"after":{output:value}}},
        {"address":target,"mode":"managed","type":kind,"provider_name":"registry.terraform.io/hashicorp/aws","change":{"actions":["create"],"after":{field:if collection {json!([value])}else{json!(value)},"policy_scope":"RESOURCE"}}}
    ],"configuration":{"provider_config":{"aws":{"name":"aws","full_name":"registry.terraform.io/hashicorp/aws","expressions":{"region":{"constant_value":"us-east-1"}}}},"root_module":{"resources":[
        {"address":source,"mode":"managed","type":endpoint,"name":"source","provider_config_key":"aws","expressions":{}},
        {"address":target,"mode":"managed","type":kind,"name":"helper","provider_config_key":"aws","expressions":{field:{"references":[format!("{source}.{output}")]}}}
    ]}}})
}
fn verified(v: &serde_json::Value, field: &str) -> bool {
    let raw = plan::parse(&v.to_string()).unwrap();
    raw.attributes
        .iter()
        .any(|r| r.attribute == field && r.service_reference_verified)
}
#[test]
fn collection_arn_and_key_contracts_reject_incomplete_or_mismatched_evidence() {
    for (kind, field, endpoint, output, collection) in [
        (
            "aws_cloudwatch_log_anomaly_detector",
            "log_group_arn_list",
            "aws_cloudwatch_log_group",
            "arn",
            true,
        ),
        (
            "aws_cloudwatch_query_definition",
            "log_group_names",
            "aws_cloudwatch_log_group",
            "name",
            true,
        ),
        (
            "aws_cloudwatch_log_transformer",
            "log_group_arn",
            "aws_cloudwatch_log_group",
            "arn",
            false,
        ),
        (
            "aws_cloudwatch_log_resource_policy",
            "resource_arn",
            "aws_cloudwatch_log_group",
            "arn",
            false,
        ),
        (
            "aws_cloudwatch_log_group",
            "kms_key_id",
            "aws_kms_key",
            "arn",
            false,
        ),
        (
            "aws_cloudwatch_log_anomaly_detector",
            "kms_key_id",
            "aws_kms_key",
            "key_id",
            false,
        ),
        (
            "aws_cloudwatch_log_anomaly_detector",
            "kms_key_id",
            "aws_kms_key",
            "arn",
            false,
        ),
    ] {
        let base = pair(kind, field, endpoint, output, collection);
        assert!(verified(&base, field), "{kind} {field}");
        for case in [
            "mismatch",
            "unknown",
            "sensitive",
            "wrong_type",
            "wrong_output",
            "ambiguous",
            "region",
            "incomplete",
        ] {
            let mut v = base.clone();
            match case {
                "mismatch" => v["resource_changes"][0]["change"]["after"][output] = json!("other"),
                "unknown" => {
                    v["resource_changes"][1]["change"]["after_unknown"] = json!({field:true})
                }
                "sensitive" => {
                    v["resource_changes"][0]["change"]["after_sensitive"] = json!({output:true})
                }
                "wrong_type" => v["resource_changes"][0]["type"] = json!("aws_s3_bucket"),
                "ambiguous" => {
                    v["configuration"]["root_module"]["resources"][1]["expressions"][field] = json!({"references":[format!("{endpoint}.source.{output}"), format!("{endpoint}.missing.{output}")]});
                }
                "wrong_output" => {
                    v["configuration"]["root_module"]["resources"][1]["expressions"][field] =
                        json!({"references":[format!("{endpoint}.source.id")]})
                }
                "region" => {
                    v["configuration"]["provider_config"]["aws.other"] = json!({"name":"aws","full_name":"registry.terraform.io/hashicorp/aws","alias":"other","expressions":{"region":{"constant_value":"eu-west-1"}}});
                    v["configuration"]["root_module"]["resources"][0]["provider_config_key"] =
                        json!("aws.other")
                }
                "incomplete" => {
                    if !collection {
                        continue;
                    }
                    v["resource_changes"][1]["change"]["after"][field]
                        .as_array_mut()
                        .unwrap()
                        .push(json!("unreferenced"));
                }
                _ => unreachable!(),
            }
            assert!(!verified(&v, field), "{kind} {field} {case}");
        }
    }
}

#[test]
fn independent_helpers_keep_coverage_state_boundaries_and_valid_filtered_links() {
    let input = include_str!("../../fixtures/cloudwatch-logs-plan.json");
    let plans = plan::parse_inputs(
        ["first", "second"]
            .into_iter()
            .map(|state| crate::model::PlanInput {
                state_id: crate::model::StateId::new(state).unwrap(),
                plan_json: input.into(),
            })
            .collect(),
    )
    .unwrap();
    assert!(semantic::cross_state::resolve(&plans, &[]).edges.is_empty());
    for raw in plans.states.values() {
        assert_eq!(semantic::transform(raw).coverage.selected.resources, 10);
        for focus in [None, Some("aws_cloudwatch_log_stream.helper".into())] {
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

#[test]
fn collection_and_policy_discriminators_do_not_guess() {
    let base = pair(
        "aws_cloudwatch_log_anomaly_detector",
        "log_group_arn_list",
        "aws_cloudwatch_log_group",
        "arn",
        true,
    );
    for value in [
        json!([]),
        json!([null]),
        json!(["synthetic", "synthetic"]),
        json!("synthetic"),
    ] {
        let mut v = base.clone();
        v["resource_changes"][1]["change"]["after"]["log_group_arn_list"] = value;
        assert!(!verified(&v, "log_group_arn_list"));
    }
    let base = pair(
        "aws_cloudwatch_log_resource_policy",
        "resource_arn",
        "aws_cloudwatch_log_group",
        "arn",
        false,
    );
    for scope in [json!("ACCOUNT"), json!(null), json!("invalid")] {
        let mut v = base.clone();
        v["resource_changes"][1]["change"]["after"]["policy_scope"] = scope;
        assert!(!verified(&v, "resource_arn"));
    }
    let mut v = base;
    v["resource_changes"][1]["change"]["after_unknown"] = json!({"policy_scope":true});
    assert!(!verified(&v, "resource_arn"));
}
