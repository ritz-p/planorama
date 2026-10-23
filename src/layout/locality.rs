use super::{Bounds, Layout, Point};
use crate::model::{EdgeKind, Graph, ResourceRole};
use std::collections::BTreeSet;

#[cfg(test)]
#[path = "../../tests/unit/layout/locality.rs"]
mod tests;

fn leaves(graph: &Graph) -> Vec<(usize, usize)> {
    let mut neighbors = vec![BTreeSet::new(); graph.nodes.len()];
    for edge in &graph.edges {
        if edge.kind != EdgeKind::Containment && edge.from != edge.to {
            neighbors[edge.from].insert(edge.to);
            neighbors[edge.to].insert(edge.from);
        }
    }
    neighbors
        .iter()
        .enumerate()
        .filter_map(|(node, peers)| {
            if peers.len() != 1 || graph.nodes[node].role == ResourceRole::Container {
                return None;
            }
            let peer = *peers.first()?;
            if neighbors[peer].len() == 1
                && graph.nodes[node].entity.id < graph.nodes[peer].entity.id
                && graph.nodes[peer].role != ResourceRole::Container
            {
                return None;
            }
            Some((node, peer))
        })
        .collect()
}

#[cfg(test)]
pub(super) fn refine(graph: &Graph, layout: &mut Layout<'_>) {
    crate::work::complete(refine_step(graph, layout))
}

pub(super) async fn refine_step(graph: &Graph, layout: &mut Layout<'_>) {
    crate::work::checkpoint().await;
    let mut preferences = crate::provider::aws::owner_affinities(graph);
    preferences.sort_by_key(|&(node, owner)| {
        (&graph.nodes[owner].entity.id, &graph.nodes[node].entity.id)
    });
    let owners: BTreeSet<_> = preferences.iter().map(|&(_, owner)| owner).collect();
    let explicit: BTreeSet<_> = preferences
        .iter()
        .flat_map(|&(node, owner)| [node, owner])
        .collect();
    let mut leaf_preferences = leaves(graph);
    leaf_preferences
        .sort_by_key(|&(node, peer)| (&graph.nodes[peer].entity.id, &graph.nodes[node].entity.id));
    preferences.extend(
        leaf_preferences
            .into_iter()
            .filter(|(node, _)| !explicit.contains(node)),
    );
    let protected: BTreeSet<_> =
        super::containers::affinity::groups(graph, &layout.containment.parents)
            .into_iter()
            .flat_map(|g| g.sources.into_iter().chain([g.target]))
            .collect();
    let padding = super::relationship_markers::padding(graph);
    let mut peers = vec![Vec::new(); graph.nodes.len()];
    for edge in &graph.edges {
        crate::work::checkpoint().await;
        if edge.kind != EdgeKind::Containment && edge.from != edge.to {
            peers[edge.from].push(edge.to);
            peers[edge.to].push(edge.from);
        }
    }
    for (node, owner) in preferences {
        crate::work::checkpoint().await;
        if graph.nodes[node].role == ResourceRole::Container
            || !layout.containment.children[node].is_empty()
            || protected.contains(&node)
        {
            continue;
        }
        let original = layout.bounds[node];
        let target = layout.bounds[owner];
        let Some(region) = region(graph, layout, node, padding) else {
            continue;
        };
        if region.width < original.width || region.height < original.height {
            continue;
        }
        let min_x = region.origin.x;
        let max_x = region.right() - original.width;
        let min_y = region.origin.y;
        let max_y = region.origin.y + region.height - original.height;
        let obstacles: Vec<_> = layout
            .bounds
            .iter()
            .enumerate()
            .filter(|&(other, _)| other != node && !layout.containment.is_ancestor(other, node))
            .map(|(_, &bounds)| bounds)
            .collect();
        let mut xs = BTreeSet::from([original.origin.x, min_x, max_x]);
        let mut ys = BTreeSet::from([original.origin.y, min_y, max_y]);
        for bounds in obstacles.iter().chain([&target]) {
            crate::work::checkpoint().await;
            xs.extend([
                bounds.origin.x.saturating_sub(original.width + padding),
                bounds.right() + padding,
                (bounds.origin.x + bounds.width / 2).saturating_sub(original.width / 2),
            ]);
            ys.extend([
                bounds.origin.y.saturating_sub(original.height + padding),
                bounds.origin.y + bounds.height + padding,
                (bounds.origin.y + bounds.height / 2).saturating_sub(original.height / 2),
            ]);
        }
        let axes = |values: BTreeSet<usize>, min, max, desired: usize| {
            let mut values: Vec<_> = values.into_iter().map(|v| v.clamp(min, max)).collect();
            values.sort_by_key(|&v| (v.abs_diff(desired), v));
            values.dedup();
            values.truncate(32);
            values
        };
        let xs = if !explicit.contains(&node) && !layout.bands.is_empty() {
            vec![original.origin.x]
        } else {
            axes(xs, min_x, max_x, target.origin.x)
        };
        let ys = axes(ys, min_y, max_y, target.origin.y);
        let score = |candidate: Bounds| {
            let relationship: u128 = peers[node]
                .iter()
                .map(|&peer| distance(candidate, layout.bounds[peer]))
                .sum();
            let owner_distance = distance(candidate, target);
            let intrusion = owners
                .iter()
                .filter(|&&other| explicit.contains(&node) && other != owner)
                .map(|&other| {
                    owner_distance.saturating_sub(distance(candidate, layout.bounds[other]))
                })
                .max()
                .unwrap_or(0);
            owner_distance * if explicit.contains(&node) { 4 } else { 1 }
                + intrusion * 4
                + relationship
        };
        let mut best = (score(original), original.origin.x, original.origin.y);
        for x in xs {
            crate::work::checkpoint().await;
            for &y in &ys {
                crate::work::checkpoint().await;
                let candidate = Bounds {
                    origin: Point { x, y },
                    ..original
                };
                if obstacles
                    .iter()
                    .any(|&other| !separated(candidate, other, padding))
                {
                    continue;
                }
                let next = (score(candidate), x, y);
                if next.0 < best.0 {
                    best = next;
                }
            }
        }
        layout.bounds[node].origin = Point {
            x: best.1,
            y: best.2,
        };
        layout.positions[node] = layout.bounds[node].origin;
    }
}

