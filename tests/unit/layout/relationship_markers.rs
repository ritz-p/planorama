use super::*;

#[test]
fn adjacent_corridors_merge_without_closing_unreserved_gaps() {
    use super::super::{Bounds, Point};
    let mut corridors: Vec<_> = (0..200)
        .map(|i| Bounds {
            origin: Point {
                x: 500,
                y: 100 + i * PORT_SPACING,
            },
            width: 46,
            height: PORT_SPACING,
        })
        .collect();
    let separate = Bounds {
        origin: Point { x: 500, y: 5000 },
        width: 46,
        height: PORT_SPACING,
    };
    corridors.push(separate);
    corridors.reverse();
    let merged = merge_corridors(corridors);
    assert_eq!(merged.len(), 2);
    assert_eq!(merged[0].height, 200 * PORT_SPACING);
    assert_eq!(merged[1], separate);
}
