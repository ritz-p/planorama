use super::*;

#[test]
fn projection_keeps_native_relationship_endpoints_and_source_immutable() {
    let plan = crate::plan::parse(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/containment-plan.json"
    )))
    .unwrap();
    let architecture = crate::semantic::transform(&plan);
    let original = architecture.clone();
    let full = single(&architecture, &Options::default()).unwrap();
    assert_eq!(*full, architecture.0);
    let focused = single(
        &architecture,
        &Options {
            focus: Some("aws_instance.app".into()),
            depth: Some(0),
            ..Default::default()
        },
    )
    .unwrap();
    let ids: std::collections::BTreeSet<_> = focused.entities().map(|entity| &entity.id).collect();
    assert!(
        focused
            .nodes
            .iter()
            .any(|node| node.resource_type == "aws_vpc")
    );
    for relationship in &focused.relationships {
        assert!(ids.contains(&relationship.from) && ids.contains(&relationship.to));
        assert!(architecture.relationships.contains(relationship));
    }
    assert_eq!(architecture, original);
}
