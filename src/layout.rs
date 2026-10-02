use crate::graph::Graph;
use std::collections::BTreeMap;

pub const NODE_WIDTH: usize = 320;
pub const NODE_HEIGHT: usize = 96;
const COLUMN_STEP: usize = 420;
const ROW_STEP: usize = 144;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Point {
    pub x: usize,
    pub y: usize,
}

pub struct Band<'a> {
    pub label: &'a str,
    pub top: usize,
    pub height: usize,
}

pub struct Layout<'a> {
    pub width: usize,
    pub height: usize,
    pub positions: Vec<Point>,
    pub bands: Vec<Band<'a>>,
    /// Paths correspond one-to-one with Graph::edges.
    pub paths: Vec<Vec<Point>>,
}

struct Group<'a> {
    label: &'a str,
    columns: Vec<Vec<usize>>,
    top: usize,
    rows: usize,
}

fn update_positions(groups: &[Group<'_>], positions: &mut [Point]) {
    for group in groups {
        for (column, members) in group.columns.iter().enumerate() {
            for (row, &node) in members.iter().enumerate() {
                positions[node] = Point {
                    x: 60 + column * COLUMN_STEP,
                    y: group.top + 48 + row * ROW_STEP,
                };
            }
        }
    }
}

impl<'a> Layout<'a> {
    pub fn new(graph: &'a Graph) -> Self {
        let ranks = graph.ranks();
        let columns = ranks.iter().max().copied().unwrap_or(0) + 1;
        let mut modules = BTreeMap::new();
        for (node, resource) in graph.nodes.iter().enumerate() {
            modules
                .entry(resource.module.as_str())
                .or_insert_with(|| vec![Vec::new(); columns])[ranks[node]]
                .push(node);
        }
        let mut groups: Vec<_> = modules
            .into_iter()
            .map(|(label, columns)| Group {
                rows: columns.iter().map(Vec::len).max().unwrap_or(0),
                label,
                columns,
                top: 0,
            })
            .collect();
        // Keep root first, then module paths in deterministic lexical order.
        groups.sort_by_key(|g| (g.label != "root", g.label));
        let mut top = 160;
        let mut channels = Vec::new();
        for group in &mut groups {
            group.top = top;
            for row in 0..=group.rows {
                channels.push(top + 32 + row * ROW_STEP);
            }
            top += group.rows * ROW_STEP + 80;
        }
        let mut positions = vec![Point { x: 0, y: 0 }; graph.nodes.len()];
        update_positions(&groups, &mut positions);
        let mut incoming = vec![Vec::new(); graph.nodes.len()];
        let mut outgoing = vec![Vec::new(); graph.nodes.len()];
        for &(a, b) in &graph.edges {
            incoming[b].push(a);
            outgoing[a].push(b);
        }
        // Alternating barycentric sweeps reduce crossings without disturbing
        // module bands or dependency ranks. Ties retain their previous order.
        for sweep in 0..6 {
            let forward = sweep % 2 == 0;
            for step in 0..columns {
                let rank = if forward { step } else { columns - step - 1 };
                let neighbors = if forward { &incoming } else { &outgoing };
                for group in &mut groups {
                    group.columns[rank].sort_by_cached_key(|&node| {
                        let ys: Vec<_> = neighbors[node]
                            .iter()
                            .copied()
                            .filter(|&other| {
                                if forward {
                                    ranks[other] < rank
                                } else {
                                    ranks[other] > rank
                                }
                            })
                            .map(|other| positions[other].y as u64)
                            .collect();
                        if ys.is_empty() {
                            positions[node].y as u64 * 1024
                        } else {
                            ys.iter().sum::<u64>() * 1024 / ys.len() as u64
                        }
                    });
                }
                update_positions(&groups, &mut positions);
            }
        }

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
        let paths = graph
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
                    x: if ranks[b] > ranks[a] {
                        target.x
                    } else {
                        target.x + NODE_WIDTH
                    },
                    y: target.y + target_ports[edge],
                };
                let lane = 24 + (edge % 5) * 10;
                let route = if ranks[b] == ranks[a] + 1 || ranks[b] == ranks[a] {
                    let x = start.x + lane;
                    vec![start, Point { x, y: start.y }, Point { x, y: end.y }, end]
                } else {
                    // Every horizontal channel is in a global gap between node rows.
                    // All vertical segments are in column gutters. Thus even long
                    // edges cannot cut through intermediate resource cards.
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
                    let right = if ranks[b] > ranks[a] {
                        end.x - 24 - edge % 5 * 10
                    } else {
                        end.x + lane
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
                };
                simplify(route)
            })
            .collect();
        Self {
            width: (columns * COLUMN_STEP + 80).max(1040),
            height: (top + 40).max(300),
            positions,
            paths,
            bands: groups
                .into_iter()
                .map(|g| Band {
                    label: g.label,
                    top: g.top,
                    height: g.rows * ROW_STEP + 56,
                })
                .collect(),
        }
    }
}

