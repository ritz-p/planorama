#[test]
fn configured_links_require_current_typed_evidence() {
    crate::configured_service_links::check(
        include_str!("../../fixtures/config-rules-plan.json"),
        crate::plan::parse,
        super::label,
        &[
            ("aws_config_config_rule.sample0", "source.source_identifier"),
            (
                "aws_config_organization_custom_rule.sample2",
                "lambda_function_arn",
            ),
            (
                "aws_config_remediation_configuration.sample3",
                "config_rule_name",
            ),
            ("aws_config_remediation_configuration.sample3", "target_id"),
        ],
        ("aws_lambda_function.sample1", "arn"),
    );
}
