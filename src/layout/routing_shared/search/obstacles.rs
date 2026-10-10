use super::Bounds;

pub(super) struct Grid {
    horizontal: Vec<i32>,
    vertical: Vec<i32>,
    columns: usize,
}

impl Grid {
    pub fn new(xs: &[usize], ys: &[usize], obstacles: &[Bounds]) -> Self {
        let columns = xs.len();
        let mut horizontal = vec![0; columns * ys.len()];
        let mut vertical = vec![0; columns * ys.len()];
        for bounds in obstacles {
            let left = xs
                .partition_point(|&x| x <= bounds.origin.x)
                .saturating_sub(1);
            let right = xs.partition_point(|&x| x < bounds.right()).min(columns - 1);
            let top = ys
                .partition_point(|&y| y <= bounds.origin.y)
                .saturating_sub(1);
            let bottom = ys
                .partition_point(|&y| y < bounds.origin.y + bounds.height)
                .min(ys.len() - 1);
            if left < right {
                for row in ys.partition_point(|&y| y <= bounds.origin.y)
                    ..ys.partition_point(|&y| y < bounds.origin.y + bounds.height)
                {
                    horizontal[row * columns + left] += 1;
                    horizontal[row * columns + right] -= 1;
                }
            }
            if top < bottom {
                for column in xs.partition_point(|&x| x <= bounds.origin.x)
                    ..xs.partition_point(|&x| x < bounds.right())
                {
                    vertical[top * columns + column] += 1;
                    vertical[bottom * columns + column] -= 1;
                }
            }
        }
        for row in horizontal.chunks_mut(columns) {
            let mut count = 0;
            for cell in row {
                count += *cell;
                *cell = count;
            }
        }
        for column in 0..columns {
            let mut count = 0;
            for row in 0..ys.len() {
                let cell = &mut vertical[row * columns + column];
                count += *cell;
                *cell = count;
            }
        }
        Self {
            horizontal,
            vertical,
            columns,
        }
    }

    pub fn blocks(&self, from: usize, to: usize) -> bool {
        let cells = if from / self.columns == to / self.columns {
            &self.horizontal
        } else {
            &self.vertical
        };
        cells[from.min(to)] > 0
    }
}

#[cfg(test)]
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
