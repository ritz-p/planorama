use super::{Bounds, ContainmentTree, Point, ScopePanel, containers::placement};
use crate::model::architecture::{DeploymentScope, Graph};
use std::collections::{BTreeMap, BTreeSet};

#[cfg(test)]
#[path = "../../tests/unit/layout/accounts.rs"]
mod account_tests;
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
    if !groups
        .keys()
        .flatten()
        .any(|scope| scope.region.is_some() || scope.account.is_some())
    {
        let columns = placement::columns(graph, None, &tree.roots, &tree.parents);
        let relationships = placement::relationships(graph, &tree.roots, &tree.parents);
        let (_, height) = placement::pack(&columns, &relationships, sizes, offsets, spacing);
        let movable = movable_roots(&tree.roots, &protected, offsets);
        placement::refine(&movable, &relationships, sizes, offsets, height, spacing);
        return (height, Vec::new());
    }
    let mut top = 0;
    let mut panels = Vec::new();
    let accounts: BTreeSet<_> = groups
        .keys()
        .flatten()
        .filter_map(|s| s.account.clone())
        .collect();
    for account in accounts.iter().map(Some).chain(std::iter::once(None)) {
        let account_top = top;
        let account_index = panels.len();
        let mut account_width = 320;
        if let Some(account) = account {
            panels.push(ScopePanel {
                account_panel: true,
                scope: DeploymentScope {
                    provider: "AWS".into(),
                    region: None,
                    account: Some(account.clone()),
                    global: false,
                },
                bounds: Bounds {
                    origin: Point {
                        x: 40,
                        y: 160 + top,
                    },
                    width: 0,
                    height: 0,
                },
            });
            top += 40;
        }
        for (scope, roots) in groups
            .iter()
            .filter(|(scope, _)| scope.as_ref().and_then(|s| s.account.as_ref()) == account)
            .filter(|(scope, _)| scope.is_some())
            .chain(
                groups
                    .iter()
                    .filter(|(scope, _)| account.is_none() && scope.is_none()),
            )
        {
            let columns = placement::columns(graph, None, roots, &tree.parents);
            let relationships = placement::relationships(graph, roots, &tree.parents);
            let (width, height) =
                placement::pack(&columns, &relationships, sizes, offsets, spacing);
            let movable = movable_roots(roots, &protected, offsets);
            placement::refine(&movable, &relationships, sizes, offsets, height, spacing);
            let framed = scope
                .as_ref()
                .is_some_and(|s| s.global || s.region.is_some());
            let padding = if framed { 40 } else { 0 };
            let inset = if account.is_some() { 20 } else { 0 };
            account_width = account_width.max(width + spacing.max(40) + 40);
            for &root in roots {
                offsets[root].x += inset;
                offsets[root].y += top + padding;
            }
            if let Some(scope) = scope.as_ref().filter(|_| framed) {
                panels.push(ScopePanel {
                    account_panel: false,
                    scope: scope.clone(),
                    bounds: Bounds {
                        origin: Point {
                            x: 40 + inset,
                            y: 160 + top,
                        },
                        width: width + spacing.max(40),
                        height: height + 60,
                    },
                });
            }
            top += height + padding + 60;
        }
        if account.is_some() {
            panels[account_index].bounds.width = account_width;
            panels[account_index].bounds.height = top - account_top;
            top += 40;
        }
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
