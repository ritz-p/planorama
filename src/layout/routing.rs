use super::{Bounds, Point, Side};
use crate::model::architecture::Graph;
use std::cmp::Ordering;
mod bundles;
mod candidates;

mod lanes;
use super::routing_shared::scoring;
use bundles::Bundles;
use candidates::RouteCandidate;
use lanes::VerticalLanes;
use scoring::Scorer;

#[cfg(test)]
#[path = "../../tests/unit/layout/routing.rs"]
mod tests;

pub(super) struct Routed {
    pub paths: Vec<Vec<Point>>,
    pub width: usize,
    pub junctions: Vec<Point>,
}

pub(super) fn route(
    graph: &Graph,
    ranks: &[usize],
    bounds: &mut [Bounds],
    channels: &[usize],
) -> Routed {
    let mut source_ports = vec![0; graph.edges.len()];
    let mut target_ports = vec![0; graph.edges.len()];
    let mut sources = vec![Vec::new(); graph.nodes.len()];
    let mut targets = vec![Vec::new(); graph.nodes.len()];
    for (edge, link) in graph.edges.iter().enumerate() {
        let (a, b) = link.endpoints();
        sources[a].push(edge);
        targets[b].push(edge);
    }
    for (node, edges) in sources.iter_mut().enumerate() {
        edges.sort_by_key(|&edge| {
            (
                bounds[graph.edges[edge].to].origin.y,
                graph.nodes[graph.edges[edge].to].entity.id.as_str(),
            )
        });
        for (port, &edge) in edges.iter().enumerate() {
            source_ports[edge] = 20 + (port + 1) * (bounds[node].height - 40) / (edges.len() + 1);
        }
    }
    for (node, edges) in targets.iter_mut().enumerate() {
        edges.sort_by_key(|&edge| {
            (
                bounds[graph.edges[edge].from].origin.y,
                graph.nodes[graph.edges[edge].from].entity.id.as_str(),
            )
        });
        for (port, &edge) in edges.iter().enumerate() {
            target_ports[edge] = 20 + (port + 1) * (bounds[node].height - 40) / (edges.len() + 1);
        }
    }

    let bundles = Bundles::new(graph, ranks);
    let containment = super::ContainmentTree::new(graph);
    bundles.align_ports(graph, bounds, &mut source_ports, &mut target_ports);
    let columns = ranks.iter().max().copied().unwrap_or(0) + 1;
    let mut column_widths = vec![0; columns];
    for (node, bound) in bounds.iter().enumerate() {
        column_widths[ranks[node]] = column_widths[ranks[node]].max(bound.width);
    }
    let mut gutter_widths = vec![100; columns];
    loop {
        let mut column_x = Vec::with_capacity(columns);
        let mut x = 60;
        for (column, &width) in gutter_widths.iter().enumerate() {
            column_x.push(x);
            x += column_widths[column] + width;
        }
        for (index, bounds) in bounds.iter_mut().enumerate() {
            bounds.origin.x = column_x[ranks[index]];
        }
        let column_right: Vec<_> = column_x
            .iter()
            .zip(&column_widths)
            .map(|(x, width)| x + width)
            .collect();
        let mut scorer = Scorer::default();
        let incident: Vec<_> = sources
            .iter()
            .zip(&targets)
            .map(|(s, t)| {
                s.iter()
                    .map(|&e| (e, true))
                    .chain(t.iter().map(|&e| (e, false)))
                    .collect()
            })
            .collect();
        let (source_slots, target_slots) =
            super::routing_shared::slots::assign(graph, bounds, &incident);
        let mut lanes = VerticalLanes::default();
        let mut bundle_lanes = vec![None; bundles.len()];
        let mut paths = Vec::with_capacity(graph.edges.len());
        for (edge, link) in graph.edges.iter().enumerate() {
            let (a, b) = link.endpoints();
            scorer.set_peers(containment.routing_peers(a, b, bounds));
            let start = bounds[a].port(Side::Right, source_ports[edge]).point;
            let end = bounds[b]
                .port(
                    if ranks[b] > ranks[a] {
                        Side::Left
                    } else {
                        Side::Right
                    },
                    target_ports[edge],
                )
                .point;
            let candidates: Vec<_> = match ranks[b].checked_sub(ranks[a]) {
                Some(0 | 1) => match bundles.group(edge) {
                    Some(group) => {
                        let lane = *bundle_lanes[group].get_or_insert_with(|| {
                            let (from, to) =
                                bundles.span(group, graph, bounds, &source_ports, &target_ports);
                            let lane = lanes
                                .candidates(ranks[a], from, to, 3)
                                .into_iter()
                                .min_by_key(|lane| {
                                    let x = column_right[lane.gutter] + lane.offset();
                                    scorer
                                        .score(&[Point { x, y: from }, Point { x, y: to }], bounds)
                                })
                                .expect("bundling requires a lane");
                            lanes.reserve(lane, from, to);
                            lane
                        });
                        vec![lane]
                    }
                    None => lanes.candidates(ranks[a], start.y, end.y, 3),
                }
                .into_iter()
                .map(|lane| RouteCandidate::direct(start, end, lane, &column_right))
                .collect(),
                _ => {
                    let gutter = match ranks[b].cmp(&ranks[a]) {
                        Ordering::Greater => ranks[b] - 1,
                        Ordering::Equal | Ordering::Less => ranks[b],
                    };
                    channels
                        .iter()
                        .map(|&y| {
                            RouteCandidate::channel(
                                start,
                                end,
                                lanes.preview(ranks[a], start.y, y),
                                lanes.preview(gutter, y, end.y),
                                y,
                                &column_right,
                            )
                        })
                        .collect()
                }
            };
            let chosen = candidates
                .into_iter()
                .min_by_key(|candidate| scorer.score(&candidate.points, bounds))
                .expect("routing requires a route candidate");
            if bundles.group(edge).is_none() {
                chosen.reserve(&mut lanes);
            }
            let points = if bundles.group(edge).is_none() {
                super::routing_shared::ports::select_with_slots(
                    bounds[a],
                    bounds[b],
                    chosen.points,
                    bounds,
                    &scorer,
                    Some((&source_slots[edge], &target_slots[edge])),
                )
            } else {
                chosen.points
            };
            scorer.insert(points.clone());
            paths.push(points);
        }
        let mut expanded = false;
        for (column, width) in gutter_widths.iter_mut().enumerate() {
            let needed = lanes.width(column);
            if needed > *width {
                *width = needed;
                expanded = true;
            }
        }
        match expanded {
            true => continue,
            false => {
                return Routed {
                    junctions: bundles.junctions(&paths),
                    paths,
                    width: (x + 20).max(1040),
                };
            }
        }
    }
}
fn simplify(points: Vec<Point>) -> Vec<Point> {
    super::routing_shared::simplify(
        points,
        super::routing_shared::Simplification::CollapseCollinear,
    )
}
