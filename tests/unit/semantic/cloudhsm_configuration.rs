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
        "../../fixtures/cloudhsm-configuration-plan.json"
    ))
    .unwrap();
    for (address, path) in [
        ("aws_cloudhsm_v2_cluster.sample1", "subnet_ids"),
        ("aws_cloudhsm_v2_hsm.sample3", "cluster_id"),
        ("aws_cloudhsm_v2_hsm.sample3", "subnet_id"),
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
    changed["configuration"]["root_module"]["resources"][2]["expressions"]["vpc_id"] =
        json!({"constant_value":"vpc-fixture"});
    assert!(!verified(
        &changed,
        "aws_cloudhsm_v2_cluster.sample1",
        "subnet_ids"
    ));
    let mut changed = fixture.clone();
    changed["resource_changes"][1]["change"]["after"]["subnet_ids"] =
        json!(["subnet-fixture", "subnet-unproven"]);
    assert!(!verified(
        &changed,
        "aws_cloudhsm_v2_hsm.sample3",
        "subnet_id"
    ));
}

#[test]
fn placement_requires_explicit_subnet_vpc_evidence() {
    use crate::model::EdgeKind;
    let mut fixture: Value = serde_json::from_str(include_str!(
        "../../fixtures/cloudhsm-configuration-plan.json"
    ))
    .unwrap();
    let placed = |value: &Value| {
        let parsed = plan::parse(&value.to_string()).unwrap();
        let graph = semantic::transform(&parsed);
        assert_eq!(graph.coverage.selected.resources, 4);
        graph.edges.iter().any(|edge| {
            edge.kind == EdgeKind::Containment
                && graph.nodes[edge.from].address == "aws_vpc.sample0"
                && graph.nodes[edge.to].address == "aws_cloudhsm_v2_cluster.sample1"
        })
    };
    assert!(placed(&fixture));
    fixture["resource_changes"][1]["change"]["after"]["vpc_id"] = json!("vpc-fixture");
    fixture["configuration"]["root_module"]["resources"][2]["expressions"]["vpc_id"] =
        json!({"constant_value": "vpc-fixture"});
    assert!(!placed(&fixture));
}
