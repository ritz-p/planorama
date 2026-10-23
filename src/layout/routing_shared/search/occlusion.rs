use crate::layout::routing_shared::coverage::Coverage;
use crate::layout::routing_shared::scoring::Occluder;

pub(super) struct Grid {
    horizontal: Vec<u128>,
    vertical: Vec<u128>,
    columns: usize,
}

impl Grid {
    #[cfg(test)]
    pub fn new(xs: &[usize], ys: &[usize], bounds: &[Occluder]) -> Self {
        crate::work::complete(Self::new_step(xs, ys, bounds))
    }

    pub async fn new_step(xs: &[usize], ys: &[usize], bounds: &[Occluder]) -> Self {
        let columns = xs.len();
        let mut horizontal = vec![0; columns * ys.len()];
        let mut vertical = vec![0; columns * ys.len()];
        for (row, &y) in ys.iter().enumerate() {
            crate::work::checkpoint().await;
            let mut coverage = [Coverage::default(), Coverage::default()];
            for bounds in bounds {
                if bounds.origin.y < y && y < bounds.origin.y + bounds.height {
                    for _ in 0..bounds.cost {
                        coverage[bounds.step - 1]
                            .insert(bounds.origin.x / bounds.step, bounds.right() / bounds.step);
                    }
                }
            }
            for (column, pair) in xs.windows(2).enumerate() {
                horizontal[row * columns + column] = coverage
                    .iter()
                    .enumerate()
                    .map(|(i, c)| c.overlap(pair[0] / (i + 1), pair[1] / (i + 1)))
                    .sum();
            }
        }
        for (column, &x) in xs.iter().enumerate() {
            crate::work::checkpoint().await;
            let mut coverage = [Coverage::default(), Coverage::default()];
            for bounds in bounds {
                if bounds.origin.x < x && x < bounds.right() {
                    for _ in 0..bounds.cost {
                        coverage[bounds.step - 1].insert(
                            bounds.origin.y / bounds.step,
                            (bounds.origin.y + bounds.height) / bounds.step,
                        );
                    }
                }
            }
            for (row, pair) in ys.windows(2).enumerate() {
                vertical[row * columns + column] = coverage
                    .iter()
                    .enumerate()
                    .map(|(i, c)| c.overlap(pair[0] / (i + 1), pair[1] / (i + 1)))
                    .sum();
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
