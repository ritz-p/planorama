use super::*;
use crate::{layout::Layout, plan, semantic};

#[test]
fn real_and_synthetic_entities_have_stable_distinct_ids_and_provenance() {
    let input = include_str!("../../fixtures/components-plan.json");
    let raw = plan::parse(input).unwrap();
    let graph = semantic::transform(&raw);
    assert_eq!(graph.entities().count(), graph.nodes.len() + 1);
    for node in &graph.nodes {
        assert_eq!(node.entity.kind, ArchitectureEntityKind::Terraform);
        assert_eq!(
            node.entity.provenance,
            vec![TerraformEntityId {
                address: node.address.clone(),
                deposed_key: node.deposed_key.clone()
            }]
        );
    }
    let component = &graph.components[0];
    assert_eq!(component.entity.kind, ArchitectureEntityKind::Synthetic);
    assert_eq!(component.entity.provenance.len(), 3);
    assert_ne!(component.entity.id.as_str(), component.label);
    let ids: std::collections::BTreeSet<_> = graph.entities().map(|e| &e.id).collect();
    assert_eq!(ids.len(), graph.entities().count());
    let mut reordered: serde_json::Value = serde_json::from_str(input).unwrap();
    reordered["resource_changes"]
        .as_array_mut()
        .unwrap()
        .reverse();
    assert_eq!(
        graph,
        semantic::transform(&plan::parse(&reordered.to_string()).unwrap())
    );
    let mut sources = component.entity.provenance.clone();
    sources.reverse();
    sources.push(sources[0].clone());
    let reconstructed = ArchitectureEntity::synthetic("load_balancer", "aws_lb.app", sources);
    assert_eq!(component.entity, reconstructed);
}

#[test]
fn deposed_identity_is_distinct_and_change_metadata_is_not_identity() {
    let current = TerraformEntityId {
        address: "module.a.test.x[\"日本語\"]".into(),
        deposed_key: None,
    };
    let old = TerraformEntityId {
        deposed_key: Some("old".into()),
        ..current.clone()
    };
    assert_ne!(
        ArchitectureEntity::terraform(current.clone()).id,
        ArchitectureEntity::terraform(old).id
    );
    assert_ne!(
        ArchitectureEntity::terraform(current).id,
        ArchitectureId::synthetic("logical", "same")
    );
    let mut raw = plan::parse(include_str!("../../fixtures/containment-plan.json")).unwrap();
    let before: Vec<_> = semantic::transform(&raw)
        .entities()
        .map(|e| e.id.clone())
        .collect();
    for node in &mut raw.nodes {
        node.action = crate::model::Action::Delete;
        node.previous_address = Some("old.name".into());
    }
    assert_eq!(
        before,
        semantic::transform(&raw)
            .entities()
            .map(|e| e.id.clone())
            .collect::<Vec<_>>()
    );
}

#[test]
fn layout_uses_native_identity_and_can_position_an_address_free_aggregate() {
    let raw = plan::parse(include_str!("../../fixtures/components-plan.json")).unwrap();
    let graph = semantic::transform(&raw);
    let component = graph.components[0].clone();
    let only_aggregate = Graph {
        nodes: vec![],
        edges: vec![],
        relationships: Vec::new(),
        components: vec![component.clone()],
        checks: vec![],
        status: Default::default(),
    };
    let layout = Layout::new(&only_aggregate);
    let panels = layout.component_bounds(&only_aggregate);
    assert_eq!(panels.len(), 1);
    assert_eq!(*panels[0].0, component.entity.id);
    assert!(panels[0].1.height > 0);
    let mut graph = semantic::transform(
        &plan::parse(include_str!("../../fixtures/containment-plan.json")).unwrap(),
    );
    let bounds = Layout::new(&graph).bounds;
    for node in &mut graph.0.nodes {
        node.address.clear();
    }
    assert_eq!(Layout::new(&graph).bounds, bounds);
}

#[test]
fn svg_exposes_all_entity_sources_without_raw_values() {
    let graph = semantic::transform(
        &plan::parse(include_str!("../../fixtures/components-plan.json")).unwrap(),
    );
    let image = crate::svg::render(&graph, &Layout::new(&graph));
    let doc = roxmltree::Document::parse(&image).unwrap();
    let text = doc
        .descendants()
        .find(|n| n.attribute("id") == Some("architecture-entities"))
        .unwrap()
        .text()
        .unwrap();
    let metadata: serde_json::Value = serde_json::from_str(text).unwrap();
    assert_eq!(metadata.as_array().unwrap().len(), graph.entities().count());
    let aggregate = metadata
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["kind"] == "synthetic")
        .unwrap();
    assert_eq!(aggregate["sources"].as_array().unwrap().len(), 3);
    assert!(!text.contains("TOP_SECRET"));
}
