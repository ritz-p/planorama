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

pub(in crate::layout) fn candidate_pairs(source: Bounds, target: Bounds) -> Vec<(Side, Side)> {
    let mut pairs = facing_pairs(source, target);
    for a in [Side::Left, Side::Right, Side::Top, Side::Bottom] {
        for b in [Side::Left, Side::Right, Side::Top, Side::Bottom] {
            if !pairs.contains(&(a, b)) {
                pairs.push((a, b));
            }
        }
    }
    pairs
}

pub(in crate::layout) fn distance_bound(source: Bounds, target: Bounds) -> u128 {
    (source.origin.x.saturating_sub(target.right())
        + target.origin.x.saturating_sub(source.right())
        + source
            .origin
            .y
            .saturating_sub(target.origin.y + target.height)
        + target
            .origin
            .y
            .saturating_sub(source.origin.y + source.height)) as u128
}

pub(in crate::layout) fn cost_bound(start: Port, end: Port, clearance: [usize; 2]) -> u128 {
    let distance = |a: Point, b: Point| a.x.abs_diff(b.x) as u128 + a.y.abs_diff(b.y) as u128;
    if valid_with_clearances(&[start.point, end.point], start, end, &[], clearance) {
        return distance(start.point, end.point);
    }
    let vertical = |side| matches!(side, Side::Top | Side::Bottom);
    let bends = if vertical(start.side) == vertical(end.side) {
        2
    } else {
        1
    };
    distance(start.outward(clearance[0]), end.outward(clearance[1]))
        + clearance.iter().sum::<usize>() as u128
        + bends * super::scoring::BEND_COST
}

pub(in crate::layout) fn select_with_slots(
    source: Bounds,
    target: Bounds,
    baseline: Vec<Point>,
    obstacles: &[Bounds],
    scorer: &Scorer,
    slots: Option<(&[usize; 4], &[usize; 4])>,
) -> Vec<Point> {
    let pairs = candidate_pairs(source, target);
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
    let mut pairs: Vec<_> = pairs
        .into_iter()
        .enumerate()
        .map(|(rank, (a, b))| {
            let start = source.port(
                a,
                if on_side(source, first, a) {
                    offset(source, first, a)
                } else {
                    slots.map_or_else(|| offset(source, first, a), |s| s.0[a as usize])
                },
            );
            let end = target.port(
                b,
                if on_side(target, last, b) {
                    offset(target, last, b)
                } else {
                    slots.map_or_else(|| offset(target, last, b), |s| s.1[b as usize])
                },
            );
            (cost_bound(start, end, [16; 2]), rank, start, end)
        })
        .collect();
    pairs.sort_by_key(|&(bound, rank, _, _)| (bound, rank));
    let mut candidates = Vec::new();
    let mut limit = u128::MAX;
    for (bound, rank, start, end) in pairs {
        if bound > limit {
            break;
        }
        if (start.point != first && scorer.uses_endpoint(start.point))
            || (end.point != last && scorer.uses_endpoint(end.point))
        {
            continue;
        }
        if start.point == first
            && end.point == last
            && valid_with_clearances(&baseline, start, end, obstacles, [16; 2])
        {
            candidates.push((rank, baseline.clone()));
        }
        for candidate in candidates_with_clearances(start, end, obstacles, Some(scorer), [16; 2]) {
            if scorer.readability_cost(&candidate) == distance_bound(source, target) {
                return candidate;
            }
            candidates.push((rank, candidate));
        }
        if let Some(best) =
            scorer.best_index(candidates.iter().map(|(_, p)| p.as_slice()), obstacles)
        {
            limit = scorer.readability_cost(&candidates[best].1);
        }
    }
    candidates.sort_by_key(|(rank, _)| *rank);
    scorer
        .choose(candidates.into_iter().map(|(_, p)| p).collect(), obstacles)
        .unwrap_or(baseline)
}

fn on_side(bounds: Bounds, point: Point, side: Side) -> bool {
    match side {
        Side::Left => point.x == bounds.origin.x,
        Side::Right => point.x == bounds.right(),
        Side::Top => point.y == bounds.origin.y,
        Side::Bottom => point.y == bounds.origin.y + bounds.height,
    }
}

#[test]
fn target_side_change_preserves_source_offset() {
    let source = Bounds {
        origin: Point { x: 100, y: 100 },
        width: 200,
        height: 160,
    };
    let target = Bounds {
        origin: Point { x: 600, y: 100 },
        ..source
    };
    let start = source.port(Side::Right, 45);
    let end = target.port(Side::Right, 70);
    let obstacles = [source, target];
    let baseline = connect(start, end, &obstacles, None).unwrap();
    let slots = [100; 4];
    let path = select_with_slots(
        source,
        target,
        baseline,
        &obstacles,
        &Scorer::default(),
        Some((&slots, &slots)),
    );
    assert_eq!(path.first(), Some(&start.point));
    assert_eq!(path.last(), Some(&target.port(Side::Left, 100).point));
}

