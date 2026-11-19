#[test]
fn configured_links_require_current_typed_evidence() {
    crate::configured_service_links::check(
        include_str!("../../fixtures/guardduty-members-plan.json"),
        crate::plan::parse,
        super::label,
        &[
        ("aws_guardduty_detector_feature.sample0", "detector_id"),
        ("aws_guardduty_filter.sample2", "detector_id"),
        ("aws_guardduty_invite_accepter.sample3", "detector_id"),
        ("aws_guardduty_ipset.sample4", "detector_id"),
        ("aws_guardduty_member.sample5", "detector_id"),
        (
            "aws_guardduty_member_detector_feature.sample6",
            "detector_id",
        ),
        (
            "aws_guardduty_organization_configuration.sample7",
            "detector_id",
        ),
        (
            "aws_guardduty_organization_configuration_feature.sample8",
            "detector_id",
        ),
        ("aws_guardduty_threatintelset.sample9", "detector_id"),
        ("aws_guardduty_member.sample5", "account_id"),
        (
            "aws_guardduty_member_detector_feature.sample6",
            "account_id",
        ),
        (
            "aws_guardduty_organization_admin_account.sample11",
            "admin_account_id",
        ),
        ("aws_guardduty_invite_accepter.sample3", "master_account_id"),
    ],
        ("aws_guardduty_detector.sample1", "id"),
    );
}
