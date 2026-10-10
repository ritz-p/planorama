use super::{Bounds, Point, Scorer, Simplification, crosses, search, simplify};
use crate::layout::{Port, Side};

pub(in crate::layout) fn select(
    source: Bounds,
    target: Bounds,
    baseline: Vec<Point>,
    obstacles: &[Bounds],
    scorer: &Scorer,
) -> Vec<Point> {
    let mut pairs = Vec::new();
    if source.right() < target.origin.x {
        pairs.push((Side::Right, Side::Left));
    }
    if target.right() < source.origin.x {
        pairs.push((Side::Left, Side::Right));
    }
    if source.origin.y + source.height < target.origin.y {
        pairs.push((Side::Bottom, Side::Top));
    }
    if target.origin.y + target.height < source.origin.y {
        pairs.push((Side::Top, Side::Bottom));
    }
    let offset = |bounds: Bounds, point: Point, side| match side {
        Side::Left | Side::Right => point.y.saturating_sub(bounds.origin.y),
        Side::Top | Side::Bottom => {
            point.y.saturating_sub(bounds.origin.y).min(bounds.height) * bounds.width
                / bounds.height.max(1)
        }
    };
    let (Some(&first), Some(&last)) = (baseline.first(), baseline.last()) else {
        return baseline;
    };
    let mut best = baseline;
    for (a, b) in pairs {
        let start = source.port(a, offset(source, first, a));
        let end = target.port(b, offset(target, last, b));
        if start.point == first && end.point == last {
            continue;
        }
        for quality in [None, Some(scorer)] {
            if let Some(candidate) = connect(start, end, obstacles, quality) {
                if (start.point != first && scorer.uses_endpoint(start.point))
                    || (end.point != last && scorer.uses_endpoint(end.point))
                {
                    continue;
                }
                if scorer.score(&candidate, obstacles) < scorer.score(&best, obstacles) {
                    best = candidate;
                }
            }
        }
    }
    best
}

pub(in crate::layout) fn connect(
    start: Port,
    end: Port,
    obstacles: &[Bounds],
    scorer: Option<&Scorer>,
) -> Option<Vec<Point>> {
    let first = start.outward(16);
    let last = end.outward(16);
    if first == start.point
        || last == end.point
        || obstacles
            .iter()
            .any(|&b| crosses(start.point, first, b) || crosses(last, end.point, b))
    {
        return None;
    }
    let middle = search::search(first, last, obstacles, scorer, Some((start.side, end.side)))?;
    let path = simplify(
        std::iter::once(start.point)
            .chain(middle)
            .chain(std::iter::once(end.point)),
        Simplification::PreserveReversals,
    );
    (!path
        .windows(2)
        .any(|p| obstacles.iter().any(|&b| crosses(p[0], p[1], b))))
    .then_some(path)
}

#[test]
fn chooses_facing_sides_for_horizontal_vertical_and_container_bounds() {
    let card = |x, y| Bounds {
        origin: Point { x, y },
        width: 100,
        height: 80,
    };
    for (source, target, from_side, to_side) in [
        (card(100, 100), card(350, 100), Side::Right, Side::Left),
        (card(350, 100), card(100, 100), Side::Left, Side::Right),
        (card(100, 100), card(100, 350), Side::Bottom, Side::Top),
        (card(100, 350), card(100, 100), Side::Top, Side::Bottom),
        (
            Bounds {
                width: 220,
                height: 200,
                ..card(100, 100)
            },
            card(500, 100),
            Side::Right,
            Side::Left,
        ),
        (
            card(500, 100),
            Bounds {
                width: 220,
                height: 200,
                ..card(100, 100)
            },
            Side::Left,
            Side::Right,
        ),
    ] {
        let obstacles = [source, target];
        let baseline = connect(
            source.port(Side::Right, source.height / 2),
            target.port(Side::Right, target.height / 2),
            &obstacles,
            None,
        )
        .unwrap();
        let chosen = select(
            source,
            target,
            baseline.clone(),
            &obstacles,
            &Scorer::default(),
        );
        let offset = |bounds: Bounds, side| {
            if matches!(side, Side::Left | Side::Right) {
                bounds.height / 2
            } else {
                bounds.width / 2
            }
        };
        assert_eq!(
            chosen.first(),
            Some(&source.port(from_side, offset(source, from_side)).point)
        );
        assert_eq!(
            chosen.last(),
            Some(&target.port(to_side, offset(target, to_side)).point)
        );
        assert_eq!(
            chosen,
            select(source, target, baseline, &obstacles, &Scorer::default())
        );
        assert!(
            chosen
                .windows(2)
                .all(|p| p[0].x == p[1].x || p[0].y == p[1].y)
        );
    }
}

#[test]
fn side_selection_preserves_obstacle_avoidance() {
    let source = Bounds {
        origin: Point { x: 100, y: 100 },
        width: 100,
        height: 80,
    };
    let target = Bounds {
        origin: Point { x: 500, y: 100 },
        ..source
    };
    let obstacle = Bounds {
        origin: Point { x: 280, y: 70 },
        width: 100,
        height: 160,
    };
    let obstacles = [source, target, obstacle];
    let baseline = connect(
        source.port(Side::Right, 40),
        target.port(Side::Right, 40),
        &obstacles,
        None,
    )
    .unwrap();
    let chosen = select(source, target, baseline, &obstacles, &Scorer::default());
    assert!(
        chosen
            .windows(2)
            .all(|p| obstacles.iter().all(|&b| !crosses(p[0], p[1], b)))
    );
}
