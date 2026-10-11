use crate::model::{EntityMode, Graph};

#[cfg(test)]
#[path = "../../../tests/unit/semantic/data.rs"]
mod tests;

pub(super) fn visible(mut graph: Graph) -> Graph {
    let mut indices = vec![None; graph.nodes.len()];
    let mut next = 0;
    for (index, node) in graph.nodes.iter().enumerate() {
        let metadata = node.mode == EntityMode::Data
            && node.provider.is_aws()
            && matches!(
                node.resource_type.as_str(),
                "aws_region"
                    | "aws_partition"
                    | "aws_caller_identity"
                    | "aws_availability_zones"
                    | "aws_iam_policy_document"
            );
        if !metadata {
            indices[index] = Some(next);
            next += 1;
        }
    }
    let omitted: std::collections::BTreeSet<_> = graph
        .nodes
        .iter()
        .enumerate()
        .filter(|(index, _)| indices[*index].is_none())
        .flat_map(|(_, node)| node.entity.provenance.iter().cloned())
        .collect();
    for node in &mut graph.nodes {
        for reference in &mut node.references {
            reference
                .targets
                .retain(|target| !omitted.contains(&target.id));
        }
        node.references
            .retain(|r| !r.targets.is_empty() || !r.complete);
    }
    graph.nodes = graph
        .nodes
        .into_iter()
        .enumerate()
        .filter_map(|(index, node)| indices[index].map(|_| node))
        .collect();
    graph.edges = graph
        .edges
        .into_iter()
        .filter_map(|mut edge| {
            let (from, to) = indices[edge.from].zip(indices[edge.to])?;
            edge.from = from;
            edge.to = to;
            Some(edge)
        })
        .collect();
    graph
}
