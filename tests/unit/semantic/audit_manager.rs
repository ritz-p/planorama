#[test]
fn configured_links_require_current_typed_evidence() {
    crate::configured_service_links::check(
        include_str!("../../fixtures/audit-manager-plan.json"),
        crate::plan::parse,
        super::label,
        &[
            ("aws_auditmanager_account_registration.sample0", "kms_key"),
            (
                "aws_auditmanager_account_registration.sample0",
                "delegated_admin_account",
            ),
            ("aws_auditmanager_assessment.sample3", "framework_id"),
            ("aws_auditmanager_assessment.sample3", "roles.role_arn"),
            (
                "aws_auditmanager_assessment.sample3",
                "scope.aws_accounts.id",
            ),
            (
                "aws_auditmanager_assessment.sample3",
                "assessment_reports_destination.destination",
            ),
            (
                "aws_auditmanager_assessment_delegation.sample7",
                "assessment_id",
            ),
            (
                "aws_auditmanager_assessment_report.sample8",
                "assessment_id",
            ),
            ("aws_auditmanager_assessment_delegation.sample7", "role_arn"),
            (
                "aws_auditmanager_framework.sample4",
                "control_sets.controls.id",
            ),
            ("aws_auditmanager_framework_share.sample10", "framework_id"),
            (
                "aws_auditmanager_framework_share.sample10",
                "destination_account",
            ),
            (
                "aws_auditmanager_organization_admin_account_registration.sample11",
                "admin_account_id",
            ),
        ],
        ("aws_kms_key.sample1", "arn"),
    );
}