#[test]
fn side_selection_is_independent_of_the_legacy_pair() {
    let source = Bounds {
        origin: Point { x: 100, y: 100 },
        width: 160,
        height: 160,
    };
    for target in [
        Bounds {
            origin: Point { x: 600, y: 100 },
            ..source
        },
        Bounds {
            origin: Point { x: 100, y: 600 },
            ..source
        },
    ] {
        let obstacles = [source, target];
        let mut expected = None;
        for (a, b) in [
            (Side::Right, Side::Right),
            (Side::Left, Side::Left),
            (Side::Top, Side::Bottom),
        ] {
            let baseline =
                connect(source.port(a, 80), target.port(b, 80), &obstacles, None).unwrap();
            let selected = select(source, target, baseline, &obstacles, &Scorer::default());
            if let Some(expected) = &expected {
                assert_eq!(&selected, expected);
            }
            expected = Some(selected);
        }
    }
}

#[test]
fn blocked_facing_ports_choose_a_shorter_valid_alternative() {
    let source = Bounds {
        origin: Point { x: 100, y: 100 },
        width: 100,
        height: 100,
    };
    let target = Bounds {
        origin: Point { x: 600, y: 100 },
        ..source
    };
    let blocker = Bounds {
        origin: Point { x: 205, y: 90 },
        width: 30,
        height: 130,
    };
    let obstacles = [source, target, blocker];
    assert!(
        connect(
            source.port(Side::Right, 50),
            target.port(Side::Left, 50),
            &obstacles,
            None
        )
        .is_none()
    );
    let baseline = connect(
        source.port(Side::Left, 50),
        target.port(Side::Right, 50),
        &obstacles,
        None,
    )
    .unwrap();
    let scorer = Scorer::default();
    let selected = select(source, target, baseline.clone(), &obstacles, &scorer);
    assert!(scorer.readability_cost(&selected) < scorer.readability_cost(&baseline));
    assert!(
        selected
            .windows(2)
            .all(|p| obstacles.iter().all(|&b| !crosses(p[0], p[1], b)))
    );
}

#[test]
fn bounded_selection_matches_exhaustive_candidate_cost() {
    let source = Bounds {
        origin: Point { x: 100, y: 100 },
        width: 100,
        height: 100,
    };
    let target = Bounds {
        origin: Point { x: 500, y: 300 },
        ..source
    };
    for blocker in [
        Bounds {
            origin: Point { x: 250, y: 120 },
            width: 100,
            height: 250,
        },
        Bounds {
            origin: Point { x: 350, y: 210 },
            width: 120,
            height: 40,
        },
    ] {
        let obstacles = [source, target, blocker];
        let mut scorer = Scorer::default();
        scorer.insert(vec![Point { x: 80, y: 220 }, Point { x: 650, y: 220 }]);
        let baseline = connect(
            source.port(Side::Right, 50),
            target.port(Side::Right, 50),
            &obstacles,
            None,
        )
        .unwrap();
        let selected = select(source, target, baseline, &obstacles, &scorer);
        let mut all = Vec::new();
        for (a, b) in candidate_pairs(source, target) {
            let start = source.port(a, 50);
            let end = target.port(b, 50);
            for path in candidates_with_clearances(start, end, &obstacles, Some(&scorer), [16; 2]) {
                assert!(
                    cost_bound(start, end, [16; 2]) <= Scorer::default().readability_cost(&path)
                );
                all.push(path);
            }
        }
        let exhaustive = scorer.choose(all, &obstacles).unwrap();
        assert_eq!(
            scorer.score(&selected, &obstacles),
            scorer.score(&exhaustive, &obstacles)
        );
    }
}

pub(in crate::layout) fn connect(
    start: Port,
    end: Port,
    obstacles: &[Bounds],
    scorer: Option<&Scorer>,
) -> Option<Vec<Point>> {
    connect_with_clearances(start, end, obstacles, scorer, [16; 2])
}

pub(in crate::layout) fn connect_with_clearances(
    start: Port,
    end: Port,
    obstacles: &[Bounds],
    scorer: Option<&Scorer>,
    clearances: [usize; 2],
) -> Option<Vec<Point>> {
    let default_scorer = Scorer::default();
    scorer.unwrap_or(&default_scorer).choose(
        candidates_with_clearances(start, end, obstacles, scorer, clearances),
        obstacles,
    )
}

