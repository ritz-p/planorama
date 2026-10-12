use super::{Bounds, Layout, Point, Scorer, Side, obstacles};
use crate::layout::containers::affinity::Group;
use crate::layout::routing_shared::{Simplification, ports, simplify};
use crate::model::architecture::{EdgeKind, Graph};

pub(super) fn dependencies(graph: &Graph, layout: &Layout<'_>) -> Vec<Group> {
    use std::collections::{BTreeMap, BTreeSet};
    let numbered = crate::layout::resource_edges(graph);
    let mut buckets: BTreeMap<_, Vec<usize>> = BTreeMap::new();
    for reverse in [true, false] {
        for (index, edge) in graph.edges.iter().enumerate() {
            if edge.kind != EdgeKind::Dependency
                || edge.change.is_some()
                || numbered[index]
                || edge.from == edge.to
                || super::represented_by_nesting(edge, &layout.containment)
            {
                continue;
            }
            let (shared, peer) = if reverse {
                (edge.from, edge.to)
            } else {
                (edge.to, edge.from)
            };
            buckets
                .entry((reverse, shared, layout.containment.parents[peer]))
                .or_default()
                .push(index);
        }
    }
    buckets
        .into_iter()
        .filter_map(|((reverse, target, _), mut edges)| {
            edges.sort_by_key(|&i| {
                let e = &graph.edges[i];
                (
                    graph.nodes[e.from].entity.id.as_str(),
                    graph.nodes[e.to].entity.id.as_str(),
                )
            });
            let sources: BTreeSet<_> = edges
                .iter()
                .map(|&i| {
                    if reverse {
                        graph.edges[i].to
                    } else {
                        graph.edges[i].from
                    }
                })
                .collect();
            (sources.len() > 1 && sources.len() == edges.len()).then(|| Group {
                reverse,
                target,
                sources: sources.into_iter().collect(),
                edges,
            })
        })
        .collect()
}

pub(super) struct Bundle {
    pub paths: Vec<(usize, Vec<Point>)>,
    pub junctions: Vec<Point>,
}

pub(super) fn try_bundle(
    graph: &Graph,
    layout: &Layout<'_>,
    group: &Group,
    offsets: (&[[usize; 4]], &[[usize; 4]]),
    scorer: &mut Scorer,
    reservations: &[Bounds],
    independent: impl Fn(usize, &Scorer) -> Vec<Point>,
) -> Option<Bundle> {
    let target = layout.bounds[group.target];
    let first = *group.sources.first()?;
    let pairs = ports::facing_pairs(layout.bounds[first], target);
    let mut independent_cost = None;
    let mut best = None;
    for pair in pairs {
        if !group
            .sources
            .iter()
            .all(|&source| ports::facing_pairs(layout.bounds[source], target).contains(&pair))
        {
            continue;
        }
        for clearance in [16, 24, 8] {
            if let Some((bundle, cost)) = candidate(
                graph,
                layout,
                group,
                offsets,
                scorer,
                reservations,
                (pair, clearance),
            ) {
                let independent_cost = *independent_cost.get_or_insert_with(|| {
                    self::independent_cost(graph, layout, group, scorer, reservations, &independent)
                });
                if cost <= independent_cost && best.as_ref().is_none_or(|(_, old)| cost < *old) {
                    best = Some((bundle, cost));
                }
            }
        }
    }
    best.map(|(bundle, _)| bundle)
}

