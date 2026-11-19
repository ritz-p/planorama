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
    let fixture: Value =
        serde_json::from_str(include_str!("../../fixtures/arc-configuration-plan.json")).unwrap();
    for (address, path) in [
        ("aws_arcregionswitch_plan.sample0", "execution_role"),
        (
            "aws_arcregionswitch_plan.sample0",
            "associated_alarms.resource_identifier",
        ),
        (
            "aws_arcregionswitch_plan.sample0",
            "associated_alarms.cross_account_role",
        ),
        (
            "aws_arcregionswitch_plan.sample0",
            "workflow.step.arc_routing_control_config.region_and_routing_controls.routing_control.routing_control_arn",
        ),
        (
            "aws_arcregionswitch_plan.sample0",
            "workflow.step.custom_action_lambda_config.lambda.arn",
        ),
        (
            "aws_arczonalshift_zonal_autoshift_configuration.sample5",
            "resource_arn",
        ),
        (
            "aws_arczonalshift_zonal_autoshift_configuration.sample5",
            "blocking_alarms.alarm_identifier",
        ),
        (
            "aws_arczonalshift_zonal_autoshift_configuration.sample5",
            "outcome_alarms.alarm_identifier",
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
        entry["change"]["after_sensitive"] = json!(true);
        assert!(
            !verified(&masked, address, path),
            "{address}/{path} sensitive"
        );
        let entry = masked["resource_changes"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|entry| entry["address"] == address)
            .unwrap();
        entry["change"]["after_sensitive"] = json!(false);
        entry["change"]["after_unknown"] = json!(true);
        assert!(!verified(&masked, address, path), "{address}/{path}");
    }
    let mut changed = fixture.clone();
    changed["resource_changes"][1]["change"]["actions"] = json!(["delete"]);
    assert!(!verified(
        &changed,
        "aws_arcregionswitch_plan.sample0",
        "execution_role"
    ));
    let mut changed = fixture.clone();
    changed["resource_changes"][1]["change"]["after_sensitive"] = json!(true);
    assert!(!verified(
        &changed,
        "aws_arcregionswitch_plan.sample0",
        "execution_role"
    ));
    let mut changed = fixture.clone();
    changed["resource_changes"][1]["change"]["after"]["arn"] = json!("different");
    assert!(!verified(
        &changed,
        "aws_arcregionswitch_plan.sample0",
        "execution_role"
    ));
    let mut changed = fixture.clone();
    changed["resource_changes"][1]["deposed"] = json!("old");
    assert!(!verified(
        &changed,
        "aws_arcregionswitch_plan.sample0",
        "execution_role"
    ));
}
