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
        serde_json::from_str(include_str!("../../fixtures/transit-policies-plan.json")).unwrap();
    for (address, path) in [
        (
            "aws_ec2_transit_gateway_metering_policy.sample0",
            "transit_gateway_id",
        ),
        (
            "aws_ec2_transit_gateway_metering_policy.sample0",
            "middlebox_attachment_ids",
        ),
        (
            "aws_ec2_transit_gateway_metering_policy_entry.sample3",
            "transit_gateway_metering_policy_id",
        ),
        (
            "aws_ec2_transit_gateway_metering_policy_entry.sample3",
            "source_transit_gateway_attachment_id",
        ),
        (
            "aws_ec2_transit_gateway_policy_table.sample4",
            "transit_gateway_id",
        ),
        (
            "aws_ec2_transit_gateway_policy_table_entry.sample5",
            "transit_gateway_policy_table_id",
        ),
        (
            "aws_ec2_transit_gateway_policy_table_entry.sample5",
            "target_route_table_id",
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
    changed["resource_changes"][3]["change"]["after"]["source_transit_gateway_attachment_type"] =
        json!("vpn");
    assert!(!verified(
        &changed,
        "aws_ec2_transit_gateway_metering_policy_entry.sample3",
        "source_transit_gateway_attachment_id"
    ));
    let mut changed = fixture.clone();
    changed["resource_changes"][6]["change"]["after"]["transit_gateway_id"] = json!("other");
    assert!(!verified(
        &changed,
        "aws_ec2_transit_gateway_policy_table_entry.sample5",
        "target_route_table_id"
    ));
    let mut changed = fixture.clone();
    changed["resource_changes"][1]["deposed"] = json!("old");
    assert!(!verified(
        &changed,
        "aws_ec2_transit_gateway_metering_policy.sample0",
        "transit_gateway_id"
    ));
}