pub(in crate::layout) fn candidates_with_clearances(
    start: Port,
    end: Port,
    obstacles: &[Bounds],
    scorer: Option<&Scorer>,
    clearances: [usize; 2],
) -> Vec<Vec<Point>> {
    let first = start.outward(clearances[0]);
    let last = end.outward(clearances[1]);
    if first == start.point
        || last == end.point
        || obstacles
            .iter()
            .any(|&b| crosses(start.point, first, b) || crosses(last, end.point, b))
    {
        return Vec::new();
    }
    let default_scorer = Scorer::default();
    let quality = scorer.unwrap_or(&default_scorer);
    let mut candidates = super::simple::candidates(start.point, end.point)
        .into_iter()
        .filter(|path| valid_with_clearances(path, start, end, obstacles, clearances))
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
            if valid_with_clearances(&path, start, end, obstacles, clearances) {
                candidates.push(path);
            }
        }
    }
    let mut barriers = obstacles.to_vec();
    barriers.extend(terminal_corridors(start, end, clearances));
    for occupancy in [None, scorer]
        .into_iter()
        .take(if scorer.is_some() { 2 } else { 1 })
    {
        if let Some(middle) = search::search(
            first,
            last,
            &barriers,
            occupancy,
            Some((start.side, end.side)),
        ) {
            let path = simplify(
                std::iter::once(start.point)
                    .chain(middle)
                    .chain(std::iter::once(end.point)),
                Simplification::PreserveReversals,
            );
            if valid_with_clearances(&path, start, end, obstacles, clearances) {
                candidates.push(path);
            }
        }
    }
    candidates
}

#[cfg(test)]
pub(in crate::layout) fn valid(
    path: &[Point],
    start: Port,
    end: Port,
    obstacles: &[Bounds],
) -> bool {
    valid_with_clearance(path, start, end, obstacles, 16)
}

pub(in crate::layout) fn valid_with_clearance(
    path: &[Point],
    start: Port,
    end: Port,
    obstacles: &[Bounds],
    clearance: usize,
) -> bool {
    valid_with_clearances(path, start, end, obstacles, [clearance; 2])
}

pub(in crate::layout) fn terminal_corridors(
    start: Port,
    end: Port,
    clearances: [usize; 2],
) -> Vec<Bounds> {
    [start, end]
        .into_iter()
        .zip(clearances)
        .filter(|&(_, clearance)| clearance > 16)
        .map(|(port, clearance)| crate::layout::relationship_markers::corridor(port, clearance))
        .collect()
}

pub(in crate::layout) fn valid_with_clearances(
    path: &[Point],
    start: Port,
    end: Port,
    obstacles: &[Bounds],
    clearances: [usize; 2],
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
            || (start.point.x.abs_diff(path[1].x) + start.point.y.abs_diff(path[1].y)
                >= clearances[0]
                && end.point.x.abs_diff(path[path.len() - 2].x)
                    + end.point.y.abs_diff(path[path.len() - 2].y)
                    >= clearances[1]))
        && (path.len() != 2
            || clearances == [16; 2]
            || start.point.x.abs_diff(end.point.x) + start.point.y.abs_diff(end.point.y)
                >= clearances.iter().sum())
        && (path.len() < 3
            || path[1..path.len() - 1].windows(2).all(|p| {
                terminal_corridors(start, end, clearances)
                    .iter()
                    .all(|&b| !crosses(p[0], p[1], b))
            }))
        && path.windows(2).all(|p| {
            (p[0].x == p[1].x || p[0].y == p[1].y)
                && obstacles.iter().all(|&b| !crosses(p[0], p[1], b))
        })
}

#[test]
fn soft_cards_allow_large_savings_but_prefer_small_detours() {
    let start = Port {
        point: Point { x: 100, y: 500 },
        side: Side::Right,
    };
    let end = Port {
        point: Point { x: 600, y: 500 },
        side: Side::Left,
    };
    for (height, crossed) in [(40, false), (900, true)] {
        let card = Bounds {
            origin: Point {
                x: 300,
                y: 500 - height / 2,
            },
            width: 240,
            height,
        };
        let mut scorer = Scorer::default();
        scorer.set_soft(vec![card]);
        let path = connect(start, end, &[], Some(&scorer)).unwrap();
        assert_eq!(
            path.windows(2).any(|p| crosses(p[0], p[1], card)),
            crossed,
            "{path:?}"
        );
        let shortened = super::shortcuts::simplify_path(path, &[], &scorer);
        assert_eq!(
            shortened.windows(2).any(|p| crosses(p[0], p[1], card)),
            crossed
        );
        let hard = connect(start, end, &[card], Some(&scorer)).unwrap();
        let hard = super::shortcuts::simplify_path(hard, &[card], &scorer);
        assert!(hard.windows(2).all(|p| !crosses(p[0], p[1], card)));
    }
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
        if source.width == target.width && source.height == target.height {
            assert_eq!(
                chosen.first(),
                Some(&source.port(from_side, offset(source, from_side)).point)
            );
            assert_eq!(
                chosen.last(),
                Some(&target.port(to_side, offset(target, to_side)).point)
            );
        } else {
            assert!(
                Scorer::default().readability_cost(&chosen)
                    <= Scorer::default().readability_cost(&baseline)
            );
        }
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
fn perpendicular_pair_beats_equal_quality_facing_pairs() {
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
    assert_eq!(chosen.last(), Some(&target.port(Side::Top, 40).point));
    assert!(scorer.readability_cost(&chosen) < scorer.readability_cost(&preferred));
}
