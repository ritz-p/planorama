#[test]
fn configured_links_require_current_typed_evidence() {
    crate::configured_service_links::check(
        include_str!("../../fixtures/config-delivery-plan.json"),
        crate::plan::parse,
        super::label,
        &[
            ("aws_config_configuration_recorder.sample0", "role_arn"),
            ("aws_config_configuration_recorder_status.sample2", "name"),
            ("aws_config_conformance_pack.sample3", "delivery_s3_bucket"),
            (
                "aws_config_organization_conformance_pack.sample5",
                "delivery_s3_bucket",
            ),
            ("aws_config_delivery_channel.sample6", "s3_bucket_name"),
            ("aws_config_delivery_channel.sample6", "sns_topic_arn"),
            ("aws_config_delivery_channel.sample6", "s3_kms_key_arn"),
        ],
        ("aws_iam_role.sample1", "arn"),
    );
}
