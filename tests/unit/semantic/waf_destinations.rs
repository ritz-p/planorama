#[test]
fn configured_links_require_current_typed_evidence() {
    crate::configured_service_links::check(
        include_str!("../../fixtures/waf-destinations-plan.json"),
        crate::plan::parse,
        super::label,
        &[
            (
                "aws_wafv2_web_acl_logging_configuration.sample0",
                "resource_arn",
            ),
            (
                "aws_wafv2_web_acl_logging_configuration.sample0",
                "log_destination_configs",
            ),
        ],
        ("aws_wafv2_web_acl.sample1", "arn"),
    );
}

#[test]
fn destinations_require_the_correct_prefix_and_service() {
    use serde_json::{Value, json};
    let f: Value =
        serde_json::from_str(include_str!("../../fixtures/waf-destinations-plan.json")).unwrap();
    for (kind, arn) in [
        ("aws_s3_bucket", "arn:aws:s3:::aws-waf-logs-app"),
        (
            "aws_cloudwatch_log_group",
            "arn:aws:logs:us-east-1:123456789012:log-group:aws-waf-logs-app",
        ),
        (
            "aws_kinesis_firehose_delivery_stream",
            "arn:aws:firehose:us-east-1:123456789012:deliverystream/aws-waf-logs-app",
        ),
    ] {
        for (value, expected) in [
            (arn.to_owned(), true),
            (arn.replace("aws-waf-logs-", "unrelated-"), false),
        ] {
            let mut plan = f.clone();
            let addr = format!("{kind}.logs");
            let row = &mut plan["resource_changes"][2];
            row["type"] = json!(kind);
            row["address"] = json!(addr);
            row["name"] = json!("logs");
            row["change"]["after"] = json!({"arn":value});
            let cfg = &mut plan["configuration"]["root_module"]["resources"][2];
            cfg["type"] = json!(kind);
            cfg["address"] = json!(addr);
            cfg["name"] = json!("logs");
            plan["resource_changes"][0]["change"]["after"]["log_destination_configs"] =
                json!([value]);
            plan["configuration"]["root_module"]["resources"][0]["expressions"]["log_destination_configs"] =
                json!({"references":[format!("{addr}.arn")]});
            let parsed = crate::plan::parse(&plan.to_string()).unwrap();
            assert_eq!(
                crate::configured_service_links::verified(
                    &crate::semantic::transform(&parsed),
                    "aws_wafv2_web_acl_logging_configuration.sample0",
                    "log_destination_configs",
                    Some("Web ACL log destination")
                ),
                expected,
                "{kind}/{value}"
            );
        }
    }
}
