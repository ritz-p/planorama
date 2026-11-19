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
