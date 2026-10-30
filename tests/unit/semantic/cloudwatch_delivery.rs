use crate::plan;
use serde_json::json;
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
fn delivery_references_require_exact_endpoint_type_value_and_lifecycle() {
    for (kind, field, endpoint, output) in [
        (
            "aws_cloudwatch_log_delivery",
            "delivery_source_name",
            "aws_cloudwatch_log_delivery_source",
            "name",
        ),
        (
            "aws_cloudwatch_log_delivery",
            "delivery_destination_arn",
            "aws_cloudwatch_log_delivery_destination",
            "arn",
        ),
        (
            "aws_cloudwatch_log_delivery_destination_policy",
            "delivery_destination_name",
            "aws_cloudwatch_log_delivery_destination",
            "name",
        ),
        (
            "aws_cloudwatch_log_destination_policy",
            "destination_name",
            "aws_cloudwatch_log_destination",
            "name",
        ),
        (
            "aws_cloudwatch_log_destination",
            "target_arn",
            "aws_kinesis_stream",
            "arn",
        ),
        (
            "aws_cloudwatch_log_destination",
            "role_arn",
            "aws_iam_role",
            "arn",
        ),
        (
            "aws_cloudwatch_log_subscription_filter",
            "log_group_name",
            "aws_cloudwatch_log_group",
            "name",
        ),
        (
            "aws_cloudwatch_log_subscription_filter",
            "destination_arn",
            "aws_lambda_function",
            "arn",
        ),
        (
            "aws_cloudwatch_log_subscription_filter",
            "destination_arn",
            "aws_kinesis_stream",
            "arn",
        ),
        (
            "aws_cloudwatch_log_subscription_filter",
            "destination_arn",
            "aws_kinesis_firehose_delivery_stream",
            "arn",
        ),
        (
            "aws_cloudwatch_log_subscription_filter",
            "destination_arn",
            "aws_cloudwatch_log_destination",
            "arn",
        ),
    ] {
        let base = pair(kind, field, endpoint, output, false);
        assert!(verified(&base, field), "{kind} {field} {endpoint}");
        for case in [
            "unknown",
            "sensitive",
            "mismatch",
            "literal",
            "ambiguous",
            "type",
            "data",
            "provider",
            "deleted",
            "deposed",
        ] {
            let mut v = base.clone();
            match case {
                "unknown" => {
                    v["resource_changes"][1]["change"]["after_unknown"] = json!({field:true})
                }
                "sensitive" => {
                    v["resource_changes"][0]["change"]["after_sensitive"] = json!({output:true})
                }
                "mismatch" => v["resource_changes"][0]["change"]["after"][output] = json!("other"),
                "literal" => {
                    v["configuration"]["root_module"]["resources"][1]["expressions"] = json!({})
                }
                "ambiguous" => {
                    v["configuration"]["root_module"]["resources"][1]["expressions"][field] = json!({"references":[format!("{endpoint}.source.{output}"),format!("{endpoint}.missing.{output}")]})
                }
                "type" => v["resource_changes"][0]["type"] = json!("aws_cloudwatch_log_group"),
                "data" => v["resource_changes"][0]["mode"] = json!("data"),
                "provider" => {
                    v["resource_changes"][0]["provider_name"] = json!("example.invalid/custom/aws")
                }
                "deleted" => {
                    v["resource_changes"][0]["change"]["actions"] = json!(["delete"]);
                    v["resource_changes"][0]["change"]["before"] =
                        v["resource_changes"][0]["change"]["after"].take();
                }
                "deposed" => v["resource_changes"][0]["deposed"] = json!("old"),
                _ => unreachable!(),
            }
            if case == "type" && endpoint == "aws_cloudwatch_log_group" {
                continue;
            }
            assert!(!verified(&v, field), "{kind} {field} {endpoint} {case}");
        }
    }
}

