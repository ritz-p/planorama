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
        serde_json::from_str(include_str!("../../fixtures/transit-attachments-plan.json")).unwrap();
    for (address, path) in [
        (
            "aws_ec2_transit_gateway_vpc_attachment.sample0",
            "transit_gateway_id",
        ),
        ("aws_ec2_transit_gateway_vpc_attachment.sample0", "vpc_id"),
        (
            "aws_ec2_transit_gateway_vpc_attachment.sample0",
            "subnet_ids",
        ),
        (
            "aws_ec2_transit_gateway_connect.sample4",
            "transit_gateway_id",
        ),
        (
            "aws_ec2_transit_gateway_connect.sample4",
            "transport_attachment_id",
        ),
        (
            "aws_ec2_transit_gateway_connect_peer.sample5",
            "transit_gateway_attachment_id",
        ),
        (
            "aws_ec2_transit_gateway_peering_attachment.sample6",
            "transit_gateway_id",
        ),
        (
            "aws_ec2_transit_gateway_peering_attachment.sample6",
            "peer_transit_gateway_id",
        ),
        (
            "aws_ec2_transit_gateway_peering_attachment_accepter.sample8",
            "transit_gateway_attachment_id",
        ),
        (
            "aws_ec2_transit_gateway_vpc_attachment_accepter.sample9",
            "transit_gateway_attachment_id",
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
    changed["resource_changes"][3]["change"]["after"]["vpc_id"] = json!("vpc-other");
    assert!(!verified(
        &changed,
        "aws_ec2_transit_gateway_vpc_attachment.sample0",
        "subnet_ids"
    ));
    let mut changed = fixture.clone();
    changed["resource_changes"][0]["change"]["after"]["transit_gateway_id"] = json!("tgw-other");
    assert!(!verified(
        &changed,
        "aws_ec2_transit_gateway_connect.sample4",
        "transport_attachment_id"
    ));
    let mut changed = fixture.clone();
    changed["resource_changes"][6]["change"]["after"]["peer_region"] = json!("eu-west-1");
    assert!(!verified(
        &changed,
        "aws_ec2_transit_gateway_peering_attachment.sample6",
        "peer_transit_gateway_id"
    ));
    let mut changed = fixture.clone();
    changed["resource_changes"][1]["deposed"] = json!("old");
    assert!(!verified(
        &changed,
        "aws_ec2_transit_gateway_vpc_attachment.sample0",
        "transit_gateway_id"
    ));
}

#[test]
fn peering_requires_the_selected_peer_account_and_accepting_provider() {
    let mut f: Value =
        serde_json::from_str(include_str!("../../fixtures/transit-attachments-plan.json")).unwrap();
    let providers = &mut f["configuration"]["provider_config"];
    providers["aws"]["expressions"]["assume_role"] =
        json!([{"role_arn":{"constant_value":"arn:aws:iam::111111111111:role/Fixture"}}]);
    let mut peer = providers["aws"].clone();
    peer["alias"] = json!("peer");
    peer["expressions"]["region"] = json!({"constant_value":"eu-west-1"});
    peer["expressions"]["assume_role"][0]["role_arn"]["constant_value"] =
        json!("arn:aws:iam::222222222222:role/Peer");
    providers["aws.peer"] = peer;
    for i in [7, 8] {
        f["configuration"]["root_module"]["resources"][i]["provider_config_key"] =
            json!("aws.peer");
    }
    f["resource_changes"][6]["change"]["after"]["peer_region"] = json!("eu-west-1");
    f["resource_changes"][6]["change"]["after"]["peer_account_id"] = json!("222222222222");
    let linked = |f: &Value, address: &str, label: &str| {
        let plan = plan::evidence::tests::parse_fixture(&f.to_string()).unwrap();
        let graph = semantic::transform(&plan);
        graph
            .nodes
            .iter()
            .find(|n| n.address == address)
            .unwrap()
            .references
            .iter()
            .any(|r| r.complete && !r.dependency && r.label == label)
    };
    let request = "aws_ec2_transit_gateway_peering_attachment.sample6";
    let accepter = "aws_ec2_transit_gateway_peering_attachment_accepter.sample8";
    assert!(linked(&f, request, "Peer transit gateway"));
    assert!(linked(&f, accepter, "Peering request"));
    f["resource_changes"][6]["change"]["after"]["peer_account_id"] = json!("333333333333");
    assert!(!linked(&f, request, "Peer transit gateway"));
    assert!(!linked(&f, accepter, "Peering request"));
}
