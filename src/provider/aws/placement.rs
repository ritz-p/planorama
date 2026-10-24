use crate::model::{EntityMode, Graph};

pub(crate) fn owner_affinities(graph: &Graph) -> Vec<(usize, usize)> {
    let eligible = |index: usize, kind: &str| {
        let node = &graph.nodes[index];
        node.provider.is_aws()
            && super::aliases::node_type(node) == kind
            && node.mode == EntityMode::Managed
            && node.deposed_key.is_none()
            && graph
                .nodes
                .iter()
                .filter(|n| n.address == node.address)
                .count()
                == 1
    };
    graph
        .nodes
        .iter()
        .enumerate()
        .filter_map(|(index, node)| {
            if !eligible(index, "aws_lb_listener") {
                return None;
            }
            let mut references = node
                .references
                .iter()
                .filter(|r| !r.dependency && r.attribute == "load_balancer_arn");
            let reference = references.next()?;
            if references.next().is_some() || !reference.complete || !reference.issues.is_empty() {
                return None;
            }
            let [target] = reference.targets.as_slice() else {
                return None;
            };
            if target.state.is_some() {
                return None;
            }
            let owner = graph.nodes.iter().position(|n| {
                n.address == target.id.address && n.deposed_key == target.id.deposed_key
            })?;
            eligible(owner, "aws_lb").then_some((index, owner))
        })
        .collect()
}
