use crate::model::*;

#[test]
fn native_relationships_support_synthetic_endpoints_and_multiple_sources() {
    let source = |address: &str| TerraformEntityId {
        address: address.into(),
        deposed_key: None,
    };
    let a = RelationshipProvenance::Reference {
        from: source("test.a"),
        to: source("test.b"),
    };
    let b = RelationshipProvenance::Reference {
        from: source("test.c"),
        to: source("test.b"),
    };
    let from = ArchitectureId::synthetic("component", "a");
    let to = ArchitectureId::synthetic("component", "b");
    let first = ArchitectureRelationship::new(
        from.clone(),
        to.clone(),
        EdgeKind::Connection,
        true,
        vec![b.clone(), a.clone(), a.clone()],
    );
    let second = ArchitectureRelationship::new(from, to, EdgeKind::Connection, true, vec![a, b]);
    assert_eq!(first, second);
    assert_eq!(first.provenance.len(), 2);
}

#[test]
fn lowered_and_inferred_relationships_retain_distinct_evidence() {
    for fixture in [
        include_str!("../../fixtures/association-plan.json"),
        include_str!("../../fixtures/security-groups-plan.json"),
        include_str!("../../fixtures/containment-plan.json"),
    ] {
        let plan = crate::plan::parse(fixture).unwrap();
        let graph = crate::semantic::transform(&plan);
        let ids: std::collections::BTreeSet<_> = graph.entities().map(|e| &e.id).collect();
        for relationship in &graph.relationships {
            assert!(ids.contains(&relationship.from) && ids.contains(&relationship.to));
            assert!(!relationship.provenance.is_empty());
            if relationship.kind == EdgeKind::Association {
                assert!(!relationship.inferred);
                assert!(matches!(
                    relationship.provenance[0],
                    RelationshipProvenance::Resource { .. }
                ));
            } else if relationship.kind != EdgeKind::Dependency {
                assert!(relationship.inferred);
                assert!(
                    relationship
                        .provenance
                        .iter()
                        .all(|p| matches!(p, RelationshipProvenance::Reference { .. }))
                );
            }
        }
        let svg = crate::svg::render(&graph, &crate::layout::Layout::new(&graph));
        assert!(svg.contains("architecture-relationships"));
    }
}
