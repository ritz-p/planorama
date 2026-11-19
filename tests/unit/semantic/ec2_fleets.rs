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
        serde_json::from_str(include_str!("../../fixtures/ec2-fleets-plan.json")).unwrap();
    for (address, path) in [
        (
            "aws_ec2_fleet.sample0",
            "launch_template_config.launch_template_specification.launch_template_id",
        ),
        (
            "aws_ec2_fleet.sample0",
            "launch_template_config.override.subnet_id",
        ),
        ("aws_spot_fleet_request.sample3", "launch_specification.ami"),
        ("aws_spot_fleet_request.sample3", "iam_fleet_role"),
        ("aws_spot_instance_request.sample5", "launch_template.name"),
        ("aws_spot_datafeed_subscription.sample7", "bucket"),
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
    changed["resource_changes"][0]["change"]["after"]["launch_template_config"][0]["launch_template_specification"]
        [0]["launch_template_name"] = json!("other");
    assert!(!verified(
        &changed,
        "aws_ec2_fleet.sample0",
        "launch_template_config.launch_template_specification.launch_template_id"
    ));
    let mut changed = fixture.clone();
    changed["resource_changes"][3]["change"]["after"]["launch_template_config"] = json!([{}]);
    assert!(!verified(
        &changed,
        "aws_spot_fleet_request.sample3",
        "launch_specification.ami"
    ));
    let mut changed = fixture.clone();
    changed["resource_changes"][1]["deposed"] = json!("old");
    assert!(!verified(
        &changed,
        "aws_ec2_fleet.sample0",
        "launch_template_config.launch_template_specification.launch_template_id"
    ));
}
