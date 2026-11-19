#[test]
fn configured_links_require_current_typed_evidence() {
    crate::configured_service_links::check(
        include_str!("../../fixtures/inspector-assessments-plan.json"),
        crate::plan::parse,
        super::label,
        &[
        (
            "aws_inspector_assessment_target.sample0",
            "resource_group_arn",
        ),
        ("aws_inspector_assessment_template.sample2", "target_arn"),
        (
            "aws_inspector_assessment_template.sample2",
            "event_subscription.topic_arn",
        ),
        (
            "aws_inspector2_delegated_admin_account.sample4",
            "account_id",
        ),
        ("aws_inspector2_member_association.sample6", "account_id"),
        ("aws_inspector2_enabler.sample7", "account_ids"),
    ],
        ("aws_inspector_resource_group.sample1", "arn"),
    );
}
