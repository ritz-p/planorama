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
#[test]
fn indexed_cost_matches_scanned_cost_in_both_directions() {
    use crate::layout::{Point, routing_shared::Scorer};
    let xs = [0, 5, 10, 18, 30, 45, 60];
    let ys = [0, 4, 8, 16, 25, 40, 60];
    let mut scorer = Scorer::default();
    scorer.set_soft(vec![
        Bounds {
            origin: Point { x: 5, y: 8 },
            width: 25,
            height: 17,
        },
        Bounds {
            origin: Point { x: 18, y: 16 },
            width: 27,
            height: 24,
        },
        Bounds {
            origin: Point { x: 1, y: 1 },
            width: 2,
            height: 2,
        },
    ]);
    let grid = Grid::new(&xs, &ys, scorer.soft());
    let point = |i: usize| Point {
        x: xs[i % xs.len()],
        y: ys[i / xs.len()],
    };
    for row in 0..ys.len() {
        for column in 0..xs.len() {
            let from = row * xs.len() + column;
            for to in [
                (column + 1 < xs.len()).then_some(from + 1),
                (row + 1 < ys.len()).then_some(from + xs.len()),
            ]
            .into_iter()
            .flatten()
            {
                assert_eq!(
                    grid.cost(from, to),
                    scorer.segment_cost(point(from), point(to))
                );
                assert_eq!(
                    grid.cost(to, from),
                    scorer.segment_cost(point(to), point(from))
                );
            }
        }
    }
}