#[test]
fn destination_type_and_producer_log_type_are_required() {
    for (kind, discriminator) in [
        ("aws_s3_bucket", "S3"),
        ("aws_cloudwatch_log_group", "CWL"),
        ("aws_kinesis_firehose_delivery_stream", "FH"),
    ] {
        let field = "delivery_destination_configuration.destination_resource_arn";
        let mut v = pair(
            "aws_cloudwatch_log_delivery_destination",
            field,
            kind,
            "arn",
            false,
        );
        let arn = v["resource_changes"][0]["change"]["after"]["arn"].clone();
        v["resource_changes"][1]["change"]["after"] = json!({"delivery_destination_type":discriminator,"delivery_destination_configuration":[{"destination_resource_arn":arn}]});
        for typ in [discriminator, "XRAY", "unknown"] {
            v["resource_changes"][1]["change"]["after"]["delivery_destination_type"] = json!(typ);
            assert_eq!(verified(&v, field), typ == discriminator, "{kind} {typ}");
        }
    }
    for (kind, service, log_type) in [
        ("aws_cloudfront_distribution", "cloudfront", "ACCESS_LOGS"),
        (
            "aws_bedrockagent_knowledge_base",
            "bedrock",
            "APPLICATION_LOGS",
        ),
    ] {
        let mut v = pair(
            "aws_cloudwatch_log_delivery_source",
            "resource_arn",
            kind,
            "arn",
            false,
        );
        let arn = format!("arn:aws:{service}:us-east-1:123456789012:synthetic");
        v["resource_changes"][0]["change"]["after"]["arn"] = json!(arn);
        v["resource_changes"][1]["change"]["after"]["resource_arn"] = json!(arn);
        for typ in [log_type, "invalid"] {
            v["resource_changes"][1]["change"]["after"]["log_type"] = json!(typ);
            assert_eq!(
                verified(&v, "resource_arn"),
                typ == log_type,
                "{kind} {typ}"
            );
        }
    }
}

#[test]
fn only_explicit_log_destinations_cross_known_accounts_without_moving_scope() {
    let left = crate::model::ProviderConfiguration {
        key: "aws".into(),
        alias: None,
        region: Some(crate::model::RegionScope("us-east-1".into())),
        account: crate::model::AwsAccountScope::new("111111111111"),
        account_conflict: false,
    };
    let mut right = left.clone();
    right.key = "aws.other".into();
    right.account = crate::model::AwsAccountScope::new("222222222222");
    for (kind, field, expected) in [
        ("aws_cloudwatch_log_destination", "destination_arn", true),
        (
            "aws_cloudwatch_log_delivery_destination",
            "delivery_destination_arn",
            true,
        ),
        ("aws_lambda_function", "destination_arn", false),
        ("aws_kinesis_stream", "destination_arn", false),
        (
            "aws_kinesis_firehose_delivery_stream",
            "destination_arn",
            false,
        ),
        ("aws_cloudwatch_log_destination", "destination_name", false),
    ] {
        assert_eq!(
            super::binding(
                Some(&left),
                Some(&right),
                left.region.as_ref(),
                right.region.as_ref(),
                kind,
                field
            ),
            expected
        );
        assert!(!super::binding(
            Some(&left),
            Some(&right),
            left.region.as_ref(),
            Some(&crate::model::RegionScope("eu-west-1".into())),
            kind,
            field
        ));
    }
    right.account = None;
    assert!(!super::binding(
        Some(&left),
        Some(&right),
        left.region.as_ref(),
        right.region.as_ref(),
        "aws_cloudwatch_log_destination",
        "destination_arn"
    ));
}

#[test]
fn subscription_roles_require_stream_destinations_and_xray_has_no_endpoint() {
    let mut v = pair(
        "aws_cloudwatch_log_subscription_filter",
        "role_arn",
        "aws_iam_role",
        "arn",
        false,
    );
    for (service, expected) in [
        ("kinesis", true),
        ("firehose", true),
        ("lambda", false),
        ("logs", false),
    ] {
        v["resource_changes"][1]["change"]["after"]["destination_arn"] = json!(format!(
            "arn:aws:{service}:us-east-1:123456789012:synthetic"
        ));
        assert_eq!(verified(&v, "role_arn"), expected);
    }
    let mut v = pair(
        "aws_cloudwatch_log_subscription_filter",
        "destination_arn",
        "aws_lambda_function",
        "arn",
        false,
    );
    v["resource_changes"][1]["change"]["after_unknown"] = json!({"role_arn":true});
    assert!(!verified(&v, "destination_arn"));
}

#[test]
fn delivery_helpers_preserve_state_coverage_and_filtered_navigation() {
    let input = include_str!("../../fixtures/cloudwatch-delivery-plan.json");
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
    assert!(
        crate::semantic::cross_state::resolve(&plans, &[])
            .edges
            .is_empty()
    );
    for raw in plans.states.values() {
        assert_eq!(
            crate::semantic::transform(raw).coverage.selected.resources,
            11
        );
        for focus in [None, Some("aws_cloudwatch_log_delivery.delivery".into())] {
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
