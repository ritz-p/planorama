#[test]
fn configured_links_require_current_typed_evidence() {
    crate::configured_service_links::check(
        include_str!("../../fixtures/firewall-manager-plan.json"),
        crate::plan::parse,
        super::label,
        &[
        ("aws_fms_admin_account.sample0", "account_id"),
        ("aws_fms_policy.sample2", "resource_set_ids"),
        ("aws_fms_policy.sample2", "include_map.account"),
        ("aws_fms_policy.sample2", "include_map.orgunit"),
        ("aws_fms_policy.excluded", "exclude_map.account"),
        ("aws_fms_policy.excluded", "exclude_map.orgunit"),
    ],
        ("aws_organizations_account.sample1", "id"),
    );
}
