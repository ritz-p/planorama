use crate::model::architecture::{ArchitectureEntityKind, Graph, RelationshipProvenance};

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
        serde_json::json!({"consumer":node.entity.id.as_str(), "attribute":reference.attribute, "dependency":reference.dependency, "complete":reference.complete, "targets":reference.targets.iter().map(|target| serde_json::json!({"address":target.id.address,"deposed_key":target.id.deposed_key})).collect::<Vec<_>>(), "issues":reference.issues.iter().map(|issue| format!("{issue:?}")).collect::<Vec<_>>()})
    })).collect();
    let reference_metadata = if references.is_empty() {
        String::new()
    } else {
        format!(
            "<metadata id=\"resource-references\">{}</metadata>\n",
            super::escape(&serde_json::Value::Array(references).to_string())
        )
    };
    format!(
        "<metadata id=\"architecture-entities\">{}</metadata>\n<metadata id=\"architecture-relationships\">{}</metadata>\n{reference_metadata}",
        super::escape(&serde_json::Value::Array(entities).to_string()),
        super::escape(&serde_json::Value::Array(relationships).to_string())
    )
}
