use super::*;

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
