use super::*;

#[test]
fn mixed_relationships_identify_each_kind_without_dependency_only_captions() {
    let raw = crate::plan::parse(include_str!("../fixtures/association-plan.json")).unwrap();
    let mut graph = crate::semantic::transform(&raw);
    graph.0.edges.push(crate::model::Edge::from((0, 1)));
    let output = render(&graph, &Layout::new(&graph));
    assert!(output.contains(" (dependency)</title>"));
    assert!(output.contains(" (association; create:"));
    assert!(output.contains("3 resources (2 cards), 2 relationships"));
    assert!(!output.contains("reference edges"));
    assert!(!output.contains("Arrows point from dependencies"));
}

#[test]
fn escapes_markup_and_filters_invalid_control_characters() {
    assert_eq!(
        escape("<script>&\"'\u{0}\u{1}\n\r\t"),
        "&lt;script&gt;&amp;&quot;&apos;\n\r\t"
    );
}
