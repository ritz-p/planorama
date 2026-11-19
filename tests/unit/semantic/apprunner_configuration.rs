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
        "../../fixtures/apprunner-configuration-plan.json"
    ))
    .unwrap();
    for (address, path) in [
        (
            "aws_apprunner_service.sample1",
            "auto_scaling_configuration_arn",
        ),
        (
            "aws_apprunner_service.sample1",
            "instance_configuration.instance_role_arn",
        ),
        (
            "aws_apprunner_service.sample1",
            "source_configuration.authentication_configuration.connection_arn",
        ),
        (
            "aws_apprunner_service.sample1",
            "observability_configuration.observability_configuration_arn",
        ),
        (
            "aws_apprunner_default_auto_scaling_configuration_version.sample5",
            "auto_scaling_configuration_arn",
        ),
        ("aws_apprunner_deployment.sample6", "service_arn"),
        (
            "aws_apprunner_custom_domain_association.sample7",
            "service_arn",
        ),
        (
            "aws_apprunner_service.sample8",
            "source_configuration.authentication_configuration.access_role_arn",
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
    changed["resource_changes"][0]["change"]["after"]["arn"] = json!(
        "arn:aws:apprunner:us-east-1:123456789012:autoscalingconfiguration/shared/4/config-id"
    );
    assert!(!verified(
        &changed,
        "aws_apprunner_service.sample1",
        "auto_scaling_configuration_arn"
    ));
    let mut changed = fixture.clone();
    changed["resource_changes"][1]["change"]["after"]["observability_configuration"][0]["observability_enabled"] =
        json!(false);
    assert!(!verified(
        &changed,
        "aws_apprunner_service.sample1",
        "observability_configuration.observability_configuration_arn"
    ));
    let mut changed = fixture.clone();
    changed["resource_changes"][8]["change"]["after"]["source_configuration"][0]["image_repository"]
        [0]["image_repository_type"] = json!("ECR_PUBLIC");
    assert!(!verified(
        &changed,
        "aws_apprunner_service.sample8",
        "source_configuration.authentication_configuration.access_role_arn"
    ));
    let mut changed = fixture.clone();
    changed["resource_changes"][1]["deposed"] = json!("old");
    assert!(!verified(
        &changed,
        "aws_apprunner_deployment.sample6",
        "service_arn"
    ));
}
