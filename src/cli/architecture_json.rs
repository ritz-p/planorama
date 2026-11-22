use crate::model::{EntityMode, Graph, cross_state::CrossStateEdge};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn export<'a>(
    states: impl IntoIterator<Item = (Option<&'a str>, &'a Graph)>,
    cross: &[CrossStateEdge],
) -> Result<String, String> {
    let mut resources = BTreeMap::new();
    let mut current = BTreeMap::new();
    let mut relationships = Vec::new();
    for (state, graph) in states {
        let endpoint = |id: &crate::model::ArchitectureId| json!({"state":state,"id":id.as_str()});
        for node in &graph.nodes {
            let id = endpoint(&node.entity.id);
            if node.deposed_key.is_none() {
                current.insert((state, node.address.as_str()), id.clone());
            }
            let scope = node.entity.scope.as_ref().map(|s| {
                json!({
                    "region":s.region.as_ref().map(|r| &r.0),
                    "account":s.account.as_ref().map(|a| a.as_str()), "global":s.global
                })
            });
            let cloudflare = node
                .entity
                .cloudflare_scope
                .as_ref()
                .map(|s| json!({"account":s.account,"zone":s.zone}));
            let mode = match node.mode {
                EntityMode::Managed => "managed",
                EntityMode::Data => "data",
            };
            resources.insert((state, &node.entity.id), json!({"identity":id,"kind":"resource",
                "address":node.address,"deposed":node.deposed_key,"type":node.resource_type,
                "provider":node.provider.source(),"mode":mode,"scope":scope,"cloudflare_scope":cloudflare}));
        }
        for component in &graph.components {
            resources.insert(
                (state, &component.entity.id),
                json!({"identity":endpoint(&component.entity.id),
                "kind":"component","type":component.kind,"members":component.members}),
            );
        }
        for edge in &graph.relationships {
            let kind = super::relationship_kind(edge.kind);
            let evidence: BTreeSet<_> = edge
                .provenance
                .iter()
                .map(|p| p.evidence().as_str())
                .collect();
            relationships.push(json!({"from":endpoint(&edge.from),"to":endpoint(&edge.to),
                "kind":kind,"direction":edge.directionality().as_str(),"evidence":evidence}));
        }
    }
    for edge in cross {
        let lookup = |e: &crate::model::cross_state::Endpoint| {
            current
                .get(&(Some(e.state.as_str()), e.address.as_str()))
                .cloned()
                .ok_or("missing cross-state endpoint")
        };
        relationships.push(json!({"from":lookup(&edge.from)?,"to":lookup(&edge.to)?,
            "kind":"dependency","direction":"directed","evidence":["remote-state"]}));
    }
    relationships.sort_by_cached_key(Value::to_string);
    relationships.dedup();
    serde_json::to_string_pretty(&json!({"schema_version":1,"resources":resources.into_values().collect::<Vec<_>>(),"relationships":relationships})).map_err(|e| e.to_string())
}
