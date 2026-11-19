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
        serde_json::from_str(include_str!("../../fixtures/transit-multicast-plan.json")).unwrap();
    for (address, path) in [
        (
            "aws_ec2_transit_gateway_multicast_domain.sample0",
            "transit_gateway_id",
        ),
        (
            "aws_ec2_transit_gateway_multicast_domain_association.sample2",
            "transit_gateway_multicast_domain_id",
        ),
        (
            "aws_ec2_transit_gateway_multicast_domain_association.sample2",
            "transit_gateway_attachment_id",
        ),
        (
            "aws_ec2_transit_gateway_multicast_domain_association.sample2",
            "subnet_id",
        ),
        (
            "aws_ec2_transit_gateway_multicast_group_member.sample6",
            "transit_gateway_multicast_domain_id",
        ),
        (
            "aws_ec2_transit_gateway_multicast_group_member.sample6",
            "network_interface_id",
        ),
        (
            "aws_ec2_transit_gateway_multicast_group_source.sample7",
            "transit_gateway_multicast_domain_id",
        ),
        (
            "aws_ec2_transit_gateway_multicast_group_source.sample7",
            "network_interface_id",
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
    changed["resource_changes"][3]["change"]["after"]["subnet_ids"] = json!(["subnet-other"]);
    assert!(!verified(
        &changed,
        "aws_ec2_transit_gateway_multicast_domain_association.sample2",
        "transit_gateway_multicast_domain_id"
    ));
    let mut changed = fixture.clone();
    changed["resource_changes"][7]["change"]["after"]["group_ip_address"] = json!("192.0.2.1");
    assert!(!verified(
        &changed,
        "aws_ec2_transit_gateway_multicast_group_source.sample7",
        "network_interface_id"
    ));
    let mut changed = fixture.clone();
    changed["resource_changes"][1]["deposed"] = json!("old");
    assert!(!verified(
        &changed,
        "aws_ec2_transit_gateway_multicast_domain.sample0",
        "transit_gateway_id"
    ));
}