fn distance(a: Bounds, b: Bounds) -> u128 {
    (a.origin.x + a.width / 2).abs_diff(b.origin.x + b.width / 2) as u128
        + (a.origin.y + a.height / 2).abs_diff(b.origin.y + b.height / 2) as u128
}

fn separated(a: Bounds, b: Bounds, padding: usize) -> bool {
    a.right() + padding <= b.origin.x
        || b.right() + padding <= a.origin.x
        || a.origin.y + a.height + padding <= b.origin.y
        || b.origin.y + b.height + padding <= a.origin.y
}

fn region(graph: &Graph, layout: &Layout<'_>, node: usize, padding: usize) -> Option<Bounds> {
    if let Some(parent) = layout.containment.parents[node] {
        let bounds = layout.bounds[parent];
        return Some(Bounds {
            origin: Point {
                x: bounds.origin.x + padding,
                y: bounds.origin.y + layout.header_heights[parent] + padding,
            },
            width: bounds.width.saturating_sub(padding * 2),
            height: bounds
                .height
                .saturating_sub(layout.header_heights[parent] + padding * 2),
        });
    }
    let current = layout.bounds[node];
    if !layout.scopes.is_empty() {
        let panel = layout.scopes.iter().find(|panel| {
            !panel.account_panel && graph.nodes[node].entity.scope.as_ref() == Some(&panel.scope)
        })?;
        if current.origin.y < panel.bounds.origin.y
            || current.origin.y + current.height > panel.bounds.origin.y + panel.bounds.height
        {
            return None;
        }
        return Some(Bounds {
            origin: Point {
                x: panel.bounds.origin.x + 20,
                y: panel.bounds.origin.y + 40,
            },
            width: panel.bounds.width.saturating_sub(40),
            height: panel.bounds.height.saturating_sub(60),
        });
    }
    let bottom = layout.bounds.iter().map(|b| b.origin.y + b.height).max()?;
    let right = layout.bounds.iter().map(|b| b.right()).max()?;
    let mut bounds = Bounds {
        origin: Point { x: 60, y: 160 },
        width: right.saturating_sub(60),
        height: bottom.saturating_sub(160),
    };
    if let Some(band) = layout
        .bands
        .iter()
        .find(|band| band.label == graph.nodes[node].module)
    {
        bounds.origin.y = band.top + 48;
        bounds.height = band.height.saturating_sub(64);
    }
    (current.origin.x >= bounds.origin.x && current.origin.y >= bounds.origin.y).then_some(bounds)
}
