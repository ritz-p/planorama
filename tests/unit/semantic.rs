use super::*;
use crate::{layout::Layout, plan, svg};

#[test]
fn unmatched_graphs_preserve_structure_order_and_svg() {
    for input in [
        include_str!("../fixtures/terraform-plan.json"),
        include_str!("../../examples/bundling-plan.json"),
        r#"{"format_version":"1.0","resource_changes":[]}"#,
    ] {
        let raw = plan::parse(input).unwrap();
        let architecture = transform(&raw);
        assert_eq!(architecture.0, raw.graph);
        assert_eq!(architecture, transform(&raw));
        assert_eq!(
            svg::render(&raw, &Layout::new(&raw)),
            svg::render(&architecture, &Layout::new(&architecture))
        );
    }
}

#[test]
fn architecture_changes_do_not_mutate_raw_terraform_graph() {
    let raw = plan::parse(include_str!("../../examples/plan.json")).unwrap();
    let original = raw.clone();
    let mut architecture = transform(&raw);
    architecture.0.nodes[0].address = "changed".into();
    architecture.0.edges.clear();
    assert_eq!(raw, original);
    assert_ne!(architecture.0, raw.graph);
}
