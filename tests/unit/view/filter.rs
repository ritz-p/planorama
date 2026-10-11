use super::*;

#[test]
fn filtering_does_not_mutate_source_graph_or_plan() {
    let raw = crate::plan::parse(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/containment-plan.json"
    )))
    .unwrap();
    let original = raw.clone();
    let graph = crate::semantic::transform(&raw);
    let before = graph.clone();
    single(
        &graph,
        &Options {
            focus: Some("aws_instance.app".into()),
            depth: Some(0),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(graph, before);
    assert_eq!(raw, original);
}
