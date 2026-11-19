use crate::{plan, semantic};
use serde_json::{Value, json};

fn verified(value: &Value, address: &str, path: &str) -> bool {
    let parsed = plan::parse(&value.to_string()).unwrap();
    let graph = semantic::transform(&parsed);
    let node = graph
        .nodes
        .iter()
        .find(|node| node.address == address)
        .unwrap();
    node.references.iter().any(|reference| {
        reference.attribute == path
            && reference.complete
            && !reference.dependency
            && !reference.targets.is_empty()
            && super::label(&node.resource_type, path) == Some(reference.label)
    })
}

#[test]
fn configured_links_require_current_typed_evidence() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../fixtures/recovery-configuration-plan.json"
    ))
    .unwrap();
    for (address, path) in [
        ("aws_dlm_lifecycle_policy.sample0", "execution_role_arn"),
        (
            "aws_dlm_lifecycle_policy.sample0",
            "policy_details.schedule.cross_region_copy_rule.cmk_arn",
        ),
        (
            "aws_dlm_lifecycle_policy.sample0",
            "policy_details.schedule.create_rule.scripts.execution_handler",
        ),
        (
            "aws_drs_replication_configuration_template.sample1",
            "staging_area_subnet_id",
        ),
        (
            "aws_drs_replication_configuration_template.sample1",
            "replication_servers_security_groups_ids",
        ),
        (
            "aws_drs_replication_configuration_template.sample1",
            "ebs_encryption_key_arn",
        ),
        (
            "aws_dlm_lifecycle_policy.sample8",
            "policy_details.action.cross_region_copy.encryption_configuration.cmk_arn",
        ),
    ] {
        assert!(verified(&fixture, address, path), "{address}/{path}");
        let mut masked = fixture.clone();
        let entry = masked["resource_changes"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|entry| entry["address"] == address)
            .unwrap();
        entry["change"]["after_unknown"] = json!(true);
        assert!(!verified(&masked, address, path), "{address}/{path}");
    }
    let mut changed = fixture.clone();
    changed["resource_changes"][0]["change"]["after"]["policy_details"][0]["schedule"][0]["cross_region_copy_rule"]
        [0]["target"] = json!("us-west-2");
    assert!(!verified(
        &changed,
        "aws_dlm_lifecycle_policy.sample0",
        "policy_details.schedule.cross_region_copy_rule.cmk_arn"
    ));
    let mut changed = fixture.clone();
    changed["resource_changes"][1]["change"]["after"]["ebs_encryption"] = json!("DEFAULT");
    assert!(!verified(
        &changed,
        "aws_drs_replication_configuration_template.sample1",
        "ebs_encryption_key_arn"
    ));
    let mut changed = fixture.clone();
    changed["resource_changes"][1]["change"]["after"]["replication_servers_security_groups_ids"] =
        json!(["sg-fixture", "sg-other"]);
    assert!(!verified(
        &changed,
        "aws_drs_replication_configuration_template.sample1",
        "replication_servers_security_groups_ids"
    ));
    let mut changed = fixture.clone();
    changed["resource_changes"][2]["deposed"] = json!("old");
    assert!(!verified(
        &changed,
        "aws_dlm_lifecycle_policy.sample0",
        "execution_role_arn"
    ));
}

#[test]
fn cross_region_copy_requires_verified_destination_account() {
    let mut fixture: Value = serde_json::from_str(include_str!(
        "../../fixtures/recovery-configuration-plan.json"
    ))
    .unwrap();
    let aws = &mut fixture["configuration"]["provider_config"];
    aws["aws"]["expressions"]["assume_role"] =
        json!([{"role_arn":{"constant_value":"arn:aws:iam::123456789012:role/Fixture"}}]);
    let mut destination = aws["aws"].clone();
    destination["alias"] = json!("destination");
    destination["expressions"]["region"] = json!({"constant_value":"us-west-2"});
    aws["aws.destination"] = destination;
    fixture["configuration"]["root_module"]["resources"][3]["provider_config_key"] =
        json!("aws.destination");
    let arn = json!("arn:aws:kms:us-west-2:123456789012:key/destination");
    let copy = &mut fixture["resource_changes"][0]["change"]["after"]["policy_details"][0]["schedule"]
        [0]["cross_region_copy_rule"][0];
    copy["target"] = json!("us-west-2");
    copy["cmk_arn"] = arn.clone();
    fixture["resource_changes"][3]["change"]["after"]["arn"] = arn;
    let path = "policy_details.schedule.cross_region_copy_rule.cmk_arn";
    let address = "aws_dlm_lifecycle_policy.sample0";
    assert!(!verified(&fixture, address, path));
    let verified_destination = |fixture: &Value| {
        let plan = plan::evidence::tests::parse_fixture(&fixture.to_string()).unwrap();
        let graph = semantic::transform(&plan);
        let policy = graph
            .nodes
            .iter()
            .find(|node| node.address == address)
            .unwrap();
        policy.references.iter().any(|reference| {
            reference.attribute == path
                && reference.complete
                && !reference.dependency
                && !reference.targets.is_empty()
                && super::label(&policy.resource_type, path) == Some(reference.label)
        })
    };
    assert!(verified_destination(&fixture));
    fixture["configuration"]["provider_config"]["aws.destination"]["expressions"]["assume_role"]
        [0]["role_arn"]["constant_value"] = json!("arn:aws:iam::234567890123:role/Fixture");
    assert!(!verified_destination(&fixture));
}
