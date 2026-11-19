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
        "../../fixtures/keyspaces-configuration-plan.json"
    ))
    .unwrap();
    for (address, path) in [
        ("aws_keyspaces_table.sample1", "keyspace_name"),
        (
            "aws_keyspaces_table.sample1",
            "encryption_specification.kms_key_identifier",
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
    changed["resource_changes"][1]["change"]["after"]["encryption_specification"][0]["type"] =
        json!("AWS_OWNED_KMS_KEY");
    assert!(!verified(
        &changed,
        "aws_keyspaces_table.sample1",
        "encryption_specification.kms_key_identifier"
    ));
    let mut changed = fixture.clone();
    changed["resource_changes"][0]["deposed"] = json!("old");
    assert!(!verified(
        &changed,
        "aws_keyspaces_table.sample1",
        "keyspace_name"
    ));
}

#[test]
fn member_owner_preserves_known_provider_account() {
    let parsed = plan::evidence::tests::parse_fixture(include_str!(
        "../../fixtures/keyspaces-configuration-plan.json"
    ))
    .unwrap();
    let graph = semantic::transform(&parsed);
    let node = graph
        .nodes
        .iter()
        .find(|node| node.address == "aws_keyspaces_table.sample1")
        .unwrap();
    let owner = node.member_owner.as_ref().unwrap();
    assert_eq!(owner.target.address, "aws_keyspaces_keyspace.sample0");
    assert_eq!(owner.section, "Tables");
    assert_eq!(
        node.entity
            .scope
            .as_ref()
            .and_then(|scope| scope.account.as_ref())
            .map(|account| account.as_str()),
        Some("123456789012")
    );

    let address = "aws_keyspaces_table.sample1";
    for options in [
        crate::view::Options::default(),
        crate::view::Options {
            focus: Some(address.into()),
            depth: Some(0),
            ..Default::default()
        },
    ] {
        let selected = crate::view::single(&graph, &options).unwrap();
        let svg = crate::svg::render(&selected, &crate::layout::Layout::new(&selected));
        let doc = roxmltree::Document::parse(&svg).unwrap();
        let mut tables = doc
            .descendants()
            .filter(|element| element.attribute("data-terraform-address") == Some(address));
        let table = tables.next().unwrap();
        assert!(tables.next().is_none());
        if options.focus.is_none() {
            assert_eq!(table.attribute("data-embedded-member"), Some("true"));
            assert_eq!(
                table.attribute("data-member-owner"),
                Some(owner.target.address.as_str())
            );
            let section = table
                .ancestors()
                .find(|element| element.attribute("data-cluster-members") == Some("true"))
                .unwrap();
            assert!(
                section
                    .children()
                    .any(|element| element.text() == Some("Tables"))
            );
        } else {
            assert_eq!(selected.coverage.selected.resources, 1);
            assert_eq!(table.attribute("data-embedded-member"), None);
        }
    }
}

#[test]
fn shared_table_retains_its_keyspace_reference_without_embedding() {
    let mut fixture: Value = serde_json::from_str(include_str!(
        "../../fixtures/keyspaces-configuration-plan.json"
    ))
    .unwrap();
    fixture["resource_changes"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "address": "terraform_data.consumer",
            "mode": "managed",
            "type": "terraform_data",
            "name": "consumer",
            "provider_name": "terraform.io/builtin/terraform",
            "change": {"actions": ["create"], "after": {"input": "fixture"}}
        }));
    fixture["configuration"]["root_module"]["resources"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "address": "terraform_data.consumer",
            "expressions": {"input": {"references": ["aws_keyspaces_table.sample1.id"]}}
        }));
    let address = "aws_keyspaces_table.sample1";
    assert!(verified(&fixture, address, "keyspace_name"));
    let parsed = plan::parse(&fixture.to_string()).unwrap();
    let graph = semantic::transform(&parsed);
    let table = graph
        .nodes
        .iter()
        .find(|node| node.address == address)
        .unwrap();
    assert_eq!(
        crate::view::presentation::effective(&graph, table),
        crate::view::presentation::Effective::Logical
    );
    assert!(table.member_owner.is_none());
}
