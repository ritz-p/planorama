use super::{Bounds, Point, Scorer, Simplification, crosses, search, simplify};
use crate::layout::{Port, Side};

#[cfg(test)]
pub(in crate::layout) fn select(
    source: Bounds,
    target: Bounds,
    baseline: Vec<Point>,
    obstacles: &[Bounds],
    scorer: &Scorer,
) -> Vec<Point> {
    select_with_slots(source, target, baseline, obstacles, scorer, None)
}

pub(in crate::layout) fn facing_pairs(source: Bounds, target: Bounds) -> Vec<(Side, Side)> {
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
    pairs
}

pub(in crate::layout) fn select_with_slots(
    source: Bounds,
    target: Bounds,
    baseline: Vec<Point>,
    obstacles: &[Bounds],
    scorer: &Scorer,
    slots: Option<(&[usize; 4], &[usize; 4])>,
) -> Vec<Point> {
    let pairs = facing_pairs(source, target);
    let offset = |bounds: Bounds, point: Point, side| {
        let horizontal = point.x > bounds.origin.x
            && point.x < bounds.right()
            && (point.y == bounds.origin.y || point.y == bounds.origin.y + bounds.height);
        let (position, extent) = if horizontal {
            (point.x.saturating_sub(bounds.origin.x), bounds.width)
        } else {
            (point.y.saturating_sub(bounds.origin.y), bounds.height)
        };
        let size = match side {
            Side::Left | Side::Right => bounds.height,
            Side::Top | Side::Bottom => bounds.width,
        };
        position.min(extent) * size / extent.max(1)
    };
    let (Some(&first), Some(&last)) = (baseline.first(), baseline.last()) else {
        return baseline;
    };
    let mut best = baseline;
    let mut preference = pairs.len();
    for (rank, (a, b)) in pairs.into_iter().enumerate() {
        let start = source.port(
            a,
            slots.map_or_else(|| offset(source, first, a), |s| s.0[a as usize]),
        );
        let end = target.port(
            b,
            slots.map_or_else(|| offset(target, last, b), |s| s.1[b as usize]),
        );
        if start.point == first && end.point == last {
            preference = preference.min(rank);
            continue;
        }
        for quality in [None, Some(scorer)] {
            if let Some(candidate) = connect(start, end, obstacles, quality) {
                if (start.point != first && scorer.uses_endpoint(start.point))
                    || (end.point != last && scorer.uses_endpoint(end.point))
                {
                    continue;
                }
                if (scorer.score(&candidate, obstacles), rank)
                    < (scorer.score(&best, obstacles), preference)
                {
                    best = candidate;
                    preference = rank;
                }
            }
        }
    }
    best
}

pub(in crate::layout) fn best_with_slots(
    source: Bounds,
    target: Bounds,
    slots: (&[usize; 4], &[usize; 4]),
    obstacles: &[Bounds],
    scorer: &Scorer,
) -> Option<Vec<Point>> {
    let baseline = std::iter::once((Side::Right, Side::Right))
        .chain(facing_pairs(source, target))
        .find_map(|(a, b)| {
            connect(
                source.port(a, slots.0[a as usize]),
                target.port(b, slots.1[b as usize]),
                obstacles,
                Some(scorer),
            )
        })?;
    Some(select_with_slots(
        source,
        target,
        baseline,
        obstacles,
        scorer,
        Some(slots),
    ))
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
    let default_scorer = Scorer::default();
    let quality = scorer.unwrap_or(&default_scorer);
    let mut candidates = super::simple::candidates(start.point, end.point)
        .into_iter()
        .filter(|path| valid(path, start, end, obstacles))
        .collect::<Vec<_>>();
    for peer in quality.peers() {
        for y in [8, 16, 24].into_iter().flat_map(|gap| {
            [
                peer.origin.y.saturating_sub(gap),
                peer.origin.y + peer.height + gap,
            ]
        }) {
            let path = simplify(
                [
                    start.point,
                    first,
                    Point { x: first.x, y },
                    Point { x: last.x, y },
                    last,
                    end.point,
                ],
                Simplification::PreserveReversals,
            );
            if valid(&path, start, end, obstacles) {
                candidates.push(path);
            }
        }
    }
    if let Some(middle) =
        search::search(first, last, obstacles, scorer, Some((start.side, end.side)))
    {
        let path = simplify(
            std::iter::once(start.point)
                .chain(middle)
                .chain(std::iter::once(end.point)),
            Simplification::PreserveReversals,
        );
        if valid(&path, start, end, obstacles) {
            candidates.push(path);
        }
    }
    candidates
        .into_iter()
        .min_by_key(|path| quality.score(path, obstacles))
}

pub(in crate::layout) fn valid(
    path: &[Point],
    start: Port,
    end: Port,
    obstacles: &[Bounds],
) -> bool {
    let forward = |port: Port, other: Point| match port.side {
        Side::Left => other.y == port.point.y && other.x < port.point.x,
        Side::Right => other.y == port.point.y && other.x > port.point.x,
        Side::Top => other.x == port.point.x && other.y < port.point.y,
        Side::Bottom => other.x == port.point.x && other.y > port.point.y,
    };
    path.len() >= 2
        && forward(start, path[1])
        && forward(end, path[path.len() - 2])
        && (path.len() == 2
            || (start.point.x.abs_diff(path[1].x) + start.point.y.abs_diff(path[1].y) >= 16
                && end.point.x.abs_diff(path[path.len() - 2].x)
                    + end.point.y.abs_diff(path[path.len() - 2].y)
                    >= 16))
        && path.windows(2).all(|p| {
            (p[0].x == p[1].x || p[0].y == p[1].y)
                && obstacles.iter().all(|&b| !crosses(p[0], p[1], b))
        })
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

#[test]
fn equal_quality_facing_pairs_have_an_explicit_preference() {
    let source = Bounds {
        origin: Point { x: 100, y: 100 },
        width: 80,
        height: 80,
    };
    let target = Bounds {
        origin: Point { x: 300, y: 300 },
        ..source
    };
    let obstacles = [source, target];
    let scorer = Scorer::default();
    let baseline = connect(
        source.port(Side::Bottom, 40),
        target.port(Side::Top, 40),
        &obstacles,
        None,
    )
    .unwrap();
    let preferred = connect(
        source.port(Side::Right, 40),
        target.port(Side::Left, 40),
        &obstacles,
        None,
    )
    .unwrap();
    assert_eq!(
        scorer.score(&baseline, &obstacles),
        scorer.score(&preferred, &obstacles)
    );
    let chosen = select(source, target, baseline, &obstacles, &scorer);
    assert_eq!(chosen.first(), Some(&source.port(Side::Right, 40).point));
    assert_eq!(chosen.last(), Some(&target.port(Side::Left, 40).point));
}
