use std::collections::BTreeMap;
use std::ops::Range;

#[cfg(test)]
#[path = "../../../tests/unit/layout/routing/lanes.rs"]
mod tests;

const MARGIN: usize = 24;
const SPACING: usize = 10;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Lane {
    pub gutter: usize,
    index: usize,
}

impl Lane {
    pub(super) fn offset(self) -> usize {
        MARGIN + self.index * SPACING
    }
}

#[derive(Default)]
pub(super) struct VerticalLanes {
    gutters: BTreeMap<usize, Vec<Vec<Range<usize>>>>,
}

impl VerticalLanes {
    pub(super) fn allocate(&mut self, gutter: usize, from: usize, to: usize) -> Lane {
        let lane = self.preview(gutter, from, to);
        let range = from.min(to)..from.max(to);
        let lanes = self.gutters.entry(gutter).or_default();
        if lane.index == lanes.len() {
            lanes.push(Vec::new());
        }
        if !range.is_empty() {
            lanes[lane.index].push(range);
        }
        lane
    }

    pub(super) fn preview(&self, gutter: usize, from: usize, to: usize) -> Lane {
        let range = from.min(to)..from.max(to);
        let lanes = self.gutters.get(&gutter).map_or(&[][..], Vec::as_slice);
        let index = match lanes.iter().position(|occupied| {
            range.is_empty()
                || occupied
                    .iter()
                    .all(|existing| existing.start >= range.end || range.start >= existing.end)
        }) {
            Some(index) => index,
            None => lanes.len(),
        };
        Lane { gutter, index }
    }

    pub(super) fn width(&self, gutter: usize) -> usize {
        let count = self.gutters.get(&gutter).map_or(0, Vec::len);
        (2 * MARGIN + count.saturating_sub(1) * SPACING).max(100)
    }
}
