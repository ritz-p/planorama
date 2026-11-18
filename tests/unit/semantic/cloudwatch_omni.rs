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
        serde_json::from_str(include_str!("../../fixtures/cloudwatch-omni-plan.json")).unwrap();
    for (address, path) in [
        ("aws_cloudwatchomni_space.sample1", "domain_id"),
        ("aws_cloudwatchomni_space.sample1", "data_access_role_arn"),
        (
            "aws_cloudwatchomni_space.sample1",
            "encryption_configuration.kms_key_arn",
        ),
        ("aws_cloudwatchomni_access_grant.sample3", "space_id"),
        (
            "aws_cloudwatchomni_access_grant.sample3",
            "principal.principal_id",
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
    changed["resource_changes"][1]["change"]["after"]["domain_id"] = json!("another-domain");
    assert!(!verified(
        &changed,
        "aws_cloudwatchomni_access_grant.sample3",
        "space_id"
    ));
    let mut changed = fixture.clone();
    changed["resource_changes"][3]["change"]["after"]["principal"][0]["principal_type"] =
        json!("IDC_USER");
    assert!(!verified(
        &changed,
        "aws_cloudwatchomni_access_grant.sample3",
        "principal.principal_id"
    ));
    let mut changed = fixture.clone();
    changed["resource_changes"][1]["change"]["after"]["encryption_configuration"][0]["encryption_strategy"] =
        json!("AWS_OWNED");
    assert!(!verified(
        &changed,
        "aws_cloudwatchomni_space.sample1",
        "encryption_configuration.kms_key_arn"
    ));
}

#[test]
fn verified_domain_and_space_retain_their_account_and_regions() {
    let mut fixture: Value =
        serde_json::from_str(include_str!("../../fixtures/cloudwatch-omni-plan.json")).unwrap();
    for provider in fixture["configuration"]["provider_config"]
        .as_object_mut()
        .unwrap()
        .values_mut()
    {
        provider["expressions"]["assume_role"] =
            json!([{"role_arn":{"constant_value":"arn:aws:iam::123456789012:role/fixture"}}]);
    }
    let parsed = plan::evidence::tests::parse_fixture(&fixture.to_string()).unwrap();
    let graph = semantic::transform(&parsed);
    for (address, region) in [
        ("aws_cloudwatchomni_domain.sample0", "us-west-2"),
        ("aws_cloudwatchomni_space.sample1", "us-east-1"),
    ] {
        let node = graph
            .nodes
            .iter()
            .find(|node| node.address == address)
            .unwrap();
        let scope = node.entity.scope.as_ref().unwrap();
        assert_eq!(scope.account.as_ref().unwrap().as_str(), "123456789012");
        assert_eq!(scope.region.as_ref().unwrap().0, region);
        assert!(node.member_owner.is_none());
    }
}
