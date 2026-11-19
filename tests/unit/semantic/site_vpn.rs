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

#[test]
fn standalone_attachment_proves_route_propagation_with_a_refreshed_gateway() {
    let mut f: Value =
        serde_json::from_str(include_str!("../../fixtures/site-vpn-plan.json")).unwrap();
    f["configuration"]["root_module"]["resources"][2]["expressions"]
        .as_object_mut()
        .unwrap()
        .remove("vpc_id");
    for (kind, name, values, expressions) in [
        (
            "aws_route_table",
            "rt",
            json!({"id":"rtb-test","vpc_id":"vpc-test"}),
            json!({"vpc_id":{"references":["aws_vpc.sample3.id"]}}),
        ),
        (
            "aws_vpn_gateway_route_propagation",
            "prop",
            json!({"vpn_gateway_id":"vgw-test","route_table_id":"rtb-test"}),
            json!({"vpn_gateway_id":{"references":["aws_vpn_gateway.sample2.id"]},"route_table_id":{"references":["aws_route_table.rt.id"]}}),
        ),
    ] {
        f["resource_changes"].as_array_mut().unwrap().push(json!({"address":format!("{kind}.{name}"),"type":kind,"mode":"managed","name":name,"provider_name":"registry.terraform.io/hashicorp/aws","change":{"actions":["create"],"after":values}}));
        f["configuration"]["root_module"]["resources"].as_array_mut().unwrap().push(json!({"address":format!("{kind}.{name}"),"type":kind,"name":name,"mode":"managed","provider_config_key":"aws","expressions":expressions}));
    }
    for path in ["route_table_id", "vpn_gateway_id"] {
        assert!(verified(&f, "aws_vpn_gateway_route_propagation.prop", path));
    }
    f["resource_changes"][2]["change"]["after"]["vpc_id"] = json!("vpc-other");
    assert!(!verified(
        &f,
        "aws_vpn_gateway_route_propagation.prop",
        "route_table_id"
    ));
}
