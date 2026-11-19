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

#[test]
fn account_sources_may_be_managed_from_another_provider_account() {
    use serde_json::{Value, json};
    let mut v: Value =
        serde_json::from_str(include_str!("../../fixtures/config-aggregation-plan.json")).unwrap();
    let cfg = &mut v["configuration"];
    cfg["provider_config"]["aws.management"] = cfg["provider_config"]["aws"].clone();
    cfg["provider_config"]["aws.management"]["alias"] = json!("management");
    cfg["provider_config"]["aws.management"]["expressions"]["assume_role"] =
        json!([{"role_arn":{"constant_value":"arn:aws:iam::210987654321:role/Management"}}]);
    cfg["provider_config"]["aws"]["expressions"]["assume_role"] =
        json!([{"role_arn":{"constant_value":"arn:aws:iam::999999999999:role/Aggregator"}}]);
    cfg["root_module"]["resources"][1]["provider_config_key"] = json!("aws.management");
    let complete = |v: &Value| {
        let plan = crate::plan::evidence::tests::parse_fixture(&v.to_string()).unwrap();
        let path = "account_aggregation_source.account_ids";
        crate::configured_service_links::verified(
            &crate::semantic::transform(&plan),
            "aws_config_configuration_aggregator.sample2",
            path,
            super::label("aws_config_configuration_aggregator", path),
        )
    };
    assert!(complete(&v));
    v["resource_changes"][1]["change"]["after"]["id"] = json!("111111111111");
    assert!(!complete(&v));
}
