#[test]
fn configured_links_require_current_typed_evidence() {
    crate::configured_service_links::check(
        include_str!("../../fixtures/fis-configuration-plan.json"),
        crate::plan::evidence::tests::parse_fixture,
        super::label,
        &[
            ("aws_fis_experiment_template.sample0", "role_arn"),
            (
                "aws_fis_experiment_template.sample0",
                "target.resource_arns",
            ),
            (
                "aws_fis_experiment_template.sample0",
                "stop_condition.value",
            ),
            (
                "aws_fis_experiment_template.sample0",
                "log_configuration.cloudwatch_logs_configuration.log_group_arn",
            ),
            (
                "aws_fis_experiment_template.sample0",
                "log_configuration.s3_configuration.bucket_name",
            ),
            (
                "aws_fis_experiment_template.sample0",
                "experiment_report_configuration.data_sources.cloudwatch_dashboard.dashboard_arn",
            ),
            (
                "aws_fis_experiment_template.sample0",
                "experiment_report_configuration.outputs.s3_configuration.bucket_name",
            ),
            (
                "aws_fis_target_account_configuration.sample7",
                "experiment_template_id",
            ),
            ("aws_fis_target_account_configuration.sample7", "account_id"),
            ("aws_fis_target_account_configuration.sample7", "role_arn"),
        ],
        ("aws_iam_role.sample1", "arn"),
    );
}

#[test]
fn target_type_must_match_the_referenced_resource() {
    use serde_json::{Value, json};
    let mut fixture: Value =
        serde_json::from_str(include_str!("../../fixtures/fis-configuration-plan.json")).unwrap();
    let check = |v: &Value| {
        let parsed = crate::plan::evidence::tests::parse_fixture(&v.to_string()).unwrap();
        crate::configured_service_links::verified(
            &crate::semantic::transform(&parsed),
            "aws_fis_experiment_template.sample0",
            "target.resource_arns",
            super::label("aws_fis_experiment_template", "target.resource_arns"),
        )
    };

    assert!(check(&fixture));
    fixture["resource_changes"][0]["change"]["after"]["target"][0]["resource_type"] =
        json!("aws:ec2:ebs-volume");
    assert!(!check(&fixture));
}

#[test]
fn cross_account_targets_require_the_matching_template_account_configuration() {
    use serde_json::{Value, json};
    let mut v: Value =
        serde_json::from_str(include_str!("../../fixtures/fis-configuration-plan.json")).unwrap();
    let config = &mut v["configuration"];
    config["provider_config"]["aws.other"] = config["provider_config"]["aws"].clone();
    config["provider_config"]["aws.other"]["alias"] = json!("other");
    config["provider_config"]["aws.other"]["expressions"]["assume_role"][0]["role_arn"]["constant_value"] =
        json!("arn:aws:iam::210987654321:role/Other");
    config["root_module"]["resources"][2]["provider_config_key"] = json!("aws.other");
    let arn = "arn:aws:ec2:us-east-1:210987654321:instance/i-0123456789abcdef0";
    v["resource_changes"][2]["change"]["after"]["arn"] = json!(arn);
    v["resource_changes"][0]["change"]["after"]["target"][0]["resource_arns"] = json!([arn]);
    v["resource_changes"][0]["change"]["after"]["experiment_options"] =
        json!([{"account_targeting":"multi-account"}]);
    v["resource_changes"][7]["change"]["after"]["account_id"] = json!("210987654321");
    let verifies = |v: &Value| {
        let plan = crate::plan::evidence::tests::parse_fixture(&v.to_string()).unwrap();
        crate::configured_service_links::verified(
            &crate::semantic::transform(&plan),
            "aws_fis_experiment_template.sample0",
            "target.resource_arns",
            super::label("aws_fis_experiment_template", "target.resource_arns"),
        )
    };
    assert!(verifies(&v));
    v["resource_changes"][7]["change"]["after"]["experiment_template_id"] = json!("unrelated");
    assert!(!verifies(&v));
}
