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

#[test]
fn export_uses_the_bucket_and_key_region() {
    use serde_json::{Value, json};
    let mut fixture: Value = serde_json::from_str(include_str!(
        "../../fixtures/guardduty-destinations-plan.json"
    ))
    .unwrap();
    let cfg = &mut fixture["configuration"];
    cfg["provider_config"]["aws.store"] = cfg["provider_config"]["aws"].clone();
    cfg["provider_config"]["aws.store"]["alias"] = json!("store");
    cfg["provider_config"]["aws.store"]["expressions"]["region"]["constant_value"] =
        json!("us-west-2");
    cfg["provider_config"]["aws.store"]["expressions"]["assume_role"] =
        json!([{"role_arn":{"constant_value":"arn:aws:iam::210987654321:role/Storage"}}]);
    cfg["root_module"]["resources"][2]["provider_config_key"] = json!("aws.store");
    cfg["root_module"]["resources"][5]["provider_config_key"] = json!("aws.store");
    let arn = json!("arn:aws:kms:us-west-2:210987654321:key/12345678-1234-1234-1234-123456789012");
    fixture["resource_changes"][5]["change"]["after"]["arn"] = arn.clone();
    fixture["resource_changes"][3]["change"]["after"]["kms_key_arn"] = arn;
    fixture["resource_changes"][3]["change"]["after"]["destination_arn"] =
        json!("arn:aws:s3:::fixture-export-bucket/audit/guardduty");
    let check = |v: &Value, path: &str| {
        let parsed = crate::plan::evidence::tests::parse_fixture(&v.to_string()).unwrap();
        crate::configured_service_links::verified(
            &crate::semantic::transform(&parsed),
            "aws_guardduty_publishing_destination.sample3",
            path,
            super::label("aws_guardduty_publishing_destination", path),
        )
    };
    let bucket = "destination_arn";
    let key = "kms_key_arn";
    assert!(check(&fixture, bucket));
    assert!(check(&fixture, key));
    let mut wrong = fixture.clone();
    wrong["resource_changes"][3]["change"]["after"]["destination_arn"] =
        json!("arn:aws:s3:::other-bucket/audit/guardduty");
    assert!(!check(&wrong, bucket));
    let mut detector = fixture.clone();
    detector["configuration"]["root_module"]["resources"][4]["provider_config_key"] =
        json!("aws.store");
    assert!(!check(&detector, "detector_id"));
    fixture["configuration"]["root_module"]["resources"][2]["provider_config_key"] = json!("aws");
    assert!(!check(&fixture, bucket));
    assert!(!check(&fixture, key));
}
