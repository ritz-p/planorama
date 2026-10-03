use super::{NODE_HEIGHT, NODE_WIDTH, Point};
use crate::model::Graph;
use std::cmp::Ordering;
mod bundles;
mod candidates;
mod coverage;
mod lanes;
mod scoring;
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
    positions: &mut [Point],
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
    for edges in &mut sources {
        edges.sort_by_key(|&edge| (positions[graph.edges[edge].to].y, edge));
        for (port, &edge) in edges.iter().enumerate() {
            source_ports[edge] = 20 + (port + 1) * (NODE_HEIGHT - 40) / (edges.len() + 1);
        }
    }
    for edges in &mut targets {
        edges.sort_by_key(|&edge| (positions[graph.edges[edge].from].y, edge));
        for (port, &edge) in edges.iter().enumerate() {
            target_ports[edge] = 20 + (port + 1) * (NODE_HEIGHT - 40) / (edges.len() + 1);
        }
    }

    let bundles = Bundles::new(graph, ranks);
    bundles.align_ports(&mut source_ports, &mut target_ports);
    let columns = ranks.iter().max().copied().unwrap_or(0) + 1;
    let mut gutter_widths = vec![100; columns];
    loop {
        let mut column_x = Vec::with_capacity(columns);
        let mut x = 60;
        for &width in &gutter_widths {
            column_x.push(x);
            x += NODE_WIDTH + width;
        }
        for (index, position) in positions.iter_mut().enumerate() {
            position.x = column_x[ranks[index]];
        }
        let mut scorer = Scorer::default();
        let mut lanes = VerticalLanes::default();
        let mut bundle_lanes = vec![None; bundles.len()];
        let mut paths = Vec::with_capacity(graph.edges.len());
        for (edge, link) in graph.edges.iter().enumerate() {
            let (a, b) = link.endpoints();
            let start = Point {
                x: positions[a].x + NODE_WIDTH,
                y: positions[a].y + source_ports[edge],
            };
            let end = Point {
                x: match ranks[b].cmp(&ranks[a]) {
                    Ordering::Greater => positions[b].x,
                    Ordering::Equal | Ordering::Less => positions[b].x + NODE_WIDTH,
                },
                y: positions[b].y + target_ports[edge],
            };
            let candidates: Vec<_> = match ranks[b].checked_sub(ranks[a]) {
                Some(0 | 1) => match bundles.group(edge) {
                    Some(group) => {
                        let lane = *bundle_lanes[group].get_or_insert_with(|| {
                            let (from, to) =
                                bundles.span(group, graph, positions, &source_ports, &target_ports);
                            let lane = lanes
                                .candidates(ranks[a], from, to, 3)
                                .into_iter()
                                .min_by_key(|lane| {
                                    let x = column_x[lane.gutter] + NODE_WIDTH + lane.offset();
                                    scorer.score(
                                        &[Point { x, y: from }, Point { x, y: to }],
                                        positions,
                                    )
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
                .map(|lane| RouteCandidate::direct(start, end, lane, &column_x))
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
                                &column_x,
                            )
                        })
                        .collect()
                }
            };
            let chosen = candidates
                .into_iter()
                .min_by_key(|candidate| scorer.score(&candidate.points, positions))
                .expect("routing requires a route candidate");
            if bundles.group(edge).is_none() {
                chosen.reserve(&mut lanes);
            }
            scorer.insert(chosen.points.clone());
            paths.push(chosen.points);
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
    let mut result: Vec<Point> = Vec::new();
    for point in points {
        if result.last() == Some(&point) {
            continue;
        }
        loop {
            match result.as_slice() {
                [.., a, b] if (a.x == b.x && b.x == point.x) || (a.y == b.y && b.y == point.y) => {
                    result.pop();
                }
                _ => break,
            }
        }
        result.push(point);
    }
    result
}
