use crate::model::{EdgeKind, EntityMode, Graph, ResourceRole, TerraformGraph};
use std::collections::BTreeSet;

#[cfg(test)]
#[path = "../../tests/unit/semantic/containment.rs"]
mod tests;

pub(super) fn infer(raw: &TerraformGraph) -> Graph {
    let mut relationships = BTreeSet::new();
    for reference in &raw.attributes {
        let child = &raw.nodes[reference.target];
        if child.mode != EntityMode::Managed || !reference.complete {
            continue;
        }
        let parent_type = match (child.resource_type.as_str(), reference.attribute.as_str()) {
            ("aws_subnet", "vpc_id") => "aws_vpc",
            ("aws_instance", "subnet_id") => "aws_subnet",
            _ => continue,
        };
        if let [parent] = reference.sources.as_slice() {
            let node = &raw.nodes[*parent];
            if *parent != reference.target
                && node.resource_type == parent_type
                && node.role == ResourceRole::Container
            {
                relationships.insert((*parent, reference.target));
            }
        }
    }
    let mut graph = raw.graph.clone();
    for edge in &mut graph.edges {
        if edge.kind == EdgeKind::Dependency && relationships.contains(&edge.endpoints()) {
            edge.kind = EdgeKind::Containment;
        }
    }
    graph
}
