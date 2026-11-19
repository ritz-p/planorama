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
        serde_json::from_str(include_str!("../../fixtures/client-vpn-plan.json")).unwrap();
    for (address, path) in [
        ("aws_ec2_client_vpn_endpoint.sample0", "vpc_id"),
        (
            "aws_ec2_client_vpn_endpoint.sample0",
            "authentication_options.root_certificate_chain_arn",
        ),
        (
            "aws_ec2_client_vpn_network_association.sample3",
            "client_vpn_endpoint_id",
        ),
        (
            "aws_ec2_client_vpn_network_association.sample3",
            "subnet_id",
        ),
        ("aws_ec2_client_vpn_route.sample5", "client_vpn_endpoint_id"),
        ("aws_ec2_client_vpn_route.sample5", "target_vpc_subnet_id"),
        (
            "aws_ec2_client_vpn_authorization_rule.sample6",
            "client_vpn_endpoint_id",
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
    changed["resource_changes"][0]["change"]["after"]["authentication_options"][0]["type"] =
        json!("federated-authentication");
    assert!(!verified(
        &changed,
        "aws_ec2_client_vpn_endpoint.sample0",
        "authentication_options.root_certificate_chain_arn"
    ));
    let mut changed = fixture.clone();
    changed["resource_changes"][3]["change"]["after"]["subnet_id"] = json!("subnet-other");
    assert!(!verified(
        &changed,
        "aws_ec2_client_vpn_route.sample5",
        "target_vpc_subnet_id"
    ));
    let mut changed = fixture.clone();
    changed["resource_changes"][1]["deposed"] = json!("old");
    assert!(!verified(
        &changed,
        "aws_ec2_client_vpn_endpoint.sample0",
        "vpc_id"
    ));
}
