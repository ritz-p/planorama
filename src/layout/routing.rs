use super::{NODE_HEIGHT, NODE_WIDTH, Point};
use crate::model::Graph;
use std::cmp::Ordering;

pub(super) fn route(
    graph: &Graph,
    ranks: &[usize],
    positions: &[Point],
    channels: &[usize],
) -> Vec<Vec<Point>> {
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
    graph
        .edges
        .iter()
        .enumerate()
        .map(|(edge, &(a, b))| {
            let source = positions[a];
            let target = positions[b];
            let start = Point {
                x: source.x + NODE_WIDTH,
                y: source.y + source_ports[edge],
            };
            let end = Point {
                x: match ranks[b].cmp(&ranks[a]) {
                    Ordering::Greater => target.x,
                    Ordering::Equal | Ordering::Less => target.x + NODE_WIDTH,
                },
                y: target.y + target_ports[edge],
            };
            let lane = 24 + (edge % 5) * 10;
            let route = match ranks[b].checked_sub(ranks[a]) {
                Some(0 | 1) => {
                    let x = start.x + lane;
                    vec![start, Point { x, y: start.y }, Point { x, y: end.y }, end]
                }
                _ => {
                    let channel = (0..channels.len())
                        .min_by_key(|&i| {
                            (
                                start.y.abs_diff(channels[i])
                                    + end.y.abs_diff(channels[i])
                                    + channel_uses[i] * 16,
                                i,
                            )
                        })
                        .unwrap();
                    channel_uses[channel] += 1;
                    let y = channels[channel];
                    let left = start.x + lane;
                    let right = match ranks[b].cmp(&ranks[a]) {
                        Ordering::Greater => end.x - 24 - edge % 5 * 10,
                        Ordering::Equal | Ordering::Less => end.x + lane,
                    };
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
            };
            simplify(route)
        })
        .collect()
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
