use super::{NODE_HEIGHT, NODE_WIDTH};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Side {
    Left,
    Right,
    Top,
    Bottom,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Port {
    pub point: Point,
    pub side: Side,
}
impl Port {
    pub fn outward(self, distance: usize) -> Point {
        let Point { x, y } = self.point;
        match self.side {
            Side::Left => Point {
                x: x.saturating_sub(distance),
                y,
            },
            Side::Right => Point { x: x + distance, y },
            Side::Top => Point {
                x,
                y: y.saturating_sub(distance),
            },
            Side::Bottom => Point { x, y: y + distance },
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Point {
    pub x: usize,
    pub y: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Bounds {
    pub origin: Point,
    pub width: usize,
    pub height: usize,
}

impl Bounds {
    pub fn port(self, side: Side, offset: usize) -> Port {
        let Point { x, y } = self.origin;
        let point = match side {
            Side::Left => Point {
                x,
                y: y + offset.min(self.height),
            },
            Side::Right => Point {
                x: self.right(),
                y: y + offset.min(self.height),
            },
            Side::Top => Point {
                x: x + offset.min(self.width),
                y,
            },
            Side::Bottom => Point {
                x: x + offset.min(self.width),
                y: y + self.height,
            },
        };
        Port { point, side }
    }
    pub fn card(origin: Point) -> Self {
        Self {
            origin,
            width: NODE_WIDTH,
            height: NODE_HEIGHT,
        }
    }

    pub fn right(self) -> usize {
        self.origin.x + self.width
    }

    pub fn header(self, height: usize) -> Self {
        Self { height, ..self }
    }
}

#[cfg(test)]
#[path = "../../tests/unit/layout/geometry.rs"]
mod tests;