fn simplify(points: Vec<Point>) -> Vec<Point> {
    let mut result: Vec<Point> = Vec::new();
    for point in points {
        if result.last() == Some(&point) {
            continue;
        }
        while result.len() >= 2 {
            let a = result[result.len() - 2];
            let b = result[result.len() - 1];
            if (a.x == b.x && b.x == point.x) || (a.y == b.y && b.y == point.y) {
                result.pop();
            } else {
                break;
            }
        }
        result.push(point);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{Action, Node};

    fn graph(count: usize, edges: Vec<(usize, usize)>) -> Graph {
        Graph {
            nodes: (0..count)
                .map(|i| Node {
                    address: format!("test.n{i}"),
                    resource_type: "test".into(),
                    module: "root".into(),
                    action: Action::Create,
                })
                .collect(),
            edges,
        }
    }

    fn check_geometry(graph: &Graph) {
        let layout = Layout::new(graph);
        for (i, a) in layout.positions.iter().enumerate() {
            assert!(a.x + NODE_WIDTH <= layout.width && a.y + NODE_HEIGHT <= layout.height);
            for b in layout.positions.iter().skip(i + 1) {
                assert!(
                    a.x + NODE_WIDTH <= b.x
                        || b.x + NODE_WIDTH <= a.x
                        || a.y + NODE_HEIGHT <= b.y
                        || b.y + NODE_HEIGHT <= a.y
                );
            }
        }
        for path in &layout.paths {
            for segment in path.windows(2) {
                let (a, b) = (segment[0], segment[1]);
                assert!(a.x == b.x || a.y == b.y);
                assert!(a.x <= layout.width && a.y <= layout.height);
                for node in &layout.positions {
                    let crosses = if a.x == b.x {
                        a.x > node.x
                            && a.x < node.x + NODE_WIDTH
                            && a.y.max(b.y) > node.y
                            && a.y.min(b.y) < node.y + NODE_HEIGHT
                    } else {
                        a.y > node.y
                            && a.y < node.y + NODE_HEIGHT
                            && a.x.max(b.x) > node.x
                            && a.x.min(b.x) < node.x + NODE_WIDTH
                    };
                    assert!(!crosses, "edge {segment:?} crosses node {node:?}");
                }
            }
        }
    }

    #[test]
    fn sweeps_uncross_reversed_dependencies() {
        let graph = graph(4, vec![(0, 3), (1, 2)]);
        let positions = Layout::new(&graph).positions;
        assert_eq!(
            positions[0].y < positions[1].y,
            positions[3].y < positions[2].y
        );
    }

    #[test]
    fn long_edges_and_cycles_avoid_cards() {
        check_geometry(&graph(
            6,
            vec![(0, 1), (1, 2), (2, 3), (0, 3), (3, 4), (4, 3), (0, 5)],
        ));
        check_geometry(&Graph::parse(include_str!("../examples/plan.json")).unwrap());
    }

    #[test]
    fn dense_dag_and_multiple_modules_avoid_cards() {
        let mut edges = Vec::new();
        for a in 0..15 {
            for b in a + 1..15 {
                if (a + b) % 3 != 0 {
                    edges.push((a, b));
                }
            }
        }
        let mut graph = graph(15, edges);
        for (i, node) in graph.nodes.iter_mut().enumerate() {
            node.module = format!("module.group{}", i % 3);
        }
        check_geometry(&graph);
        check_geometry(
            &Graph::parse(include_str!("../tests/fixtures/terraform-plan.json")).unwrap(),
        );
    }
}
