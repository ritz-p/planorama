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
        serde_json::from_str(include_str!("../../fixtures/verified-access-plan.json")).unwrap();
    for (address, path) in [
        (
            "aws_verifiedaccess_endpoint.sample0",
            "verified_access_group_id",
        ),
        (
            "aws_verifiedaccess_group.sample1",
            "verifiedaccess_instance_id",
        ),
        (
            "aws_verifiedaccess_endpoint.sample0",
            "cidr_options.subnet_ids",
        ),
        ("aws_verifiedaccess_endpoint.sample0", "security_group_ids"),
        (
            "aws_verifiedaccess_instance_logging_configuration.sample6",
            "verifiedaccess_instance_id",
        ),
        (
            "aws_verifiedaccess_instance_logging_configuration.sample6",
            "access_logs.s3.bucket_name",
        ),
        (
            "aws_verifiedaccess_instance_trust_provider_attachment.sample8",
            "verifiedaccess_instance_id",
        ),
        (
            "aws_verifiedaccess_instance_trust_provider_attachment.sample8",
            "verifiedaccess_trust_provider_id",
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
    changed["resource_changes"][0]["change"]["after"]["endpoint_type"] = json!("rds");
    assert!(!verified(
        &changed,
        "aws_verifiedaccess_endpoint.sample0",
        "cidr_options.subnet_ids"
    ));
    let mut changed = fixture.clone();
    changed["resource_changes"][3]["change"]["after"]["vpc_id"] = json!("other-vpc");
    assert!(!verified(
        &changed,
        "aws_verifiedaccess_endpoint.sample0",
        "security_group_ids"
    ));
    let mut changed = fixture.clone();
    changed["resource_changes"][6]["change"]["after"]["access_logs"][0]["s3"][0]["enabled"] =
        json!(false);
    assert!(!verified(
        &changed,
        "aws_verifiedaccess_instance_logging_configuration.sample6",
        "access_logs.s3.bucket_name"
    ));
    let mut changed = fixture.clone();
    changed["resource_changes"][1]["deposed"] = json!("old");
    assert!(!verified(
        &changed,
        "aws_verifiedaccess_endpoint.sample0",
        "verified_access_group_id"
    ));
}

#[test]
fn endpoint_placement_requires_all_configured_network_evidence() {
    let mut f: Value =
        serde_json::from_str(include_str!("../../fixtures/verified-access-plan.json")).unwrap();
    let contained = |f: &Value| {
        let parsed = plan::parse(&f.to_string()).unwrap();
        let graph = semantic::transform(&parsed);
        graph.edges.iter().any(|edge| {
            edge.kind == crate::model::EdgeKind::Containment
                && graph.nodes[edge.from].address == "aws_vpc.sample4"
                && graph.nodes[edge.to].address == "aws_verifiedaccess_endpoint.sample0"
        })
    };
    assert!(contained(&f));
    f["resource_changes"][5]["change"]["after_sensitive"]["vpc_id"] = json!(true);
    assert!(!contained(&f));
}
