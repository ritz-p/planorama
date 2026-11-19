#[test]
fn configured_links_require_current_typed_evidence() {
    crate::configured_service_links::check(
        include_str!("../../fixtures/securityhub-organization-plan.json"),
        crate::plan::parse,
        super::label,
        &[
            (
                "aws_securityhub_configuration_policy_association.sample0",
                "policy_id",
            ),
            (
                "aws_securityhub_configuration_policy_association.sample0",
                "target_id",
            ),
            ("aws_securityhub_invite_accepter.sample3", "master_id"),
            ("aws_securityhub_member.sample4", "account_id"),
            (
                "aws_securityhub_organization_admin_account.sample5",
                "admin_account_id",
            ),
        ],
        ("aws_securityhub_configuration_policy.sample1", "id"),
    );
}

#[test]
fn self_managed_policy_is_never_a_managed_endpoint() {
    use serde_json::{Value, json};
    let mut fixture: Value = serde_json::from_str(include_str!(
        "../../fixtures/securityhub-organization-plan.json"
    ))
    .unwrap();
    let check = |v: &Value| {
        let parsed = crate::plan::parse(&v.to_string()).unwrap();
        crate::configured_service_links::verified(
            &crate::semantic::transform(&parsed),
            "aws_securityhub_configuration_policy_association.sample0",
            "policy_id",
            super::label(
                "aws_securityhub_configuration_policy_association",
                "policy_id",
            ),
        )
    };

    assert!(check(&fixture));
    fixture["resource_changes"][0]["change"]["after"]["policy_id"] =
        json!("SELF_MANAGED_SECURITY_HUB");
    fixture["resource_changes"][1]["change"]["after"]["id"] = json!("SELF_MANAGED_SECURITY_HUB");
    assert!(!check(&fixture));
}
