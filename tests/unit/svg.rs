use super::*;

#[test]
fn policy_and_controller_fallbacks_keep_resources_actions_and_dependencies() {
    let raw = crate::plan::parse(r#"{"format_version":"1.2","resource_changes":[
        {"address":"aws_security_group.app","type":"aws_security_group","change":{"actions":["update"]}},
        {"address":"aws_ecs_service.app","type":"aws_ecs_service","change":{"actions":["create"]}}
    ],"configuration":{"root_module":{"resources":[
        {"address":"aws_security_group.app"},
        {"address":"aws_ecs_service.app","expressions":{"security_groups":{"references":["aws_security_group.app.id"]}}}
    ]}}}"#).unwrap();
    let graph = crate::semantic::transform(&raw);
    assert_eq!(graph.0, raw.graph);
    let output = render(&graph, &Layout::new(&graph));
    for value in [
        "data-role=\"policy\"",
        "data-role=\"controller\"",
        "aws_security_group.app",
        "aws_ecs_service.app",
        "update (1)",
        "create (1)",
    ] {
        assert!(output.contains(value), "missing {value}");
    }
    assert_eq!(output.matches("marker-end=").count(), 1);
    assert_eq!(output, render(&graph, &Layout::new(&graph)));
}

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
