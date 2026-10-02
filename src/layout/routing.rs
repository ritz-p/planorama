use super::{NODE_HEIGHT, NODE_WIDTH, Point};
use crate::model::Graph;
use std::cmp::Ordering;
mod lanes;
use lanes::{Lane, VerticalLanes};

#[cfg(test)]
#[path = "../../tests/unit/layout/routing.rs"]
mod tests;

pub(super) struct Routed {
    pub paths: Vec<Vec<Point>>,
    pub width: usize,
}

enum Route {
    Direct(Lane),
    Channel {
        source: Lane,
        target: Lane,
        y: usize,
    },
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
    for (edge, &(a, b)) in graph.edges.iter().enumerate() {
        sources[a].push(edge);
        targets[b].push(edge);
    }
    for edges in &mut sources {
        edges.sort_by_key(|&edge| (positions[graph.edges[edge].1].y, edge));
        for (port, &edge) in edges.iter().enumerate() {
            source_ports[edge] = 20 + (port + 1) * (NODE_HEIGHT - 40) / (edges.len() + 1);
        }
    }
    for edges in &mut targets {
        edges.sort_by_key(|&edge| (positions[graph.edges[edge].0].y, edge));
        for (port, &edge) in edges.iter().enumerate() {
            target_ports[edge] = 20 + (port + 1) * (NODE_HEIGHT - 40) / (edges.len() + 1);
        }
    }

    let mut channel_uses = vec![0usize; channels.len()];
    let mut lanes = VerticalLanes::default();
    let routes: Vec<_> = graph
        .edges
        .iter()
        .enumerate()
        .map(|(edge, &(a, b))| {
            let source_y = positions[a].y + source_ports[edge];
            let target_y = positions[b].y + target_ports[edge];
            match ranks[b].checked_sub(ranks[a]) {
                Some(0 | 1) => Route::Direct(lanes.allocate(ranks[a], source_y, target_y)),
                _ => {
                    let channel = (0..channels.len())
                        .min_by_key(|&i| {
                            (
                                source_y.abs_diff(channels[i])
                                    + target_y.abs_diff(channels[i])
                                    + channel_uses[i] * 16,
                                i,
                            )
                        })
                        .unwrap();
                    channel_uses[channel] += 1;
                    let y = channels[channel];
                    let gutter = match ranks[b].cmp(&ranks[a]) {
                        Ordering::Greater => ranks[b] - 1,
                        Ordering::Equal | Ordering::Less => ranks[b],
                    };
                    Route::Channel {
                        source: lanes.allocate(ranks[a], source_y, y),
                        target: lanes.allocate(gutter, y, target_y),
                        y,
                    }
                }
            }
        })
        .collect();
    let columns = ranks.iter().max().copied().unwrap_or(0) + 1;
    let mut column_x = Vec::with_capacity(columns);
    let mut x = 60;
    for column in 0..columns {
        column_x.push(x);
        x += NODE_WIDTH + lanes.width(column);
    }
    for (index, position) in positions.iter_mut().enumerate() {
        position.x = column_x[ranks[index]];
    }
    let lane_x = |lane: Lane| column_x[lane.gutter] + NODE_WIDTH + lane.offset();
    let paths = graph
        .edges
        .iter()
        .zip(routes)
        .enumerate()
        .map(|(edge, (&(a, b), route))| {
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
            simplify(match route {
                Route::Direct(lane) => {
                    let x = lane_x(lane);
                    vec![start, Point { x, y: start.y }, Point { x, y: end.y }, end]
                }
                Route::Channel { source, target, y } => {
                    let left = lane_x(source);
                    let right = lane_x(target);
                    vec![
                        start,
                        Point {
                            x: left,
                            y: start.y,
                        },
                        Point { x: left, y },
                        Point { x: right, y },
                        Point { x: right, y: end.y },
                        end,
                    ]
                }
            })
        })
        .collect();
    Routed {
        paths,
        width: (x + 20).max(1040),
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
