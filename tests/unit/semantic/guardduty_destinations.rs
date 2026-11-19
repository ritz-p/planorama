#[test]
fn configured_links_require_current_typed_evidence() {
    crate::configured_service_links::check(
        include_str!("../../fixtures/guardduty-destinations-plan.json"),
        crate::plan::parse,
        super::label,
        &[
        ("aws_guardduty_malware_protection_plan.sample0", "role"),
        (
            "aws_guardduty_malware_protection_plan.sample0",
            "protected_resource.s3_bucket.bucket_name",
        ),
        (
            "aws_guardduty_publishing_destination.sample3",
            "detector_id",
        ),
        (
            "aws_guardduty_publishing_destination.sample3",
            "destination_arn",
        ),
        (
            "aws_guardduty_publishing_destination.sample3",
            "kms_key_arn",
        ),
    ],
        ("aws_iam_role.sample1", "arn"),
    );
}
