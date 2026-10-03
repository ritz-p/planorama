use crate::layout::{NODE_HEIGHT, Point};
use crate::model::Graph;
use std::collections::{BTreeMap, BTreeSet};

#[cfg(test)]
#[path = "../../../tests/unit/layout/routing/bundles.rs"]
mod tests;

#[derive(Clone, Copy)]
enum Shared {
    Source,
    Target,
}

struct Bundle {
    edges: Vec<usize>,
    shared: Shared,
}

pub(super) struct Bundles {
    groups: Vec<Bundle>,
    membership: Vec<Option<usize>>,
}

impl Bundles {
    pub(super) fn new(graph: &Graph, ranks: &[usize]) -> Self {
        let mut incoming = vec![0; graph.nodes.len()];
        let mut outgoing = vec![0; graph.nodes.len()];
        for &(source, target) in &graph.edges {
            outgoing[source] += 1;
            incoming[target] += 1;
        }
        let mut result = Self {
            groups: Vec::new(),
            membership: vec![None; graph.edges.len()],
        };
        for shared in [Shared::Source, Shared::Target] {
            let mut groups: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
            for (edge, &(source, target)) in graph.edges.iter().enumerate() {
                if ranks[target] != ranks[source] + 1
                    || graph.nodes[source].module != graph.nodes[target].module
                    || result.membership[edge].is_some()
                {
                    continue;
                }
                let key = match shared {
                    Shared::Source if incoming[target] == 1 => source,
                    Shared::Target if outgoing[source] == 1 => target,
                    _ => continue,
                };
                groups.entry(key).or_default().push(edge);
            }
            for edges in groups.into_values().filter(|edges| edges.len() > 1) {
                let index = result.groups.len();
                for &edge in &edges {
                    result.membership[edge] = Some(index);
                }
                result.groups.push(Bundle { edges, shared });
            }
        }
        result
    }

    pub(super) fn len(&self) -> usize {
        self.groups.len()
    }

    pub(super) fn group(&self, edge: usize) -> Option<usize> {
        self.membership[edge]
    }

    pub(super) fn align_ports(&self, sources: &mut [usize], targets: &mut [usize]) {
        for group in &self.groups {
            for &edge in &group.edges {
                match group.shared {
                    Shared::Source => sources[edge] = NODE_HEIGHT / 2,
                    Shared::Target => targets[edge] = NODE_HEIGHT / 2,
                }
            }
        }
    }

    pub(super) fn span(
        &self,
        index: usize,
        graph: &Graph,
        positions: &[Point],
        sources: &[usize],
        targets: &[usize],
    ) -> (usize, usize) {
        self.groups[index]
            .edges
            .iter()
            .flat_map(|&edge| {
                let (source, target) = graph.edges[edge];
                [
                    positions[source].y + sources[edge],
                    positions[target].y + targets[edge],
                ]
            })
            .fold((usize::MAX, 0), |(low, high), y| (low.min(y), high.max(y)))
    }

    pub(super) fn junctions(&self, paths: &[Vec<Point>]) -> Vec<Point> {
        let mut result = BTreeSet::new();
        for group in &self.groups {
            let segments: Vec<_> = group
                .edges
                .iter()
                .flat_map(|&edge| paths[edge].windows(2))
                .collect();
            let x = match segments
                .iter()
                .find(|s| s[0].x == s[1].x && s[0].y != s[1].y)
            {
                Some(segment) => segment[0].x,
                None => continue,
            };
            let ys: BTreeSet<_> = segments.iter().flat_map(|s| [s[0].y, s[1].y]).collect();
            for y in ys {
                let mut directions = 0u8;
                for segment in &segments {
                    let (a, b) = (segment[0], segment[1]);
                    match (a.x == b.x, a.y == b.y) {
                        (true, _) if a.x == x && y >= a.y.min(b.y) && y <= a.y.max(b.y) => {
                            if y > a.y.min(b.y) {
                                directions |= 1;
                            }
                            if y < a.y.max(b.y) {
                                directions |= 2;
                            }
                        }
                        (_, true) if a.y == y && x >= a.x.min(b.x) && x <= a.x.max(b.x) => {
                            if x > a.x.min(b.x) {
                                directions |= 4;
                            }
                            if x < a.x.max(b.x) {
                                directions |= 8;
                            }
                        }
                        _ => {}
                    }
                }
                if directions.count_ones() >= 3 {
                    result.insert((x, y));
                }
            }
        }
        result.into_iter().map(|(x, y)| Point { x, y }).collect()
    }
}
