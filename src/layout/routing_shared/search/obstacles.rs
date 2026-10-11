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
#[path = "../../../../tests/unit/layout/routing_shared/search/obstacles.rs"]
mod tests;
