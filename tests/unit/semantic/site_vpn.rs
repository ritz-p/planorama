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
        serde_json::from_str(include_str!("../../fixtures/site-vpn-plan.json")).unwrap();
    for (address, path) in [
        ("aws_vpn_connection.sample0", "customer_gateway_id"),
        ("aws_vpn_connection.sample0", "vpn_gateway_id"),
        ("aws_vpn_gateway.sample2", "vpc_id"),
        ("aws_vpn_connection_route.sample4", "vpn_connection_id"),
        (
            "aws_vpn_connection.sample0",
            "tunnel1_log_options.cloudwatch_log_options.log_group_arn",
        ),
        ("aws_vpn_gateway_attachment.sample6", "vpn_gateway_id"),
        ("aws_vpn_gateway_attachment.sample6", "vpc_id"),
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
    changed["resource_changes"][0]["change"]["after"]["transit_gateway_id"] = json!("tgw-test");
    assert!(!verified(
        &changed,
        "aws_vpn_connection.sample0",
        "vpn_gateway_id"
    ));
    let mut changed = fixture.clone();
    changed["resource_changes"][0]["change"]["after"]["static_routes_only"] = json!(false);
    assert!(!verified(
        &changed,
        "aws_vpn_connection_route.sample4",
        "vpn_connection_id"
    ));
    let mut changed = fixture.clone();
    changed["resource_changes"][1]["deposed"] = json!("old");
    assert!(!verified(
        &changed,
        "aws_vpn_connection.sample0",
        "customer_gateway_id"
    ));
}
