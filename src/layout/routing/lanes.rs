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
    #[cfg(test)]
    pub(super) fn allocate(&mut self, gutter: usize, from: usize, to: usize) -> Lane {
        let lane = self.preview(gutter, from, to);
        self.reserve(lane, from, to);
        lane
    }

    pub(super) fn reserve(&mut self, lane: Lane, from: usize, to: usize) {
        let range = from.min(to)..from.max(to);
        let lanes = self.gutters.entry(lane.gutter).or_default();
        lanes.resize_with(lanes.len().max(lane.index + 1), Vec::new);
        if !range.is_empty() {
            lanes[lane.index].push(range);
        }
    }

    pub(super) fn preview(&self, gutter: usize, from: usize, to: usize) -> Lane {
        self.candidates(gutter, from, to, 1)[0]
    }

    pub(super) fn candidates(
        &self,
        gutter: usize,
        from: usize,
        to: usize,
        count: usize,
    ) -> Vec<Lane> {
        let range = from.min(to)..from.max(to);
        let lanes = self.gutters.get(&gutter).map_or(&[][..], Vec::as_slice);
        (0..)
            .filter(|&index| match lanes.get(index) {
                None => true,
                Some(occupied) => {
                    range.is_empty()
                        || occupied.iter().all(|existing| {
                            existing.start >= range.end || range.start >= existing.end
                        })
                }
            })
            .take(count)
            .map(|index| Lane { gutter, index })
            .collect()
    }

    pub(super) fn width(&self, gutter: usize) -> usize {
        let count = self.gutters.get(&gutter).map_or(0, Vec::len);
        (2 * MARGIN + count.saturating_sub(1) * SPACING).max(100)
    }
}
