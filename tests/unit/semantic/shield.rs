#[test]
fn configured_links_require_current_typed_evidence() {
    crate::configured_service_links::check(
        include_str!("../../fixtures/shield-plan.json"),
        crate::plan::parse,
        super::label,
        &[
            (
                "aws_shield_application_layer_automatic_response.sample0",
                "resource_arn",
            ),
            ("aws_shield_protection.sample2", "resource_arn"),
            ("aws_shield_protection_group.sample3", "members"),
            (
                "aws_shield_drt_access_log_bucket_association.sample4",
                "log_bucket",
            ),
            (
                "aws_shield_drt_access_log_bucket_association.sample4",
                "role_arn_association_id",
            ),
            (
                "aws_shield_drt_access_role_arn_association.sample6",
                "role_arn",
            ),
            (
                "aws_shield_protection_health_check_association.sample8",
                "health_check_arn",
            ),
            (
                "aws_shield_protection_health_check_association.sample8",
                "shield_protection_id",
            ),
        ],
        ("aws_lb.sample1", "arn"),
    );
}

#[test]
fn group_selectors_cannot_enumerate_members() {
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("../../fixtures/shield-plan.json")).unwrap();
    for pattern in ["ALL", "BY_RESOURCE_TYPE"] {
        let mut plan = fixture.clone();
        let group = plan["resource_changes"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|r| r["type"] == "aws_shield_protection_group")
            .unwrap();
        group["change"]["after"]["pattern"] = serde_json::json!(pattern);
        let address = group["address"].as_str().unwrap().to_owned();
        let parsed = crate::plan::parse(&plan.to_string()).unwrap();
        assert!(!crate::configured_service_links::verified(
            &crate::semantic::transform(&parsed),
            &address,
            "members",
            Some("Protected group members")
        ));
    }
}
