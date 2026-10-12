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
    pub border: bool,
    pub(super) minimum_margin: usize,
    pub(super) preferred_margin: usize,
    pub(super) urgent: bool,
    pub(super) deficit_cost: u128,
    pub(super) hidden: BTreeSet<usize>,
}

fn join(groups: &mut Vec<Vec<Segment>>, mut joined: Vec<Segment>) {
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
    let mut merged: Vec<Segment> = Vec::new();
    for segment in joined {
        if let Some(last) = merged.last_mut() {
            if last.edge == segment.edge && last.index == segment.index {
                last.low = last.low.min(segment.low);
                last.high = last.high.max(segment.high);
                continue;
            }
        }
        merged.push(segment);
    }
    groups.push(merged);
}

pub(super) fn corridors(graph: &Graph, layout: &Layout<'_>) -> Vec<Corridor> {
    let pairs = pairs(graph, &layout.paths, PREFERRED - 1);
    let mut groups: Vec<Vec<Segment>> = Vec::new();
    for &(a, b, _) in &pairs {
        join(&mut groups, vec![a, b]);
    }
    let mut border_demand = BTreeMap::<_, (usize, usize, bool)>::new();
    let mut border_cost = vec![0; layout.paths.len()];
    let mut contact_frames = BTreeMap::<_, BTreeSet<usize>>::new();
    let masks: Vec<_> = graph
        .edges
        .iter()
        .map(|edge| layout.occluders(edge))
        .collect();
    let mut scorer = super::super::scoring::Scorer::default();
    scorer.set_layout(graph, layout);
    for (edge, path) in layout.paths.iter().enumerate() {
        if path.len() < 2 {
            continue;
        }
        scorer.set_kind(graph.edges[edge].kind);
        scorer.set_soft(crate::layout::containers::routing::soft_obstacles(
            layout,
            &graph.edges[edge],
        ));
        let metrics = scorer.border_metrics(path);
        let urgent = metrics.minimum_deficit > 0;
        border_cost[edge] = metrics.cost();
        for contact in scorer.border_contacts(path) {
            if let Some(node) = contact.frame.node {
                contact_frames
                    .entry((edge, contact.segment))
                    .or_default()
                    .insert(node);
            }
            let demand = border_demand.entry((edge, contact.segment)).or_default();
            demand.0 = demand.0.max(contact.minimum);
            demand.1 = demand.1.max(contact.preferred);
            demand.2 |= urgent;
            join(
                &mut groups,
                vec![Segment {
                    edge,
                    index: contact.segment,
                    horizontal: contact.horizontal,
                    coordinate: contact.coordinate,
                    low: contact.low,
                    high: contact.high,
                }],
            );
        }
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
        let border = group
            .iter()
            .any(|s| border_demand.contains_key(&(s.edge, s.index)));
        if (lanes.len() < 2 && !border) || lanes.len() > 16 {
            continue;
        }
        let demand = group
            .iter()
            .filter_map(|s| border_demand.get(&(s.edge, s.index)))
            .fold((0, 0, false), |a, b| {
                (a.0.max(b.0), a.1.max(b.1), a.2 || b.2)
            });
        let line_margin = if lanes.len() > 1 { PREFERRED } else { 0 };
        let visible_frames: BTreeSet<_> = group
            .iter()
            .filter_map(|s| contact_frames.get(&(s.edge, s.index)))
            .flatten()
            .copied()
            .collect();
        let hidden = if border {
            (0..layout.bounds.len())
                .filter(|node| {
                    !visible_frames.contains(node)
                        && lanes.iter().all(|lane| {
                            masks[lane.edge]
                                .iter()
                                .any(|&parent| layout.containment.is_ancestor(parent, *node))
                        })
                })
                .collect()
        } else {
            BTreeSet::new()
        };
        let line_deficit: usize = pairs
            .iter()
            .filter(|(a, b, _)| {
                [a, b].iter().all(|pair| {
                    group
                        .iter()
                        .any(|s| s.edge == pair.edge && s.index == pair.index)
                })
            })
            .map(|(a, b, length)| length * (PREFERRED - a.coordinate.abs_diff(b.coordinate)))
            .sum();
        let deficit_cost = (line_deficit / 4) as u128
            + lanes
                .iter()
                .filter(|lane| reroute::movable(layout, lane.edge))
                .map(|lane| border_cost[lane.edge])
                .sum::<u128>();
        result.push(Corridor {
            horizontal: group[0].horizontal,
            low: group.iter().map(|s| s.low).min().unwrap(),
            high: group.iter().map(|s| s.high).max().unwrap(),
            lanes,
            border,
            minimum_margin: line_margin.max(demand.0),
            preferred_margin: line_margin.max(demand.1),
            urgent: demand.2,
            deficit_cost,
            hidden,
        });
    }
    result.sort_by_key(|c| {
        (
            std::cmp::Reverse(c.deficit_cost),
            !c.urgent,
            !c.border,
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
        for (n, frame) in layout.bounds.iter().enumerate() {
            if frame.width == 0 || frame.height == 0 || self.hidden.contains(&n) {
                continue;
            }
            let container = layout.containers.contains(&n);
            let bounds = if container && self.border {
                vec![*frame, frame.header(layout.header_heights[n])]
            } else if container {
                vec![frame.header(layout.header_heights[n])]
            } else {
                vec![*frame]
            };
            for b in bounds {
                let (from, to, near, far) = if self.horizontal {
                    (b.origin.x, b.right(), b.origin.y, b.origin.y + b.height)
                } else {
                    (b.origin.y, b.origin.y + b.height, b.origin.x, b.right())
                };
                if to.min(self.high).saturating_sub(from.max(self.low))
                    < if self.border { 1 } else { RUN }
                {
                    continue;
                }
                let inside = container && self.border && b == *frame && near < first && last < far;
                if inside {
                    low = low.max(near);
                    high = high.min(far);
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
        }
        (low <= high).then_some((low, high))
    }

    pub fn demand(&self, layout: &Layout<'_>) -> Option<(usize, usize)> {
        let (low, high) = self.gap(layout)?;
        let required = (self.lanes.len() - 1) * PREFERRED + 2 * self.preferred_margin;
        (high - low < required)
            .then_some((low + (high - low) / 2, required.saturating_sub(high - low)))
    }

    pub fn allocate(&mut self, layout: &Layout<'_>) {
        let first = self.lanes[0].original;
        let last = self.lanes.last().unwrap().original;
        let span = (self.lanes.len() - 1) * PREFERRED;
        let mut start = (first + last).saturating_sub(span) / 2;
        if let Some((low, high)) = self.gap(layout) {
            let margin = if high - low >= span + 2 * self.preferred_margin {
                self.preferred_margin
            } else {
                self.minimum_margin
            };
            if high - low >= span + 2 * margin {
                start = start.max(low + margin).min(high - margin - span);
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
