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
        "../../fixtures/network-insights-configuration-plan.json"
    ))
    .unwrap();
    for (address, path) in [
        ("aws_ec2_network_insights_path.sample1", "source"),
        (
            "aws_ec2_network_insights_analysis.sample2",
            "network_insights_path_id",
        ),
        (
            "aws_ec2_network_insights_access_scope.sample3",
            "match_paths.source.resource_statement.resources",
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
    changed["resource_changes"][1]["change"]["after"]["source"] = json!("192.0.2.5");
    assert!(!verified(
        &changed,
        "aws_ec2_network_insights_path.sample1",
        "source"
    ));
    let mut changed = fixture.clone();
    changed["resource_changes"][3]["change"]["after"]["match_paths"][0]["source"][0]["resource_statement"]
        [0]["resource_types"] = json!(["AWS::EC2::InternetGateway"]);
    assert!(!verified(
        &changed,
        "aws_ec2_network_insights_access_scope.sample3",
        "match_paths.source.resource_statement.resources"
    ));
}
