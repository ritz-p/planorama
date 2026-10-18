use crate::model::{Graph, Node, Placement};

#[cfg(test)]
#[path = "../../tests/unit/presentation.rs"]
mod tests;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Effective {
    Independent,
    Logical,
    Embedded { owner: usize },
}

pub(crate) fn effective(graph: &Graph, node: &Node) -> Effective {
    if node.placement == Placement::Logical {
        return Effective::Logical;
    }
    owner(graph, node).map_or(Effective::Independent, |owner| Effective::Embedded {
        owner,
    })
}

pub(crate) fn owner(graph: &Graph, node: &Node) -> Option<usize> {
    if node.placement == Placement::Logical {
        return None;
    }
    let identity = match (&node.routes.owner, &node.member_owner) {
        (Some(owner), None) => owner,
        (None, Some(owner)) => &owner.target,
        _ => return None,
    };
    let mut candidates = graph
        .nodes
        .iter()
        .enumerate()
        .filter(|(_, candidate)| candidate.entity.provenance.contains(identity));
    let (index, parent) = candidates.next()?;
    if candidates.next().is_some()
        || parent.entity.id == node.entity.id
        || parent.placement == Placement::Logical
        || parent.routes.owner.is_some()
        || parent.member_owner.is_some()
    {
        return None;
    }
    if node.member_owner.is_some()
        && (node.deposed_key.is_some()
            || parent.deposed_key.is_some()
            || !node.presentation_scope_matches(parent))
    {
        return None;
    }
    Some(index)
}
