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
        serde_json::from_str(include_str!("../../fixtures/local-gateways-plan.json")).unwrap();
    for (address, path) in [
        ("aws_ec2_carrier_gateway.sample0", "vpc_id"),
        (
            "aws_ec2_local_gateway_route_table.sample2",
            "local_gateway_id",
        ),
        (
            "aws_ec2_local_gateway_route_table_virtual_interface_group_association.sample4",
            "local_gateway_route_table_id",
        ),
        (
            "aws_ec2_local_gateway_route_table_virtual_interface_group_association.sample4",
            "local_gateway_virtual_interface_group_id",
        ),
        (
            "aws_ec2_local_gateway_route_table_vpc_association.sample6",
            "vpc_id",
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
    changed["resource_changes"][2]["change"]["after"]["local_gateway_route_table_id"] =
        json!("other-table");
    assert!(!verified(
        &changed,
        "aws_ec2_local_gateway_route_table_virtual_interface_group_association.sample4",
        "local_gateway_route_table_id"
    ));
    let mut changed = fixture.clone();
    changed["resource_changes"][1]["deposed"] = json!("old");
    assert!(!verified(
        &changed,
        "aws_ec2_carrier_gateway.sample0",
        "vpc_id"
    ));
}
