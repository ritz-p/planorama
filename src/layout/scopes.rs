use super::{Bounds, ContainmentTree, Point, ScopePanel, containers::placement};
use crate::model::architecture::{DeploymentScope, Graph};
use std::collections::{BTreeMap, BTreeSet};

#[cfg(test)]
#[path = "../../tests/unit/layout/scopes.rs"]
mod tests;

pub(super) fn pack(
    graph: &Graph,
    tree: &ContainmentTree,
    sizes: &[(usize, usize)],
    offsets: &mut [Point],
) -> (usize, Vec<ScopePanel>) {
    let spacing = super::relationship_markers::padding(graph);
    let protected = super::containers::affinity::protected_roots(graph, &tree.parents);
    let mut groups: BTreeMap<Option<DeploymentScope>, Vec<usize>> = BTreeMap::new();
    for &root in &tree.roots {
        let scope = graph.nodes[root].entity.scope.as_ref();
        let mut pending = vec![root];
        let mut consistent = true;
        while let Some(node) = pending.pop() {
            consistent &= graph.nodes[node].entity.scope.as_ref() == scope;
            pending.extend(tree.children[node].iter().copied());
        }
        groups
            .entry(if consistent { scope.cloned() } else { None })
            .or_default()
            .push(root);
    }
    if !groups.keys().flatten().any(|scope| scope.region.is_some()) {
        let columns = placement::columns(graph, None, &tree.roots, &tree.parents);
        let (_, height) = placement::pack(&columns, sizes, offsets, spacing);
        let relationships = placement::relationships(graph, &tree.roots, &tree.parents);
        let movable = movable_roots(&tree.roots, &protected, offsets);
        placement::refine(&movable, &relationships, sizes, offsets, height, spacing);
        return (height, Vec::new());
    }
    let mut top = 0;
    let mut panels = Vec::new();
    for (scope, roots) in groups
        .iter()
        .filter(|(scope, _)| scope.is_some())
        .chain(groups.iter().filter(|(scope, _)| scope.is_none()))
    {
        let columns = placement::columns(graph, None, roots, &tree.parents);
        let (width, height) = placement::pack(&columns, sizes, offsets, spacing);
        let relationships = placement::relationships(graph, roots, &tree.parents);
        let movable = movable_roots(roots, &protected, offsets);
        placement::refine(&movable, &relationships, sizes, offsets, height, spacing);
        let padding = if scope.is_some() { 40 } else { 0 };
        for &root in roots {
            offsets[root].y += top + padding;
        }
        if let Some(scope) = scope {
            panels.push(ScopePanel {
                scope: scope.clone(),
                bounds: Bounds {
                    origin: Point {
                        x: 40,
                        y: 160 + top,
                    },
                    width: width + spacing.max(40),
                    height: height + 60,
                },
            });
        }
        top += height + padding + 60;
    }
    (top, panels)
}

fn movable_roots(roots: &[usize], protected: &BTreeSet<usize>, offsets: &[Point]) -> Vec<usize> {
    let columns: BTreeSet<_> = roots
        .iter()
        .filter(|node| protected.contains(node))
        .map(|&node| offsets[node].x)
        .collect();
    roots
        .iter()
        .copied()
        .filter(|&node| !columns.contains(&offsets[node].x))
        .collect()
}
