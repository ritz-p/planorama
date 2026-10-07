use crate::model::{EdgeKind, EntityMode, Graph, ResourceRole, TerraformGraph};
use std::collections::BTreeSet;
mod indirect;
pub(super) use indirect::resolved_subnets;
mod spanning;

pub(super) fn direct_rule(resource_type: &str) -> Option<(&'static str, &'static str)> {
    match resource_type {
        "aws_subnet" | "aws_vpc_endpoint" | "aws_route_table" | "aws_security_group"
        | "aws_network_acl" => Some(("vpc_id", "aws_vpc")),
        "aws_instance" | "aws_nat_gateway" => Some(("subnet_id", "aws_subnet")),
        _ => None,
    }
}

#[cfg(test)]
#[path = "../../tests/unit/semantic/containment.rs"]
mod tests;

pub(super) fn infer(raw: &TerraformGraph) -> Graph {
    let mut relationships = BTreeSet::new();
    for reference in &raw.attributes {
        let child = &raw.nodes[reference.target];
        if child.mode != EntityMode::Managed
            || !child.provider.is_aws()
            || !reference.complete
            || reference
                .issues
                .contains(&crate::model::DiagnosticReason::DynamicInstanceSelection)
        {
            continue;
        }
        let Some((attribute, parent_type)) = direct_rule(&child.resource_type) else {
            continue;
        };
        if reference.attribute != attribute {
            continue;
        }
        if let [parent] = reference.sources.as_slice() {
            let node = &raw.nodes[*parent];
            if *parent != reference.target
                && node.resource_type == parent_type
                && node.provider.is_aws()
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
    indirect::infer(raw, spanning::infer(raw, graph))
}
