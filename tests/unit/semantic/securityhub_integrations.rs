#[test]
fn configured_links_require_current_typed_evidence() {
    crate::configured_service_links::check(
        include_str!("../../fixtures/securityhub-integrations-plan.json"),
        crate::plan::parse,
        super::label,
        &[
        (
            "aws_securityhub_automation_rule_v2.sample0",
            "action.external_integration_configuration.connector_arn",
        ),
        ("aws_securityhub_connector_v2.sample1", "kms_key_arn"),
        (
            "aws_securityhub_connector_v2.sample1",
            "connector_provider.service_now.secret_arn",
        ),
    ],
        ("aws_securityhub_connector_v2.sample1", "arn"),
    );
}
