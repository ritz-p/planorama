//! Provider-neutral rectangle and edge attachment geometry.
use super::{NODE_HEIGHT, NODE_WIDTH};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(
    dead_code,
    reason = "All four sides are the port contract; side selection follows in #81"
)]
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
    /// Offset is measured from the top/left of this rectangle, clamped to its side.
    /// Use `header(height)` to request ports on a container's visible header.
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

#[test]
fn ports_follow_all_sides_actual_sizes_and_container_headers() {
    for (width, height) in [(320, 96), (840, 600)] {
        let bounds = Bounds {
            origin: Point { x: 50, y: 70 },
            width,
            height,
        };
        assert_eq!(bounds.port(Side::Left, 20).point, Point { x: 50, y: 90 });
        assert_eq!(
            bounds.port(Side::Right, 20).point,
            Point {
                x: 50 + width,
                y: 90
            }
        );
        assert_eq!(bounds.port(Side::Top, 30).point, Point { x: 80, y: 70 });
        assert_eq!(
            bounds.port(Side::Bottom, 30).point,
            Point {
                x: 80,
                y: 70 + height
            }
        );
        let port = bounds.header(120).port(Side::Bottom, usize::MAX);
        assert_eq!(
            port,
            Port {
                point: Point {
                    x: 50 + width,
                    y: 190
                },
                side: Side::Bottom
            }
        );
    }
}
