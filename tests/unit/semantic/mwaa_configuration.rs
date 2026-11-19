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
        serde_json::from_str(include_str!("../../fixtures/mwaa-configuration-plan.json")).unwrap();
    for (address, path) in [
        (
            "aws_mwaa_environment.sample0",
            "network_configuration.subnet_ids",
        ),
        (
            "aws_mwaa_environment.sample0",
            "network_configuration.security_group_ids",
        ),
        ("aws_mwaa_environment.sample0", "execution_role_arn"),
        ("aws_mwaa_environment.sample0", "source_bucket_arn"),
        ("aws_mwaa_environment.sample0", "kms_key"),
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
    changed["resource_changes"][0]["change"]["after"]["network_configuration"][0]["security_group_ids"] =
        json!(["sg-fixture", "sg-unproven"]);
    assert!(!verified(
        &changed,
        "aws_mwaa_environment.sample0",
        "network_configuration.subnet_ids"
    ));
    let mut changed = fixture.clone();
    changed["resource_changes"][0]["change"]["after"]["kms_key"] = json!("alias/aws/airflow");
    assert!(!verified(
        &changed,
        "aws_mwaa_environment.sample0",
        "kms_key"
    ));
    let mut changed = fixture.clone();
    changed["resource_changes"][1]["deposed"] = json!("old");
    assert!(!verified(
        &changed,
        "aws_mwaa_environment.sample0",
        "network_configuration.subnet_ids"
    ));
}

#[test]
fn vpc_placement_requires_both_configured_collections() {
    use crate::model::EdgeKind;
    let mut fixture: Value =
        serde_json::from_str(include_str!("../../fixtures/mwaa-configuration-plan.json")).unwrap();
    let has_vpc = |fixture: &Value| {
        let graph = semantic::transform(&plan::parse(&fixture.to_string()).unwrap());
        graph.edges.iter().any(|edge| {
            edge.kind == EdgeKind::Containment
                && graph.nodes[edge.from].address == "aws_vpc.sample3"
                && graph.nodes[edge.to].address == "aws_mwaa_environment.sample0"
        })
    };
    assert!(has_vpc(&fixture));
    fixture["resource_changes"][2]["change"]["after"]["vpc_id"] = json!("vpc-other");
    assert!(!has_vpc(&fixture));
}
