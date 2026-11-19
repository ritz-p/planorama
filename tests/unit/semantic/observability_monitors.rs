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
        "../../fixtures/observability-monitors-plan.json"
    ))
    .unwrap();
    for (address, path) in [
        (
            "aws_applicationinsights_application.sample1",
            "resource_group_name",
        ),
        ("aws_evidently_feature.sample3", "project"),
        ("aws_internetmonitor_monitor.sample5", "resources"),
        ("aws_internetmonitor_monitor.sample7", "resources"),
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
    changed["resource_changes"][1]["change"]["after"]["grouping_type"] = json!("ACCOUNT_BASED");
    assert!(!verified(
        &changed,
        "aws_applicationinsights_application.sample1",
        "resource_group_name"
    ));
}

#[test]
fn launch_feature_accepts_project_name_and_arn_proof_for_one_project() {
    let mut input: Value = serde_json::from_str(include_str!(
        "../../fixtures/observability-monitors-plan.json"
    ))
    .unwrap();
    let project_arn = "arn:aws:evidently:us-east-1:123456789012:project/fixture-project";
    input["resource_changes"][2]["change"]["after"]["arn"] = json!(project_arn);
    input["resource_changes"].as_array_mut().unwrap().push(json!({
        "address":"aws_evidently_launch.test","mode":"managed","type":"aws_evidently_launch","name":"test",
        "provider_name":"registry.terraform.io/hashicorp/aws",
        "change":{"actions":["create"],"after":{"name":"test","project":project_arn,"groups":[{"feature":"fixture-feature"}]}}
    }));
    let resources = input["configuration"]["root_module"]["resources"]
        .as_array_mut()
        .unwrap();
    resources.push(json!({
        "address":"aws_evidently_launch.test","mode":"managed","type":"aws_evidently_launch","name":"test",
        "provider_config_key":"aws","expressions":{"project":{"references":["aws_evidently_project.sample2.arn"]},
        "groups":[{"feature":{"references":["aws_evidently_feature.sample3.name"]}}]}
    }));
    assert!(verified(
        &input,
        "aws_evidently_launch.test",
        "groups.feature"
    ));
    input["resource_changes"][3]["change"]["after"]["project"] = json!("other-project");
    assert!(!verified(
        &input,
        "aws_evidently_launch.test",
        "groups.feature"
    ));
}
