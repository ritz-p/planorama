use super::{Bounds, Point, Port, Side};
use crate::model::{
    ArchitectureId, Directionality, Edge, EdgeKind, Graph, Placement, RelationshipProvenance,
    TerraformEntityId,
};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) const HEIGHT: usize = 18;
pub(crate) const GAP: usize = 10;
pub(crate) const PORT_SPACING: usize = HEIGHT + 6;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Terminal {
    pub clearance: usize,
    pub span: usize,
}

impl Terminal {
    pub(crate) const fn plain(clearance: usize) -> Self {
        Self { clearance, span: 0 }
    }

    pub(crate) fn marker(number: usize, side: Side) -> Self {
        let (length, breadth) = match side {
            Side::Left | Side::Right => (width(number), HEIGHT),
            Side::Top | Side::Bottom => (HEIGHT, width(number)),
        };
        Self {
            clearance: GAP + length + 8,
            span: breadth + 6,
        }
    }

    pub(crate) fn corridor(self, port: Port) -> Option<Bounds> {
        if self.span == 0 {
            return None;
        }
        let other = port.outward(self.clearance - 4);
        let vertical = matches!(port.side, Side::Top | Side::Bottom);
        let half = self.span / 2;
        Some(Bounds {
            origin: Point {
                x: port
                    .point
                    .x
                    .min(other.x)
                    .saturating_sub(if vertical { half } else { 0 }),
                y: port
                    .point
                    .y
                    .min(other.y)
                    .saturating_sub(if vertical { 0 } else { half }),
            },
            width: if vertical {
                self.span
            } else {
                port.point.x.abs_diff(other.x)
            },
            height: if vertical {
                port.point.y.abs_diff(other.y)
            } else {
                self.span
            },
        })
    }
}

pub(crate) fn bounds(number: usize, port: Port) -> Bounds {
    let width = width(number);
    let length = if matches!(port.side, Side::Top | Side::Bottom) {
        HEIGHT
    } else {
        width
    };
    let center = port.outward(GAP + length / 2);
    Bounds {
        origin: Point {
            x: center.x.saturating_sub(width / 2),
            y: center.y.saturating_sub(HEIGHT / 2),
        },
        width,
        height: HEIGHT,
    }
}

type Key<'a> = (
    Vec<&'a TerraformEntityId>,
    &'a ArchitectureId,
    &'a ArchitectureId,
    EdgeKind,
);

pub(crate) struct Entry<'a> {
    pub sources: Vec<&'a TerraformEntityId>,
    pub edges: Vec<usize>,
    pub previous_addresses: BTreeMap<&'a TerraformEntityId, BTreeSet<&'a str>>,
}

#[derive(Default)]
struct Group<'a> {
    edges: Vec<usize>,
    previous_addresses: BTreeMap<&'a TerraformEntityId, BTreeSet<&'a str>>,
}

pub(crate) fn entries(graph: &Graph) -> Vec<Entry<'_>> {
    let owners: Vec<_> = graph
        .nodes
        .iter()
        .map(|node| crate::view::routes::owner(graph, node))
        .collect();
    let visible: Vec<_> = graph
        .nodes
        .iter()
        .enumerate()
        .map(|(i, node)| {
            owners[i].map_or(node.placement != Placement::Logical, |parent| {
                owners[parent].is_none() && graph.nodes[parent].placement != Placement::Logical
            })
        })
        .collect();
    let mut relationships = BTreeMap::new();
    for relationship in &graph.relationships {
        relationships
            .entry((&relationship.from, &relationship.to, relationship.kind))
            .or_insert_with(Vec::new)
            .push(relationship);
    }
    let mut grouped: BTreeMap<Key<'_>, Group<'_>> = BTreeMap::new();
    let mut containment = None;
    for (index, edge) in graph.edges.iter().enumerate() {
        if !visible[edge.from]
            || !visible[edge.to]
            || ((owners[edge.from].is_some() || owners[edge.to].is_some())
                && edge.kind != EdgeKind::Association)
        {
            continue;
        }
        let from = &graph.nodes[edge.from].entity.id;
        let to = &graph.nodes[edge.to].entity.id;
        let Some(matching) = relationships.get(&(from, to, edge.kind)) else {
            continue;
        };
        if matches!(edge.kind, EdgeKind::Dependency | EdgeKind::Containment) && edge.from != edge.to
        {
            let tree = containment.get_or_insert_with(|| super::ContainmentTree::new(graph));
            if tree.is_ancestor(edge.from, edge.to) || tree.is_ancestor(edge.to, edge.from) {
                continue;
            }
        }
        for relationship in matching {
            let matches_change = edge.change.as_ref().map_or(matching.len() == 1, |change| {
                relationship.provenance.iter().any(|p| {
                    matches!(p, RelationshipProvenance::Resource { change: source, .. } if source == change)
                })
            });
            if !matches_change {
                continue;
            }
            let mut sources: Vec<_> = relationship
                .provenance
                .iter()
                .filter_map(|p| match p {
                    RelationshipProvenance::Resource { source, .. } => Some(source),
                    RelationshipProvenance::Reference { .. } => None,
                })
                .collect();
            sources.sort();
            sources.dedup();
            if !sources.is_empty() {
                let group = grouped.entry((sources, from, to, edge.kind)).or_default();
                group.edges.push(index);
                for provenance in &relationship.provenance {
                    if let RelationshipProvenance::Resource { source, change, .. } = provenance {
                        if let Some(previous) = change.previous_address.as_deref() {
                            group
                                .previous_addresses
                                .entry(source)
                                .or_default()
                                .insert(previous);
                        }
                    }
                }
            }
        }
    }
    grouped
        .into_iter()
        .map(|((sources, ..), group)| {
            let Group {
                mut edges,
                previous_addresses,
            } = group;
            edges.sort_unstable();
            edges.dedup();
            Entry {
                sources,
                edges,
                previous_addresses,
            }
        })
        .collect()
}

