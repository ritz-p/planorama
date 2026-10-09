use crate::model::{ArchitectureEntityKind, Graph};

pub(super) fn metadata(graph: &Graph) -> String {
    let entities: Vec<_> = graph.entities().map(|entity| {
        let sources: Vec<_> = entity.provenance.iter().map(|source| serde_json::json!({"address":source.address,"deposed_key":source.deposed_key})).collect();
        serde_json::json!({"id":entity.id.as_str(), "kind":match entity.kind { ArchitectureEntityKind::Terraform => "terraform", ArchitectureEntityKind::Synthetic => "synthetic" }, "sources": sources})
    }).collect();
    format!(
        "<metadata id=\"architecture-entities\">{}</metadata>\n",
        super::escape(&serde_json::Value::Array(entities).to_string())
    )
}
