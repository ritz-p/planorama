use crate::{
    model::{EdgeKind, Graph},
    plan, semantic,
};
use serde_json::{Value, json};
const FIXTURE: &str = include_str!("../../fixtures/analytics-databases-plan.json");
fn graph(v: &Value) -> crate::model::ArchitectureGraph {
    semantic::transform(&plan::evidence::tests::parse_fixture(&v.to_string()).unwrap())
}
fn verified(v: &Value, t: usize, p: &str) -> bool {
    graph(v)
        .nodes
        .iter()
        .find(|n| n.address == v["resource_changes"][t]["address"])
        .unwrap()
        .references
        .iter()
        .any(|r| {
            r.attribute == p
                && r.complete
                && Some(r.label)
                    == super::label(v["resource_changes"][t]["type"].as_str().unwrap(), p)
        })
}
fn parent(g: &Graph, address: &str) -> Option<String> {
    let t = g.nodes.iter().position(|n| n.address == address).unwrap();
    g.edges
        .iter()
        .find(|e| e.to == t && e.kind == EdgeKind::Containment)
        .map(|e| g.nodes[e.from].address.clone())
}
#[test]
fn complete_database_evidence_places_only_network_clients() {
    let v: Value = serde_json::from_str(FIXTURE).unwrap();
    for (t, p) in [
        (5, "subnet_ids"),
        (5, "vpc_security_group_ids"),
        (5, "kms_key_id"),
        (6, "kms_key_identifier"),
        (7, "graph_identifier"),
        (7, "vpc_id"),
        (7, "subnet_ids"),
        (7, "vpc_security_group_ids"),
    ] {
        assert!(verified(&v, t, p), "{t} {p}");
    }
    let g = graph(&v);
    for t in [5, 7] {
        assert_eq!(
            parent(&g, v["resource_changes"][t]["address"].as_str().unwrap()).as_deref(),
            Some("aws_vpc.main")
        );
    }
    assert_eq!(parent(&g, "aws_neptunegraph_graph.main"), None);
    assert_eq!(g.coverage.selected.resources, 8);
}
#[test]
fn incomplete_collections_and_wrong_graph_outputs_fall_back() {
    let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
    v["resource_changes"][5]["change"]["after_unknown"] = json!({"subnet_ids":[false,true]});
    v["configuration"]["root_module"]["resources"][7]["expressions"]["graph_identifier"] =
        json!({"references":["aws_neptunegraph_graph.main.name"]});
    v["resource_changes"][6]["change"]["after"]["name"] = json!("g-example");
    assert!(!verified(&v, 5, "subnet_ids"));
    assert!(!verified(&v, 7, "graph_identifier"));
    assert_eq!(parent(&graph(&v), "aws_docdbelastic_cluster.main"), None);
}
