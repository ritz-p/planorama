use super::*;
#[path = "data_icons.rs"]
mod data_icons;
#[path = "service_names.rs"]
mod service_names;
#[test]
fn unknown_only_diagrams_do_not_embed_unused_artwork() {
    let raw = crate::plan::parse(r#"{"format_version":"1.2","resource_changes":[{"address":"custom_resource.main","type":"custom_resource"}]}"#).unwrap();
    let graph = crate::semantic::transform(&raw);
    assert_eq!(definitions(&graph.nodes), "");
    let svg = crate::svg::render(&graph, &crate::layout::Layout::new(&graph));
    assert!(!svg.contains("<use"));
    assert!(!svg.contains("<symbol"));
    assert!(!svg.contains("planorama-icons-license"));
}

#[test]
fn large_example_icons_are_independent_of_roles_and_entity_modes() {
    let raw =
        crate::plan::parse(include_str!("../../../examples/terraform-large/plan.json")).unwrap();
    let graph = crate::semantic::transform(&raw);
    for node in &graph.nodes {
        let expected = for_node(node);
        assert!(expected.is_none(), "{}", node.resource_type);
        let mut modified = node.clone();
        modified.mode = crate::model::EntityMode::Data;
        modified.role = crate::model::ResourceRole::Unknown;
        assert_eq!(expected, for_node(&modified));
    }
}
