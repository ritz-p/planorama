use crate::model::{Graph, RelationshipProvenance};

pub(crate) const HEIGHT: usize = 18;
pub(crate) const GAP: usize = 10;
pub(super) const PORT_SPACING: usize = HEIGHT + 6;

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
