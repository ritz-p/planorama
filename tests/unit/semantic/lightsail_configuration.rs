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
        "../../fixtures/lightsail-configuration-plan.json"
    ))
    .unwrap();
    for (address, path) in [
        ("aws_lightsail_disk_attachment.sample9", "disk_name"),
        (
            "aws_lightsail_static_ip_attachment.sample10",
            "static_ip_name",
        ),
        ("aws_lightsail_lb_attachment.sample11", "lb_name"),
        (
            "aws_lightsail_bucket_resource_access.sample12",
            "bucket_name",
        ),
        ("aws_lightsail_bucket_access_key.sample13", "bucket_name"),
        (
            "aws_lightsail_instance_public_ports.sample14",
            "instance_name",
        ),
        (
            "aws_lightsail_container_service_deployment_version.sample15",
            "service_name",
        ),
        ("aws_lightsail_lb_certificate.sample16", "lb_name"),
        (
            "aws_lightsail_lb_certificate_attachment.sample17",
            "lb_name",
        ),
        (
            "aws_lightsail_lb_https_redirection_policy.sample18",
            "lb_name",
        ),
        ("aws_lightsail_lb_stickiness_policy.sample19", "lb_name"),
        ("aws_lightsail_domain_entry.sample20", "domain_name"),
        ("aws_lightsail_disk_attachment.sample9", "instance_name"),
        (
            "aws_lightsail_static_ip_attachment.sample10",
            "instance_name",
        ),
        ("aws_lightsail_lb_attachment.sample11", "instance_name"),
        (
            "aws_lightsail_bucket_resource_access.sample12",
            "resource_name",
        ),
        (
            "aws_lightsail_lb_certificate_attachment.sample17",
            "certificate_name",
        ),
        ("aws_lightsail_distribution.sample21", "origin.name"),
        ("aws_lightsail_distribution.sample21", "certificate_name"),
        (
            "aws_lightsail_container_service.sample5",
            "public_domain_names.certificate.certificate_name",
        ),
        ("aws_lightsail_instance.sample0", "key_pair_name"),
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
    changed["resource_changes"][21]["change"]["after"]["origin"][0]["region_name"] =
        json!("us-west-2");
    assert!(!verified(
        &changed,
        "aws_lightsail_distribution.sample21",
        "origin.name"
    ));
    let mut changed = fixture.clone();
    changed["resource_changes"][16]["change"]["after"]["lb_name"] = json!("other-lb");
    assert!(!verified(
        &changed,
        "aws_lightsail_lb_certificate_attachment.sample17",
        "certificate_name"
    ));
    let mut changed = fixture.clone();
    changed["resource_changes"][1]["deposed"] = json!("old");
    assert!(!verified(
        &changed,
        "aws_lightsail_disk_attachment.sample9",
        "disk_name"
    ));
}

#[test]
fn cross_region_distribution_requires_verified_origin_account() {
    let mut fixture: Value = serde_json::from_str(include_str!(
        "../../fixtures/lightsail-configuration-plan.json"
    ))
    .unwrap();
    let aws = &mut fixture["configuration"]["provider_config"];
    aws["aws"]["expressions"]["assume_role"] =
        json!([{"role_arn":{"constant_value":"arn:aws:iam::123456789012:role/Fixture"}}]);
    let mut origin = aws["aws"].clone();
    origin["alias"] = json!("origin");
    origin["expressions"]["region"] = json!({"constant_value":"us-west-2"});
    aws["aws.origin"] = origin;
    fixture["configuration"]["root_module"]["resources"][4]["provider_config_key"] =
        json!("aws.origin");
    fixture["resource_changes"][21]["change"]["after"]["origin"][0]["region_name"] =
        json!("us-west-2");
    let path = "origin.name";
    let address = "aws_lightsail_distribution.sample21";
    assert!(!verified(&fixture, address, path));
    let verified_origin = |fixture: &Value| {
        let plan = plan::evidence::tests::parse_fixture(&fixture.to_string()).unwrap();
        let graph = semantic::transform(&plan);
        let distribution = graph
            .nodes
            .iter()
            .find(|node| node.address == address)
            .unwrap();
        distribution.references.iter().any(|reference| {
            reference.attribute == path
                && reference.complete
                && !reference.dependency
                && !reference.targets.is_empty()
                && super::label(&distribution.resource_type, path) == Some(reference.label)
        })
    };
    assert!(verified_origin(&fixture));
    fixture["configuration"]["provider_config"]["aws.origin"]["expressions"]["assume_role"][0]["role_arn"]
        ["constant_value"] = json!("arn:aws:iam::234567890123:role/Fixture");
    assert!(!verified_origin(&fixture));
}
