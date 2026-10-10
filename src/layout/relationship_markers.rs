use crate::model::{Graph, RelationshipProvenance};

pub(crate) const HEIGHT: usize = 18;
pub(crate) const GAP: usize = 10;
pub(super) const PORT_SPACING: usize = HEIGHT + 6;

fn slot_width(edge: usize, source: bool, numbered: &[bool]) -> usize {
    if !source && numbered[edge] {
        PORT_SPACING
    } else {
        1
    }
}

pub(super) fn port_span(incidents: &[(usize, bool)], numbered: &[bool]) -> usize {
    incidents
        .iter()
        .map(|&(edge, source)| slot_width(edge, source, numbered))
        .sum::<usize>()
        + 1
}

pub(super) fn ports(incidents: &[(usize, bool)], numbered: &[bool], height: usize) -> Vec<usize> {
    let extra = height.saturating_sub(40 + port_span(incidents, numbered));
    let mut used = 0;
    incidents
        .iter()
        .enumerate()
        .map(|(slot, &(edge, source))| {
            let width = slot_width(edge, source, numbered);
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
    (clearance(graph) + 24).max(40)
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
