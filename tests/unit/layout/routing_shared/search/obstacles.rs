use super::*;

#[test]
fn grid_matches_rectangle_intersections_in_both_directions() {
    use crate::layout::{Point, routing_shared::crosses};
    let xs = [0, 5, 10, 18, 30, 45, 60];
    let ys = [0, 4, 8, 16, 25, 40, 60];
    let obstacles = [
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
    ];
    let grid = Grid::new(&xs, &ys, &obstacles);
    for y in 0..ys.len() {
        for x in 0..xs.len() {
            let from = y * xs.len() + x;
            for (nx, ny) in [(x + 1, y), (x, y + 1)] {
                if nx >= xs.len() || ny >= ys.len() {
                    continue;
                }
                let to = ny * xs.len() + nx;
                let expected = obstacles.iter().any(|&bounds| {
                    crosses(
                        Point { x: xs[x], y: ys[y] },
                        Point {
                            x: xs[nx],
                            y: ys[ny],
                        },
                        bounds,
                    )
                });
                assert_eq!(grid.blocks(from, to), expected);
                assert_eq!(grid.blocks(to, from), expected);
            }
        }
    }
}
