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
        "../../fixtures/beanstalk-configuration-plan.json"
    ))
    .unwrap();
    for (address, path) in [
        (
            "aws_elastic_beanstalk_application.sample0",
            "appversion_lifecycle.service_role",
        ),
        ("aws_elastic_beanstalk_environment.sample1", "application"),
        ("aws_elastic_beanstalk_environment.sample1", "version_label"),
        ("aws_elastic_beanstalk_environment.sample1", "template_name"),
        (
            "aws_elastic_beanstalk_application_version.sample2",
            "application",
        ),
        (
            "aws_elastic_beanstalk_application_version.sample2",
            "bucket",
        ),
        (
            "aws_elastic_beanstalk_configuration_template.sample3",
            "application",
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
    changed["resource_changes"][2]["change"]["after"]["application"] = json!("other");
    assert!(!verified(
        &changed,
        "aws_elastic_beanstalk_environment.sample1",
        "version_label"
    ));
    let mut changed = fixture.clone();
    changed["resource_changes"][3]["deposed"] = json!("old");
    assert!(!verified(
        &changed,
        "aws_elastic_beanstalk_environment.sample1",
        "template_name"
    ));
    let mut changed = fixture.clone();
    changed["resource_changes"][5]["provider_name"] = json!("example.com/foreign/aws");
    assert!(!verified(
        &changed,
        "aws_elastic_beanstalk_application_version.sample2",
        "bucket"
    ));
}

#[test]
fn environment_keeps_independent_spatial_identity_and_requires_a_shared_application_endpoint() {
    let mut input: Value = serde_json::from_str(include_str!(
        "../../fixtures/beanstalk-configuration-plan.json"
    ))
    .unwrap();
    let graph = semantic::transform(&plan::parse(&input.to_string()).unwrap());
    let environment = graph
        .nodes
        .iter()
        .find(|node| node.address == "aws_elastic_beanstalk_environment.sample1")
        .unwrap();
    assert_eq!(environment.placement, crate::model::Placement::Spatial);
    assert!(environment.member_owner.is_none());
    assert_eq!(graph.coverage.selected.resources, 6);
    assert!(
        graph
            .nodes
            .iter()
            .filter(
                |node| node.resource_type.starts_with("aws_elastic_beanstalk_")
                    && node.address != environment.address
            )
            .all(|node| node.placement == crate::model::Placement::Logical)
    );
    assert!(
        graph
            .edges
            .iter()
            .all(|edge| edge.kind != crate::model::EdgeKind::Containment)
    );

    let mut other_application = input["resource_changes"][0].clone();
    other_application["address"] = json!("aws_elastic_beanstalk_application.other");
    other_application["name"] = json!("other");
    input["resource_changes"]
        .as_array_mut()
        .unwrap()
        .push(other_application);
    let resources = input["configuration"]["root_module"]["resources"]
        .as_array_mut()
        .unwrap();
    let mut other_config = resources[0].clone();
    other_config["address"] = json!("aws_elastic_beanstalk_application.other");
    other_config["name"] = json!("other");
    resources.push(other_config);
    resources[3]["expressions"]["application"] =
        json!({"references":["aws_elastic_beanstalk_application.other.name"]});
    assert!(!verified(
        &input,
        "aws_elastic_beanstalk_environment.sample1",
        "template_name"
    ));
    assert!(verified(
        &input,
        "aws_elastic_beanstalk_environment.sample1",
        "version_label"
    ));
}