pub(crate) fn both_ends(edge: &Edge) -> bool {
    matches!(edge.kind, EdgeKind::Association | EdgeKind::Subscription)
        || edge.directionality() == Directionality::Undirected
}

pub(in crate::layout) fn numbers(graph: &Graph) -> Vec<[Option<usize>; 2]> {
    let mut numbers = vec![[None; 2]; graph.edges.len()];
    for (i, entry) in entries(graph).into_iter().enumerate() {
        for edge in entry.edges {
            numbers[edge] = [both_ends(&graph.edges[edge]).then_some(i + 1), Some(i + 1)];
        }
    }
    numbers
}

pub(in crate::layout) fn terminals(
    graph: &Graph,
    index: usize,
    numbers: [Option<usize>; 2],
    sides: [Side; 2],
) -> [Terminal; 2] {
    let mut terminals = std::array::from_fn(|end| {
        numbers[end].map_or(Terminal::plain(16), |number| {
            Terminal::marker(number, sides[end])
        })
    });
    if numbers[0].is_none()
        && crate::view::routes::owner(graph, &graph.nodes[graph.edges[index].from]).is_some()
    {
        terminals[0] = Terminal::plain(20);
    }
    terminals
}

pub(super) fn ends(graph: &Graph) -> Vec<[bool; 2]> {
    numbers(graph)
        .into_iter()
        .map(|ends| ends.map(|number| number.is_some()))
        .collect()
}

fn slot_width(edge: usize, source: bool, numbered: &[[bool; 2]]) -> usize {
    if numbered[edge][usize::from(!source)] {
        PORT_SPACING
    } else {
        1
    }
}

pub(super) fn port_span(incidents: &[(usize, bool)], numbered: &[[bool; 2]]) -> usize {
    incidents
        .iter()
        .map(|&(edge, source)| slot_width(edge, source, numbered))
        .sum::<usize>()
        + 1
}

pub(super) fn ports_with_minimum(
    incidents: &[(usize, bool)],
    numbers: &[[Option<usize>; 2]],
    height: usize,
    side: Side,
    minimum: usize,
) -> Vec<usize> {
    let widths: Vec<_> = incidents
        .iter()
        .map(|&(edge, source)| {
            numbers[edge][usize::from(!source)]
                .map_or(minimum, |number| Terminal::marker(number, side).span)
        })
        .collect();
    let extra = height.saturating_sub(41 + widths.iter().sum::<usize>());
    let mut used = 0;
    incidents
        .iter()
        .enumerate()
        .map(|(slot, _)| {
            let width = widths[slot];
            let port = 20 + used + width.div_ceil(2) + (slot + 1) * extra / (incidents.len() + 1);
            used += width;
            port
        })
        .collect()
}

pub(crate) fn width(number: usize) -> usize {
    number.to_string().len() * 8 + 16
}

pub(super) fn clearance(graph: &Graph) -> usize {
    let count = entries(graph).len();
    if count == 0 {
        16
    } else {
        width(count) + GAP + 8
    }
}

pub(super) fn padding(graph: &Graph) -> usize {
    let terminals = if ends(graph).iter().any(|ends| ends[0] && ends[1]) {
        2
    } else {
        1
    };
    let clearance = clearance(graph);
    (clearance + 24).max(clearance * terminals).max(40)
}

pub(super) fn merge_corridors(mut bounds: Vec<super::Bounds>) -> Vec<super::Bounds> {
    bounds.sort_by_key(|b| (b.origin.x, b.width, b.origin.y));
    let mut merged: Vec<super::Bounds> = Vec::new();
    for bounds in bounds {
        if let Some(last) = merged.last_mut() {
            if last.origin.x == bounds.origin.x
                && last.width == bounds.width
                && bounds.origin.y <= last.origin.y + last.height
            {
                last.height = last
                    .height
                    .max(bounds.origin.y + bounds.height - last.origin.y);
                continue;
            }
        }
        merged.push(bounds);
    }
    merged
}

#[cfg(test)]
#[path = "../../tests/unit/layout/relationship_markers.rs"]
mod tests;