fn independent_cost(
    graph: &Graph,
    layout: &Layout<'_>,
    group: &Group,
    scorer: &mut Scorer,
    reservations: &[Bounds],
    independent: impl Fn(usize, &Scorer) -> Vec<Point>,
) -> u128 {
    let mut cost = 0;
    let checkpoint = scorer.len();
    let peers = scorer.peers().to_vec();
    let soft = scorer.soft().to_vec();
    let kind = scorer.kind();
    let mut paths = Vec::new();
    let mut order = group.edges.clone();
    order.sort_by_key(|&index| {
        let edge = &graph.edges[index];
        let bounds = layout.bounds[edge.from];
        (
            layout.containment.keys[edge.from],
            layout.containment.keys[edge.to],
            bounds.origin.x,
            bounds.origin.y,
            bounds.width,
            bounds.height,
            graph.nodes[edge.from].entity.id.as_str(),
            graph.nodes[edge.to].entity.id.as_str(),
        )
    });
    for &index in &order {
        let edge = &graph.edges[index];
        scorer.set_kind(edge.kind);
        scorer.set_soft(super::soft_obstacles(layout, edge));
        scorer.set_peers(
            layout
                .containment
                .routing_peers(edge.from, edge.to, &layout.bounds),
        );
        let path = independent(index, scorer);
        scorer.insert(path.clone());
        paths.push(path);
    }
    for (slot, &index) in order.iter().enumerate() {
        scorer.set_kind(graph.edges[index].kind);
        scorer.set_soft(super::soft_obstacles(layout, &graph.edges[index]));
        let mut barriers = obstacles(layout, &graph.edges[index]);
        barriers.extend_from_slice(reservations);
        scorer.remove(checkpoint + slot);
        paths[slot] = crate::layout::routing_shared::shortcuts::simplify_path(
            std::mem::take(&mut paths[slot]),
            &barriers,
            scorer,
        );
        scorer.replace(checkpoint + slot, paths[slot].clone());
    }
    scorer.truncate(checkpoint);
    for (index, path) in order.into_iter().zip(paths) {
        scorer.set_kind(graph.edges[index].kind);
        scorer.set_soft(super::soft_obstacles(layout, &graph.edges[index]));
        cost += scorer.readability_cost(&path);
        scorer.insert(path);
    }
    scorer.truncate(checkpoint);
    scorer.set_peers(peers);
    scorer.set_soft(soft);
    scorer.set_kind(kind);
    cost
}

fn candidate(
    graph: &Graph,
    layout: &Layout<'_>,
    group: &Group,
    offsets: (&[[usize; 4]], &[[usize; 4]]),
    scorer: &Scorer,
    reservations: &[Bounds],
    choice: ((Side, Side), usize),
) -> Option<(Bundle, u128)> {
    let ((source_side, target_side), clearance) = choice;
    let target = layout.bounds[group.target];
    let end = target.port(
        target_side,
        group
            .edges
            .iter()
            .map(|&i| {
                if group.reverse {
                    offsets.0[i][target_side as usize]
                } else {
                    offsets.1[i][target_side as usize]
                }
            })
            .min()?,
    );
    let trunk = end.outward(clearance);
    if trunk == end.point {
        return None;
    }
    let horizontal = matches!(source_side, Side::Left | Side::Right);
    let along = |p: Point| if horizontal { p.y } else { p.x };
    let mut paths = Vec::new();
    let mut coordinates = vec![along(end.point)];
    let mut bundled_cost = 0;
    for &index in &group.edges {
        let edge = &graph.edges[index];
        let node = if group.reverse { edge.to } else { edge.from };
        let slots = if group.reverse { offsets.1 } else { offsets.0 };
        let start = layout.bounds[node].port(source_side, slots[index][source_side as usize]);
        let branch = if horizontal {
            Point {
                x: trunk.x,
                y: start.point.y,
            }
        } else {
            Point {
                x: start.point.x,
                y: trunk.y,
            }
        };
        let mut path = simplify(
            [start.point, branch, trunk, end.point],
            Simplification::PreserveReversals,
        );
        let mut barriers = obstacles(layout, edge);
        barriers.extend_from_slice(reservations);
        if !ports::valid_with_clearance(&path, start, end, &barriers, clearance.min(16))
            || scorer.overlaps(&path)
        {
            return None;
        }
        if group.reverse {
            path.reverse();
        }
        bundled_cost +=
            scorer.readability_cost_for(&path, &super::soft_obstacles(layout, edge), edge.kind);
        coordinates.push(along(start.point));
        paths.push((index, path));
    }
    coordinates.sort_unstable();
    coordinates.dedup();
    if group
        .edges
        .iter()
        .all(|&i| graph.edges[i].kind == EdgeKind::Dependency)
    {
        bundled_cost = bundled_cost.saturating_sub(
            (group.edges.len() - 1) as u128 * crate::layout::routing_shared::scoring::BEND_COST,
        );
    }
    let junctions = coordinates
        .iter()
        .skip(1)
        .take(coordinates.len().saturating_sub(2))
        .map(|&coordinate| {
            if horizontal {
                Point {
                    x: trunk.x,
                    y: coordinate,
                }
            } else {
                Point {
                    x: coordinate,
                    y: trunk.y,
                }
            }
        })
        .collect();
    Some((Bundle { paths, junctions }, bundled_cost))
}

#[cfg(test)]
#[path = "../../../../tests/unit/layout/containers/routing/bundles.rs"]
mod tests;
