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

#[test]
fn organization_targets_can_be_managed_in_a_separate_account() {
    crate::configured_service_links::organization_targets(
        include_str!("../../fixtures/inspector-assessments-plan.json"),
        super::label,
        &[
            (
                "aws_inspector2_delegated_admin_account.sample4",
                "account_id",
            ),
            ("aws_inspector2_member_association.sample6", "account_id"),
            ("aws_inspector2_enabler.sample7", "account_ids"),
        ],
    );
}

#[test]
fn unordered_subscriptions_match_each_referenced_topic() {
    use serde_json::{Value, json};
    let mut v: Value = serde_json::from_str(include_str!(
        "../../fixtures/inspector-assessments-plan.json"
    ))
    .unwrap();
    let config = &mut v["configuration"]["root_module"]["resources"];
    let mut topic = config[3].clone();
    topic["address"] = json!("aws_sns_topic.second");
    topic["name"] = json!("second");
    config.as_array_mut().unwrap().push(topic);
    config[2]["expressions"]["event_subscription"]
        .as_array_mut()
        .unwrap()
        .push(json!({"topic_arn":{"references":["aws_sns_topic.second.arn"]}}));
    let changes = &mut v["resource_changes"];
    let mut topic = changes[3].clone();
    topic["address"] = json!("aws_sns_topic.second");
    topic["name"] = json!("second");
    topic["change"]["after"]["arn"] = json!("arn:aws:sns:us-east-1:123456789012:second");
    changes.as_array_mut().unwrap().push(topic);
    changes[2]["change"]["after"]["event_subscription"]
        .as_array_mut()
        .unwrap()
        .insert(
            0,
            json!({"topic_arn":"arn:aws:sns:us-east-1:123456789012:second"}),
        );
    let check = |v: &Value| {
        let parsed = crate::plan::parse(&v.to_string()).unwrap();
        crate::configured_service_links::verified(
            &crate::semantic::transform(&parsed),
            "aws_inspector_assessment_template.sample2",
            "event_subscription.topic_arn",
            Some("Assessment notifications"),
        )
    };
    assert!(check(&v));
    v["resource_changes"][2]["change"]["after"]["event_subscription"][1]["topic_arn"] =
        json!("arn:aws:sns:us-east-1:123456789012:second");
    assert!(!check(&v));
}
