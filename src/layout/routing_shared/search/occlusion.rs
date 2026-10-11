use super::Bounds;
use crate::layout::routing_shared::coverage::Coverage;

pub(super) struct Grid {
    horizontal: Vec<u128>,
    vertical: Vec<u128>,
    columns: usize,
}

impl Grid {
    pub fn new(xs: &[usize], ys: &[usize], bounds: &[Bounds]) -> Self {
        let columns = xs.len();
        let mut horizontal = vec![0; columns * ys.len()];
        let mut vertical = vec![0; columns * ys.len()];
        for (row, &y) in ys.iter().enumerate() {
            let mut coverage = Coverage::default();
            for bounds in bounds {
                if bounds.origin.y < y && y < bounds.origin.y + bounds.height {
                    coverage.insert(bounds.origin.x, bounds.right());
                }
            }
            for (column, pair) in xs.windows(2).enumerate() {
                horizontal[row * columns + column] = coverage.overlap(pair[0], pair[1]) * 2;
            }
        }
        for (column, &x) in xs.iter().enumerate() {
            let mut coverage = Coverage::default();
            for bounds in bounds {
                if bounds.origin.x < x && x < bounds.right() {
                    coverage.insert(bounds.origin.y, bounds.origin.y + bounds.height);
                }
            }
            for (row, pair) in ys.windows(2).enumerate() {
                vertical[row * columns + column] = coverage.overlap(pair[0], pair[1]) * 2;
            }
        }
        Self {
            horizontal,
            vertical,
            columns,
        }
    }

    pub fn cost(&self, from: usize, to: usize) -> u128 {
        let cells = if from / self.columns == to / self.columns {
            &self.horizontal
        } else {
            &self.vertical
        };
        cells[from.min(to)]
    }
}

#[cfg(test)]
#[path = "../../../../tests/unit/layout/routing_shared/search/occlusion.rs"]
mod tests;
