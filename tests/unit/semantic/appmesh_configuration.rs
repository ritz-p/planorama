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
        "../../fixtures/appmesh-configuration-plan.json"
    ))
    .unwrap();
    for (address, path) in [
        ("aws_appmesh_route.sample1", "mesh_name"),
        ("aws_appmesh_route.sample1", "virtual_router_name"),
        (
            "aws_appmesh_route.sample1",
            "spec.http_route.action.weighted_target.virtual_node",
        ),
        (
            "aws_appmesh_virtual_service.sample4",
            "spec.provider.virtual_node.virtual_node_name",
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
    changed["resource_changes"][3]["change"]["after"]["mesh_name"] = json!("other-mesh");
    assert!(!verified(
        &changed,
        "aws_appmesh_route.sample1",
        "spec.http_route.action.weighted_target.virtual_node"
    ));
    let mut changed = fixture.clone();
    changed["resource_changes"][1]["change"]["after"]["mesh_owner"] = json!("987654321098");
    assert!(!verified(
        &changed,
        "aws_appmesh_route.sample1",
        "mesh_name"
    ));
    let mut changed = fixture.clone();
    changed["resource_changes"][4]["change"]["after"]["spec"][0]["provider"][0]["virtual_router"] =
        json!([{"virtual_router_name":"router"}]);
    assert!(!verified(
        &changed,
        "aws_appmesh_virtual_service.sample4",
        "spec.provider.virtual_node.virtual_node_name"
    ));
}

#[test]
fn member_owner_preserves_known_provider_account() {
    let parsed = plan::parse(include_str!(
        "../../fixtures/appmesh-configuration-plan.json"
    ))
    .unwrap();
    let graph = semantic::transform(&parsed);
    let node = graph
        .nodes
        .iter()
        .find(|node| node.address == "aws_appmesh_route.sample1")
        .unwrap();
    let owner = node.member_owner.as_ref().unwrap();
    assert_eq!(owner.target.address, "aws_appmesh_mesh.sample0");
    assert_eq!(owner.section, "Mesh configuration");
    assert_eq!(
        node.entity
            .scope
            .as_ref()
            .and_then(|scope| scope.account.as_ref())
            .map(|account| account.as_str()),
        Some("123456789012")
    );
}
