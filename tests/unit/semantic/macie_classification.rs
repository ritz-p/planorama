#[test]
fn configured_links_require_current_typed_evidence() {
    crate::configured_service_links::check(
        include_str!("../../fixtures/macie-classification-plan.json"),
        crate::plan::evidence::tests::parse_fixture,
        super::label,
        &[
            (
                "aws_macie2_classification_export_configuration.sample0",
                "s3_destination.bucket_name",
            ),
            (
                "aws_macie2_classification_export_configuration.sample0",
                "s3_destination.kms_key_arn",
            ),
            (
                "aws_macie2_classification_job.sample3",
                "custom_data_identifier_ids",
            ),
            (
                "aws_macie2_classification_job.sample3",
                "s3_job_definition.bucket_definitions.buckets",
            ),
            (
                "aws_macie2_classification_job.sample3",
                "s3_job_definition.bucket_definitions.account_id",
            ),
            (
                "aws_macie2_invitation_accepter.sample6",
                "administrator_account_id",
            ),
            ("aws_macie2_member.sample7", "account_id"),
            (
                "aws_macie2_organization_admin_account.sample8",
                "admin_account_id",
            ),
        ],
        ("aws_s3_bucket.sample1", "bucket"),
    );
}

#[test]
fn explicit_buckets_must_match_the_account_in_the_same_definition() {
    use serde_json::{Value, json};
    let mut fixture: Value = serde_json::from_str(include_str!(
        "../../fixtures/macie-classification-plan.json"
    ))
    .unwrap();
    let check = |v: &Value| {
        let parsed = crate::plan::evidence::tests::parse_fixture(&v.to_string()).unwrap();
        crate::configured_service_links::verified(
            &crate::semantic::transform(&parsed),
            "aws_macie2_classification_job.sample3",
            "s3_job_definition.bucket_definitions.buckets",
            super::label(
                "aws_macie2_classification_job",
                "s3_job_definition.bucket_definitions.buckets",
            ),
        )
    };
    let cfg = &mut fixture["configuration"];
    cfg["provider_config"]["aws.bucket"] = cfg["provider_config"]["aws"].clone();
    cfg["provider_config"]["aws.bucket"]["alias"] = json!("bucket");
    cfg["provider_config"]["aws.bucket"]["expressions"]["assume_role"][0]["role_arn"]["constant_value"] =
        json!("arn:aws:iam::210987654321:role/Bucket");
    cfg["root_module"]["resources"][1]["provider_config_key"] = json!("aws.bucket");
    fixture["resource_changes"][3]["change"]["after"]["s3_job_definition"][0]["bucket_definitions"]
        [0]["account_id"] = json!("210987654321");
    assert!(check(&fixture));
    fixture["resource_changes"][3]["change"]["after"]["s3_job_definition"][0]["bucket_definitions"]
        [0]["account_id"] = json!("123456789012");
    assert!(!check(&fixture));
}

#[test]
fn organization_targets_can_be_managed_in_a_separate_account() {
    crate::configured_service_links::organization_targets(
        include_str!("../../fixtures/macie-classification-plan.json"),
        super::label,
        &[
            (
                "aws_macie2_classification_job.sample3",
                "s3_job_definition.bucket_definitions.account_id",
            ),
            (
                "aws_macie2_invitation_accepter.sample6",
                "administrator_account_id",
            ),
            ("aws_macie2_member.sample7", "account_id"),
            (
                "aws_macie2_organization_admin_account.sample8",
                "admin_account_id",
            ),
        ],
    );
}

#[test]
fn export_uses_the_bucket_and_key_region() {
    use serde_json::{Value, json};
    let mut fixture: Value = serde_json::from_str(include_str!(
        "../../fixtures/macie-classification-plan.json"
    ))
    .unwrap();
    let cfg = &mut fixture["configuration"];
    cfg["provider_config"]["aws.store"] = cfg["provider_config"]["aws"].clone();
    cfg["provider_config"]["aws.store"]["alias"] = json!("store");
    cfg["provider_config"]["aws.store"]["expressions"]["region"]["constant_value"] =
        json!("us-west-2");
    cfg["provider_config"]["aws.store"]["expressions"]["assume_role"] =
        json!([{"role_arn":{"constant_value":"arn:aws:iam::210987654321:role/Storage"}}]);
    cfg["root_module"]["resources"][1]["provider_config_key"] = json!("aws.store");
    cfg["root_module"]["resources"][2]["provider_config_key"] = json!("aws.store");
    let arn = json!("arn:aws:kms:us-west-2:210987654321:key/12345678-1234-1234-1234-123456789012");
    fixture["resource_changes"][2]["change"]["after"]["arn"] = arn.clone();
    fixture["resource_changes"][0]["change"]["after"]["s3_destination"][0]["kms_key_arn"] = arn;

    let check = |v: &Value, path: &str| {
        let parsed = crate::plan::evidence::tests::parse_fixture(&v.to_string()).unwrap();
        crate::configured_service_links::verified(
            &crate::semantic::transform(&parsed),
            "aws_macie2_classification_export_configuration.sample0",
            path,
            super::label("aws_macie2_classification_export_configuration", path),
        )
    };
    let bucket = "s3_destination.bucket_name";
    let key = "s3_destination.kms_key_arn";
    assert!(check(&fixture, bucket));
    assert!(check(&fixture, key));

    fixture["configuration"]["root_module"]["resources"][1]["provider_config_key"] = json!("aws");
    assert!(!check(&fixture, bucket));
    assert!(!check(&fixture, key));
}
