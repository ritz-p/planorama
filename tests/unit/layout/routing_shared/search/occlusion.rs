use super::*;
use crate::layout::Bounds;

#[test]
fn indexed_cost_matches_scanned_cost_in_both_directions() {
    use crate::layout::{Point, routing_shared::Scorer};
    let xs = [0, 5, 10, 18, 30, 45, 60];
    let ys = [0, 4, 8, 16, 25, 40, 60];
    let mut scorer = Scorer::default();
    scorer.set_soft(vec![
        Occluder {
            bounds: Bounds {
                origin: Point { x: 5, y: 8 },
                width: 25,
                height: 17,
            },
            cost: 1,
            step: 2,
        },
        Bounds {
            origin: Point { x: 18, y: 16 },
            width: 27,
            height: 24,
        }
        .into(),
        Bounds {
            origin: Point { x: 1, y: 1 },
            width: 2,
            height: 2,
        }
        .into(),
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
