use crate::model::architecture::{ArchitectureEntityKind, Graph, RelationshipProvenance};

fn reference_target(target: &crate::model::ReferenceTarget) -> serde_json::Value {
    let mut value =
        serde_json::json!({"address":target.id.address,"deposed_key":target.id.deposed_key});
    if let Some(state) = &target.state {
        value["state"] = state.as_str().into();
    }
    value
}

pub(super) fn metadata(graph: &Graph) -> String {
    let entities: Vec<_> = graph.entities().map(|entity| {
        let sources: Vec<_> = entity.provenance.iter().map(|source| serde_json::json!({"address":source.address,"deposed_key":source.deposed_key})).collect();
        serde_json::json!({"id":entity.id.as_str(), "kind":match entity.kind { ArchitectureEntityKind::Terraform => "terraform", ArchitectureEntityKind::Synthetic => "synthetic" }, "sources": sources})
    }).collect();
    let relationships: Vec<_> = graph.relationships.iter().map(|relationship| {
        let provenance: Vec<_> = relationship.provenance.iter().map(|source| match source {
            RelationshipProvenance::Resource { source, change } => serde_json::json!({"kind":"resource", "address":source.address, "deposed_key":source.deposed_key, "action":super::label(change.action), "operation":super::operations::label(change.action,&change.metadata), "metadata":super::operations::metadata(&change.metadata), "previous_address":change.previous_address}),
            RelationshipProvenance::Reference { from, to } => serde_json::json!({"kind":"reference", "from":from.address, "from_deposed_key":from.deposed_key, "to":to.address, "to_deposed_key":to.deposed_key}),
        }).collect();
        serde_json::json!({"from":relationship.from.as_str(), "to":relationship.to.as_str(), "kind":format!("{:?}",relationship.kind).to_lowercase(), "directionality":relationship.directionality().as_str(), "inferred":relationship.inferred, "provenance":provenance})
    }).collect();
    let references: Vec<_> = graph.nodes.iter().flat_map(|node| node.references.iter().map(move |reference| {
        serde_json::json!({"consumer":node.entity.id.as_str(), "attribute":reference.attribute, "dependency":reference.dependency, "complete":reference.complete, "targets":reference.targets.iter().map(reference_target).collect::<Vec<_>>(), "issues":reference.issues.iter().map(|issue| format!("{issue:?}")).collect::<Vec<_>>()})
    })).collect();
    let reference_metadata = if references.is_empty() {
        String::new()
    } else {
        format!(
            "<metadata id=\"resource-references\">{}</metadata>\n",
            super::escape(&serde_json::Value::Array(references).to_string())
        )
    };
    let routes: Vec<_> = graph.nodes.iter().filter(|n| !n.routes.entries.is_empty() || n.routes.owner.is_some()).map(|node| {
        serde_json::json!({"source":node.entity.id.as_str(), "owner":node.routes.owner.as_ref().map(|id| &id.address), "inline":node.routes.inline,
            "entries":node.routes.entries.iter().map(|entry| serde_json::json!({"destination":entry.destination,"target":entry.target.as_ref().map(|id| &id.address),"target_kind":entry.target_kind})).collect::<Vec<_>>()})
    }).collect();
    let route_metadata = if routes.is_empty() {
        String::new()
    } else {
        format!(
            "<metadata id=\"route-presentations\">{}</metadata>\n",
            super::escape(&serde_json::Value::Array(routes).to_string())
        )
    };
    format!(
        "<metadata id=\"architecture-entities\">{}</metadata>\n<metadata id=\"architecture-relationships\">{}</metadata>\n{reference_metadata}{route_metadata}",
        super::escape(&serde_json::Value::Array(entities).to_string()),
        super::escape(&serde_json::Value::Array(relationships).to_string())
    )
}
