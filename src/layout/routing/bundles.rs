use crate::layout::{Bounds, Point};
use crate::model::architecture::{EdgeKind, Graph};
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
        for edge in &graph.edges {
            let (source, target) = edge.endpoints();
            outgoing[source] += 1;
            incoming[target] += 1;
        }
        let mut result = Self {
            groups: Vec::new(),
            membership: vec![None; graph.edges.len()],
        };
        for shared in [Shared::Source, Shared::Target] {
            let mut groups: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
            for (edge, link) in graph.edges.iter().enumerate() {
                let (source, target) = link.endpoints();
                if ranks[target] != ranks[source] + 1
                    || link.kind != EdgeKind::Dependency
                    || link.change.is_some()
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

    pub(super) fn align_ports(
        &self,
        graph: &Graph,
        bounds: &[Bounds],
        sources: &mut [usize],
        targets: &mut [usize],
    ) {
        for group in &self.groups {
            for &edge in &group.edges {
                match group.shared {
                    Shared::Source => sources[edge] = bounds[graph.edges[edge].from].height / 2,
                    Shared::Target => targets[edge] = bounds[graph.edges[edge].to].height / 2,
                }
            }
        }
    }

    pub(super) fn span(
        &self,
        index: usize,
        graph: &Graph,
        bounds: &[Bounds],
        sources: &[usize],
        targets: &[usize],
    ) -> (usize, usize) {
        self.groups[index]
            .edges
            .iter()
            .flat_map(|&edge| {
                let (source, target) = graph.edges[edge].endpoints();
                [
                    bounds[source].origin.y + sources[edge],
                    bounds[target].origin.y + targets[edge],
                ]
            })
            .fold((usize::MAX, 0), |(low, high), y| (low.min(y), high.max(y)))
    }

    pub(super) fn junctions(&self, paths: &[Vec<Point>]) -> Vec<Point> {
        let mut result = BTreeSet::new();
        for group in &self.groups {
            let segments = || group.edges.iter().flat_map(|&edge| paths[edge].windows(2));
            let x = match segments().find(|s| s[0].x == s[1].x && s[0].y != s[1].y) {
                Some(segment) => segment[0].x,
                None => continue,
            };
            let mut events: BTreeMap<usize, JunctionEvent> = BTreeMap::new();
            for segment in segments() {
                let (a, b) = (segment[0], segment[1]);
                match (a.x == b.x, a.y == b.y) {
                    (true, false) if a.x == x => {
                        events.entry(a.y.min(b.y)).or_default().starts += 1;
                        events.entry(a.y.max(b.y)).or_default().ends += 1;
                    }
                    (_, true) if x >= a.x.min(b.x) && x <= a.x.max(b.x) => {
                        let event = events.entry(a.y).or_default();
                        if x > a.x.min(b.x) {
                            event.directions |= 4;
                        }
                        if x < a.x.max(b.x) {
                            event.directions |= 8;
                        }
                    }
                    _ => {}
                }
            }
            let mut active = 0;
            for (y, event) in events {
                let mut directions = event.directions;
                if active > 0 {
                    directions |= 1;
                }
                active = active - event.ends + event.starts;
                if active > 0 {
                    directions |= 2;
                }
                if directions.count_ones() >= 3 {
                    result.insert((x, y));
                }
            }
        }
        result.into_iter().map(|(x, y)| Point { x, y }).collect()
    }
}

#[derive(Default)]
struct JunctionEvent {
    starts: usize,
    ends: usize,
    directions: u8,
}
