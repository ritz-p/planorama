use super::*;
use crate::model::Node;
#[test]
fn escapes_untrusted_labels_and_omits_values() {
    let graph = Graph {
        nodes: vec![Node {
            address: "test.<script>&\"".into(),
            resource_type: "test".into(),
            module: "root".into(),
            action: Action::Create,
        }],
        edges: vec![],
    };
    let svg = render_graph(&graph);
    assert!(!svg.contains("<script>"));
    assert!(svg.contains("&lt;script&gt;&amp;&quot;"));
    let graph = crate::plan::parse(r#"{"format_version":"1.0","resource_changes":[{"address":"test.main","change":{"actions":["create"],"after":{"password":"TOP_SECRET"}}}]}"#).unwrap();
    assert!(!render_graph(&graph).contains("TOP_SECRET"));
}
#[test]
fn output_is_deterministic_and_empty_plans_render() {
    let graph = crate::plan::parse(include_str!("../../examples/plan.json")).unwrap();
    assert_eq!(render_graph(&graph), render_graph(&graph));
    assert!(
        render_graph(&Graph {
            nodes: vec![],
            edges: vec![]
        })
        .contains("No resources")
    );
}

fn render_graph(graph: &Graph) -> String {
    super::render(graph, &Layout::new(graph))
}
