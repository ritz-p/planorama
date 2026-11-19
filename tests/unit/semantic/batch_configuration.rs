use crate::{plan, semantic};
use serde_json::{Value, json};

#[test]
fn legacy_image_requires_each_override_to_be_known_absent() {
    let mut fixture: Value =
        serde_json::from_str(include_str!("../../fixtures/batch-configuration-plan.json")).unwrap();
    let change = &mut fixture["resource_changes"][8]["change"];
    change["after"]["compute_resources"][0]["ec2_configuration"] =
        json!([{"image_id_override": null}]);
    change["after_unknown"] = json!({"compute_resources": [{"ec2_configuration": [{}]}]});
    let address = "aws_batch_compute_environment.sample8";
    let path = "compute_resources.image_id";
    assert!(verified(&fixture, address, path));

    let change = &mut fixture["resource_changes"][8]["change"];
    change["after_unknown"]["compute_resources"][0]["ec2_configuration"][0]["image_id_override"] =
        json!(true);
    assert!(!verified(&fixture, address, path));

    let change = &mut fixture["resource_changes"][8]["change"];
    change["after_unknown"]["compute_resources"][0]["ec2_configuration"][0] = json!({});
    change["after"]["compute_resources"][0]["ec2_configuration"][0]["image_id_override"] =
        json!("ami-override");
    assert!(!verified(&fixture, address, path));
}

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
        serde_json::from_str(include_str!("../../fixtures/batch-configuration-plan.json")).unwrap();
    for (address, path) in [
        ("aws_batch_compute_environment.sample0", "service_role"),
        (
            "aws_batch_compute_environment.sample0",
            "eks_configuration.eks_cluster_arn",
        ),
        (
            "aws_batch_compute_environment.sample0",
            "compute_resources.instance_role",
        ),
        (
            "aws_batch_compute_environment.sample0",
            "compute_resources.spot_iam_fleet_role",
        ),
        (
            "aws_batch_compute_environment.sample0",
            "compute_resources.launch_template.launch_template_id",
        ),
        (
            "aws_batch_compute_environment.sample0",
            "compute_resources.ec2_configuration.image_id_override",
        ),
        ("aws_batch_compute_environment.sample7", "service_role"),
        (
            "aws_batch_compute_environment.sample7",
            "eks_configuration.eks_cluster_arn",
        ),
        (
            "aws_batch_compute_environment.sample8",
            "compute_resources.launch_template.launch_template_name",
        ),
        (
            "aws_batch_compute_environment.sample8",
            "compute_resources.image_id",
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
    changed["resource_changes"][0]["change"]["after"]["compute_resources"][0]["type"] =
        json!("FARGATE");
    assert!(!verified(
        &changed,
        "aws_batch_compute_environment.sample0",
        "compute_resources.instance_role"
    ));
    let mut changed = fixture.clone();
    changed["resource_changes"][0]["change"]["after"]["compute_resources"][0]["type"] =
        json!("EC2");
    assert!(!verified(
        &changed,
        "aws_batch_compute_environment.sample0",
        "compute_resources.spot_iam_fleet_role"
    ));
    let mut changed = fixture.clone();
    changed["resource_changes"][1]["deposed"] = json!("old");
    assert!(!verified(
        &changed,
        "aws_batch_compute_environment.sample0",
        "service_role"
    ));
    let mut changed = fixture.clone();
    changed["resource_changes"][8]["change"]["after"]["compute_resources"][0]["launch_template"]
        [0]["launch_template_id"] = json!("lt-other");
    assert!(!verified(
        &changed,
        "aws_batch_compute_environment.sample8",
        "compute_resources.launch_template.launch_template_name"
    ));
}
