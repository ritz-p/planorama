use super::*;

#[test]
fn initial_stub_bend_can_outweigh_a_short_overlap() {
    let start = Point { x: 116, y: 100 };
    let end = Point { x: 516, y: 220 };
    let obstacles = [Bounds {
        origin: Point { x: 207, y: 148 },
        width: 100,
        height: 60,
    }];
    let mut scorer = Scorer::default();
    scorer.insert(vec![start, Point { x: 119, y: 100 }]);

    // Paying 3 * 8 for overlap avoids two bends (2 * 24), including
    // the turn from the fixed horizontal source stub into a vertical step.
    assert_eq!(
        find_path(start, end, &obstacles, Some(&scorer)),
        vec![start, Point { x: 516, y: 100 }, end]
    );
}

#[test]
fn terminal_stub_bend_is_charged_before_selecting_the_destination() {
    let start = Point { x: 516, y: 100 };
    let end = Point { x: 116, y: 210 };
    let obstacles = [Bounds {
        origin: Point { x: 308, y: 117 },
        width: 100,
        height: 60,
    }];
    let mut scorer = Scorer::default();
    scorer.insert(vec![end, Point { x: 120, y: 210 }]);

    // A vertical arrival saves 4 * 8 in overlap but adds two bends.
    // Omitting the terminal bend makes that worse route appear cheaper.
    assert_eq!(
        find_path(start, end, &obstacles, Some(&scorer)),
        vec![start, Point { x: 516, y: 210 }, end]
    );
}
