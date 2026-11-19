use super::*;

const FIXTURE: &str = include_str!("../../fixtures/cloudflare-imported-plan.json");

#[test]
fn configured_facts_materialize_consistent_edges_once_without_fabricating_references() {
    let plan = crate::plan::parse_with_value_matches(FIXTURE, true).unwrap();
    let mut graph = crate::semantic::transform(&plan).0;
    let expected = graph.clone();
    apply(&plan, &mut graph);
    assert_eq!(graph, expected);
    assert_eq!(
        graph.relationships.len(),
        plan.configured_value_matches.len()
    );
    for relationship in &graph.relationships {
        assert!(relationship.inferred);
        assert_eq!(relationship.kind, EdgeKind::Connection);
        assert_eq!(
            relationship.directionality(),
            crate::model::Directionality::Undirected
        );
        assert_eq!(relationship.provenance.len(), 1);
        assert_eq!(
            relationship.provenance[0].evidence(),
            crate::model::RelationshipEvidence::ConfiguredValue
        );
        let from = graph
            .nodes
            .iter()
            .position(|node| node.entity.id == relationship.from)
            .unwrap();
        let to = graph
            .nodes
            .iter()
            .position(|node| node.entity.id == relationship.to)
            .unwrap();
        assert_eq!(
            graph
                .edges
                .iter()
                .filter(|edge| edge.from == from && edge.to == to && edge.kind == relationship.kind)
                .count(),
            1
        );
    }
    let ordinary = crate::semantic::transform(&crate::plan::parse(FIXTURE).unwrap());
    assert!(ordinary.relationships.is_empty());
    assert_eq!(
        graph
            .nodes
            .iter()
            .map(|node| &node.references)
            .collect::<Vec<_>>(),
        ordinary
            .nodes
            .iter()
            .map(|node| &node.references)
            .collect::<Vec<_>>()
    );
}

#[test]
fn repeated_projection_repairs_either_representation_without_duplicates() {
    let plan = crate::plan::parse_with_value_matches(FIXTURE, true).unwrap();
    let expected = crate::semantic::transform(&plan).0;
    let mut edges = expected.clone();
    edges.edges.clear();
    apply(&plan, &mut edges);
    assert_eq!(edges, expected);
    let mut relationships = expected.clone();
    relationships.relationships.clear();
    apply(&plan, &mut relationships);
    assert_eq!(relationships, expected);
}

#[test]
fn explicit_dependency_and_value_match_keep_distinct_evidence_and_directionality() {
    let mut value: serde_json::Value = serde_json::from_str(FIXTURE).unwrap();
    let route = value["configuration"]["root_module"]["resources"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|resource| resource["address"] == "cloudflare_workers_route.api")
        .unwrap();
    route["depends_on"] = serde_json::json!(["cloudflare_worker.api"]);
    let plan = crate::plan::parse_with_value_matches(&value.to_string(), true).unwrap();
    let mut graph = crate::semantic::transform(&plan).0;
    let from = graph
        .nodes
        .iter()
        .position(|node| node.address == "cloudflare_worker.api")
        .unwrap();
    let to = graph
        .nodes
        .iter()
        .position(|node| node.address == "cloudflare_workers_route.api")
        .unwrap();
    let kinds: BTreeSet<_> = graph
        .edges
        .iter()
        .filter(|edge| edge.from == from && edge.to == to)
        .map(|edge| edge.kind)
        .collect();
    assert_eq!(
        kinds,
        BTreeSet::from([EdgeKind::Dependency, EdgeKind::Connection])
    );
    let dependency = graph
        .relationships
        .iter()
        .find(|r| {
            r.from == graph.nodes[from].entity.id
                && r.to == graph.nodes[to].entity.id
                && r.kind == EdgeKind::Dependency
        })
        .unwrap();
    assert!(!dependency.inferred);
    assert!(
        dependency
            .provenance
            .iter()
            .all(|proof| proof.evidence() == crate::model::RelationshipEvidence::Reference)
    );
    assert_eq!(
        dependency.directionality(),
        crate::model::Directionality::Directed
    );
    let matched = graph
        .relationships
        .iter()
        .find(|r| {
            r.from == graph.nodes[from].entity.id
                && r.to == graph.nodes[to].entity.id
                && r.kind == EdgeKind::Connection
        })
        .unwrap();
    assert!(matched.inferred);
    assert!(
        matched
            .provenance
            .iter()
            .all(|proof| proof.evidence() == crate::model::RelationshipEvidence::ConfiguredValue)
    );
    assert_eq!(
        matched.directionality(),
        crate::model::Directionality::Undirected
    );
    let expected = graph.clone();
    apply(&plan, &mut graph);
    assert_eq!(graph, expected);
}
