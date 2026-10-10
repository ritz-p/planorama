use crate::model::{Graph, RelationshipProvenance};

pub(crate) const HEIGHT: usize = 18;
pub(crate) const GAP: usize = 10;
pub(super) const PORT_SPACING: usize = HEIGHT + 6;

pub(in crate::layout) fn corridor(port: super::Port, clearance: usize) -> super::Bounds {
    use super::{Bounds, Point, Side};
    let other = port.outward(clearance - 4);
    let vertical = matches!(port.side, Side::Top | Side::Bottom);
    let half = if vertical {
        (clearance - GAP - 8 + 6) / 2
    } else {
        PORT_SPACING / 2
    };
    Bounds {
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
            half * 2
        } else {
            port.point.x.abs_diff(other.x)
        },
        height: if vertical {
            port.point.y.abs_diff(other.y)
        } else {
            half * 2
        },
    }
}

pub(super) fn ends(graph: &Graph) -> Vec<[bool; 2]> {
    super::resource_edges(graph)
        .into_iter()
        .zip(&graph.edges)
        .map(|(numbered, edge)| {
            [
                numbered && edge.directionality() == crate::model::Directionality::Undirected,
                numbered,
            ]
        })
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
    numbered: &[[bool; 2]],
    height: usize,
    spacing: usize,
    minimum: usize,
) -> Vec<usize> {
    let widths: Vec<_> = incidents
        .iter()
        .map(|&(edge, source)| {
            if numbered[edge][usize::from(!source)] {
                spacing
            } else {
                minimum
            }
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
    let count = graph
        .relationships
        .iter()
        .filter(|relationship| {
            relationship
                .provenance
                .iter()
                .any(|provenance| matches!(provenance, RelationshipProvenance::Resource { .. }))
        })
        .count();
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
#[test]
fn adjacent_corridors_merge_without_closing_unreserved_gaps() {
    use super::{Bounds, Point};
    let mut corridors: Vec<_> = (0..200)
        .map(|i| Bounds {
            origin: Point {
                x: 500,
                y: 100 + i * PORT_SPACING,
            },
            width: 46,
            height: PORT_SPACING,
        })
        .collect();
    let separate = Bounds {
        origin: Point { x: 500, y: 5000 },
        width: 46,
        height: PORT_SPACING,
    };
    corridors.push(separate);
    corridors.reverse();
    let merged = merge_corridors(corridors);
    assert_eq!(merged.len(), 2);
    assert_eq!(merged[0].height, 200 * PORT_SPACING);
    assert_eq!(merged[1], separate);
}
