use super::*;
use std::collections::BTreeSet;

#[derive(Clone, Debug)]
pub(super) struct Lane {
    pub edge: usize,
    pub original: usize,
    pub coordinate: usize,
}

#[derive(Clone, Debug)]
pub(super) struct Corridor {
    pub horizontal: bool,
    pub low: usize,
    pub high: usize,
    pub lanes: Vec<Lane>,
}

pub(super) fn corridors(graph: &Graph, layout: &Layout<'_>) -> Vec<Corridor> {
    let pairs = pairs(graph, &layout.paths, PREFERRED - 1);
    let mut groups: Vec<Vec<Segment>> = Vec::new();
    for (a, b, _) in pairs {
        let mut joined = vec![a, b];
        let mut i = 0;
        while i < groups.len() {
            if groups[i].iter().any(|s| {
                joined
                    .iter()
                    .any(|t| s.edge == t.edge && s.index == t.index)
            }) {
                joined.extend(groups.remove(i));
                i = 0;
            } else {
                i += 1;
            }
        }
        joined.sort_by_key(|s| (s.edge, s.index));
        joined.dedup_by_key(|s| (s.edge, s.index));
        groups.push(joined);
    }
    let identity = |edge: usize| {
        let e = &graph.edges[edge];
        (
            &graph.nodes[e.from].entity.id,
            &graph.nodes[e.to].entity.id,
            e.kind,
            e.change.as_ref().map(|c| (&c.address, c.action)),
        )
    };
    let mut result = Vec::new();
    for group in groups {
        let mut edges = BTreeSet::new();
        let mut lanes = Vec::new();
        let mut sorted = group.clone();
        sorted.sort_by_key(|s| (s.coordinate, identity(s.edge), s.index));
        for s in sorted {
            if edges.insert(s.edge) {
                lanes.push(Lane {
                    edge: s.edge,
                    original: s.coordinate,
                    coordinate: s.coordinate,
                });
            }
        }
        if lanes.len() < 2 || lanes.len() > 16 {
            continue;
        }
        result.push(Corridor {
            horizontal: group[0].horizontal,
            low: group.iter().map(|s| s.low).min().unwrap(),
            high: group.iter().map(|s| s.high).max().unwrap(),
            lanes,
        });
    }
    result.sort_by_key(|c| {
        (
            c.horizontal,
            c.lanes[0].original,
            c.low,
            identity(c.lanes[0].edge),
        )
    });
    result
}

impl Corridor {
    fn gap(&self, layout: &Layout<'_>) -> Option<(usize, usize)> {
        let first = self.lanes.iter().map(|l| l.original).min()?;
        let last = self.lanes.iter().map(|l| l.original).max()?;
        let (mut low, mut high) = (
            0,
            if self.horizontal {
                layout.height
            } else {
                layout.width
            },
        );
        for (n, b) in layout.bounds.iter().enumerate() {
            let b = if layout.containers.contains(&n) {
                b.header(layout.header_heights[n])
            } else {
                *b
            };
            let (from, to, near, far) = if self.horizontal {
                (b.origin.x, b.right(), b.origin.y, b.origin.y + b.height)
            } else {
                (b.origin.y, b.origin.y + b.height, b.origin.x, b.right())
            };
            if to.min(self.high).saturating_sub(from.max(self.low)) < RUN {
                continue;
            }
            if far <= first {
                low = low.max(far);
            } else if near >= last {
                high = high.min(near);
            } else {
                return None;
            }
        }
        Some((low, high))
    }

    pub fn demand(&self, layout: &Layout<'_>) -> Option<(usize, usize)> {
        let (low, high) = self.gap(layout)?;
        let required = (self.lanes.len() + 1) * PREFERRED;
        (high - low < required)
            .then_some((low + (high - low) / 2, required.saturating_sub(high - low)))
    }

    pub fn allocate(&mut self, layout: &Layout<'_>) {
        let first = self.lanes[0].original;
        let last = self.lanes.last().unwrap().original;
        let span = (self.lanes.len() - 1) * PREFERRED;
        let mut start = (first + last).saturating_sub(span) / 2;
        if let Some((low, high)) = self.gap(layout) {
            if high - low >= span + 2 * PREFERRED {
                start = start.max(low + PREFERRED).min(high - PREFERRED - span);
            }
        }
        for (i, lane) in self.lanes.iter_mut().enumerate() {
            lane.coordinate = start + i * PREFERRED;
        }
    }

    pub fn translate(&mut self, cut: usize, amount: usize) {
        for lane in &mut self.lanes {
            if lane.original >= cut {
                lane.original += amount;
            }
        }
    }
}
