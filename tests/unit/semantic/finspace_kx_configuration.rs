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
        "../../fixtures/finspace-kx-configuration-plan.json"
    ))
    .unwrap();
    for (address, path) in [
        ("aws_finspace_kx_environment.sample0", "kms_key_id"),
        (
            "aws_finspace_kx_environment.sample0",
            "transit_gateway_configuration.transit_gateway_id",
        ),
        ("aws_finspace_kx_cluster.sample1", "environment_id"),
        ("aws_finspace_kx_database.sample2", "environment_id"),
        ("aws_finspace_kx_dataview.sample3", "environment_id"),
        ("aws_finspace_kx_scaling_group.sample4", "environment_id"),
        ("aws_finspace_kx_user.sample5", "environment_id"),
        ("aws_finspace_kx_volume.sample6", "environment_id"),
        ("aws_finspace_kx_volume.sample7", "environment_id"),
        (
            "aws_finspace_kx_cluster.sample1",
            "vpc_configuration.subnet_ids",
        ),
        (
            "aws_finspace_kx_cluster.sample1",
            "vpc_configuration.security_group_ids",
        ),
        (
            "aws_finspace_kx_cluster.sample1",
            "vpc_configuration.vpc_id",
        ),
        ("aws_finspace_kx_cluster.sample1", "execution_role"),
        ("aws_finspace_kx_user.sample5", "iam_role"),
        ("aws_finspace_kx_cluster.sample1", "database.database_name"),
        ("aws_finspace_kx_cluster.sample1", "database.dataview_name"),
        ("aws_finspace_kx_dataview.sample3", "database_name"),
        (
            "aws_finspace_kx_cluster.sample1",
            "scaling_group_configuration.scaling_group_name",
        ),
        (
            "aws_finspace_kx_cluster.sample1",
            "savedown_storage_configuration.volume_name",
        ),
        (
            "aws_finspace_kx_cluster.sample1",
            "tickerplant_log_configuration.tickerplant_log_volumes",
        ),
        (
            "aws_finspace_kx_dataview.sample3",
            "segment_configurations.volume_name",
        ),
        ("aws_finspace_kx_cluster.sample1", "code.s3_bucket"),
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
    changed["resource_changes"][1]["change"]["after"]["vpc_configuration"][0]["vpc_id"] =
        json!("vpc-other");
    assert!(!verified(
        &changed,
        "aws_finspace_kx_cluster.sample1",
        "vpc_configuration.subnet_ids"
    ));
    let mut changed = fixture.clone();
    changed["resource_changes"][3]["change"]["after"]["database_name"] = json!("other-db");
    assert!(!verified(
        &changed,
        "aws_finspace_kx_cluster.sample1",
        "database.dataview_name"
    ));
    let mut changed = fixture.clone();
    changed["resource_changes"][7]["change"]["after"]["availability_zones"] = json!(["use1-az4"]);
    assert!(!verified(
        &changed,
        "aws_finspace_kx_dataview.sample3",
        "segment_configurations.volume_name"
    ));
    let mut changed = fixture.clone();
    changed["resource_changes"][7]["change"]["after"]["environment_id"] =
        json!("other-environment");
    assert!(!verified(
        &changed,
        "aws_finspace_kx_cluster.sample1",
        "tickerplant_log_configuration.tickerplant_log_volumes"
    ));
    let mut changed = fixture.clone();
    changed["resource_changes"][14]["deposed"] = json!("old");
    assert!(!verified(
        &changed,
        "aws_finspace_kx_environment.sample0",
        "kms_key_id"
    ));
}
