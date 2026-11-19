#[test]
fn configured_links_require_current_typed_evidence() {
    crate::configured_service_links::check(
        include_str!("../../fixtures/config-aggregation-plan.json"),
        crate::plan::parse,
        super::label,
        &[
            ("aws_config_aggregate_authorization.sample0", "account_id"),
            (
                "aws_config_configuration_aggregator.sample2",
                "account_aggregation_source.account_ids",
            ),
            (
                "aws_config_configuration_aggregator.sample2",
                "organization_aggregation_source.role_arn",
            ),
        ],
        ("aws_organizations_account.sample1", "id"),
    );
}
