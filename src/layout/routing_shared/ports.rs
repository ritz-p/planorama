use super::{Bounds, Point, Scorer, Simplification, crosses, search, simplify};
use crate::layout::{Port, Side, relationship_markers::Terminal};

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

pub(in crate::layout) fn pair_batches(
    source: Bounds,
    target: Bounds,
) -> impl Iterator<Item = Vec<(usize, (Side, Side))>> {
    std::iter::once_with(move || {
        facing_pairs(source, target)
            .into_iter()
            .enumerate()
            .collect()
    })
    .chain(std::iter::once_with(move || {
        let primary = facing_pairs(source, target).len();
        candidate_pairs(source, target)
            .into_iter()
            .enumerate()
            .skip(primary)
            .collect()
    }))
}

#[cfg(test)]
std::thread_local! {
    static CANDIDATE_COUNT: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
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

pub(in crate::layout) fn cost_bound(start: Port, end: Port, terminals: [Terminal; 2]) -> u128 {
    let distance = |a: Point, b: Point| a.x.abs_diff(b.x) as u128 + a.y.abs_diff(b.y) as u128;
    if valid_with_clearances(&[start.point, end.point], start, end, &[], terminals) {
        return distance(start.point, end.point);
    }
    let vertical = |side| matches!(side, Side::Top | Side::Bottom);
    let bends = if vertical(start.side) == vertical(end.side) {
        2
    } else {
        1
    };
    distance(
        start.outward(terminals[0].clearance),
        end.outward(terminals[1].clearance),
    ) + terminals
        .iter()
        .map(|terminal| terminal.clearance)
        .sum::<usize>() as u128
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
    select_candidates(source, target, &baseline, obstacles, scorer, slots).unwrap_or(baseline)
}

pub(in crate::layout) fn select_from_ports(
    source: Bounds,
    target: Bounds,
    ports: [Port; 2],
    obstacles: &[Bounds],
    scorer: &Scorer,
    slots: (&[usize; 4], &[usize; 4]),
) -> Option<Vec<Point>> {
    select_candidates(
        source,
        target,
        &ports.map(|p| p.point),
        obstacles,
        scorer,
        Some(slots),
    )
}

fn select_candidates(
    source: Bounds,
    target: Bounds,
    baseline: &[Point],
    obstacles: &[Bounds],
    scorer: &Scorer,
    slots: Option<(&[usize; 4], &[usize; 4])>,
) -> Option<Vec<Point>> {
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
        return None;
    };
    let mut candidates = Vec::new();
    let mut limit = u128::MAX;
    for batch in pair_batches(source, target) {
        let mut pairs: Vec<_> = batch
            .into_iter()
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
                (
                    cost_bound(start, end, [Terminal::plain(16); 2]),
                    rank,
                    start,
                    end,
                )
            })
            .collect();
        pairs.sort_by_key(|&(bound, rank, _, _)| (bound, rank));
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
                && valid_with_clearances(baseline, start, end, obstacles, [Terminal::plain(16); 2])
            {
                candidates.push((rank, baseline.to_vec()));
            }
            for candidate in candidates_with_clearances(
                start,
                end,
                obstacles,
                Some(scorer),
                [Terminal::plain(16); 2],
            ) {
                if scorer.readability_cost(&candidate) == distance_bound(source, target) {
                    return Some(candidate);
                }
                candidates.push((rank, candidate));
            }
            if let Some(best) =
                scorer.best_index(candidates.iter().map(|(_, p)| p.as_slice()), obstacles)
            {
                limit = scorer.readability_cost(&candidates[best].1);
            }
        }
    }
    candidates.sort_by_key(|(rank, _)| *rank);
    scorer.choose(candidates.into_iter().map(|(_, p)| p).collect(), obstacles)
}

fn on_side(bounds: Bounds, point: Point, side: Side) -> bool {
    match side {
        Side::Left => point.x == bounds.origin.x,
        Side::Right => point.x == bounds.right(),
        Side::Top => point.y == bounds.origin.y,
        Side::Bottom => point.y == bounds.origin.y + bounds.height,
    }
}

pub(in crate::layout) fn connect(
    start: Port,
    end: Port,
    obstacles: &[Bounds],
    scorer: Option<&Scorer>,
) -> Option<Vec<Point>> {
    connect_with_clearances(start, end, obstacles, scorer, [Terminal::plain(16); 2])
}

pub(in crate::layout) fn connect_with_clearances(
    start: Port,
    end: Port,
    obstacles: &[Bounds],
    scorer: Option<&Scorer>,
    terminals: [Terminal; 2],
) -> Option<Vec<Point>> {
    let default_scorer = Scorer::default();
    scorer.unwrap_or(&default_scorer).choose(
        candidates_with_clearances(start, end, obstacles, scorer, terminals),
        obstacles,
    )
}

pub(in crate::layout) fn candidates_with_clearances(
    start: Port,
    end: Port,
    obstacles: &[Bounds],
    scorer: Option<&Scorer>,
    terminals: [Terminal; 2],
) -> Vec<Vec<Point>> {
    candidates_impl::<true>(start, end, obstacles, scorer, terminals)
}

fn candidates_impl<const PRUNE: bool>(
    start: Port,
    end: Port,
    obstacles: &[Bounds],
    scorer: Option<&Scorer>,
    terminals: [Terminal; 2],
) -> Vec<Vec<Point>> {
    #[cfg(test)]
    CANDIDATE_COUNT.set(CANDIDATE_COUNT.get() + 1);
    let first = start.outward(terminals[0].clearance);
    let last = end.outward(terminals[1].clearance);
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
        .filter(|path| valid_with_clearances(path, start, end, obstacles, terminals))
        .collect::<Vec<_>>();
    let tracks =
        quality
            .border_axes(first, last)
            .into_iter()
            .chain(quality.peers().iter().flat_map(|peer| {
                [8, 16, 24].into_iter().flat_map(move |gap| {
                    [
                        (true, peer.origin.y.saturating_sub(gap)),
                        (true, peer.origin.y + peer.height + gap),
                    ]
                })
            }));
    for (horizontal, coordinate) in tracks {
        let bends = if horizontal {
            [
                Point {
                    x: first.x,
                    y: coordinate,
                },
                Point {
                    x: last.x,
                    y: coordinate,
                },
            ]
        } else {
            [
                Point {
                    x: coordinate,
                    y: first.y,
                },
                Point {
                    x: coordinate,
                    y: last.y,
                },
            ]
        };
        let path = simplify(
            [start.point, first, bends[0], bends[1], last, end.point],
            Simplification::PreserveReversals,
        );
        if valid_with_clearances(&path, start, end, obstacles, terminals)
            && !candidates.contains(&path)
        {
            candidates.push(path);
        }
    }
    let bound = cost_bound(start, end, terminals);
    if PRUNE
        && candidates
            .iter()
            .any(|path| quality.attains_bound(path, bound))
    {
        return candidates;
    }
    let mut barriers = obstacles.to_vec();
    barriers.extend(terminal_corridors(start, end, terminals));
    for occupancy in [None, scorer]
        .into_iter()
        .take(if scorer.is_some() { 2 } else { 1 })
    {
        if PRUNE
            && occupancy.is_some()
            && candidates
                .iter()
                .any(|path| quality.attains_bound(path, bound))
        {
            break;
        }
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
            if valid_with_clearances(&path, start, end, obstacles, terminals) {
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
    valid_with_clearances(path, start, end, obstacles, [Terminal::plain(clearance); 2])
}

pub(in crate::layout) fn terminal_corridors(
    start: Port,
    end: Port,
    terminals: [Terminal; 2],
) -> Vec<Bounds> {
    [start, end]
        .into_iter()
        .zip(terminals)
        .filter_map(|(port, terminal)| terminal.corridor(port))
        .collect()
}

pub(in crate::layout) fn valid_with_clearances(
    path: &[Point],
    start: Port,
    end: Port,
    obstacles: &[Bounds],
    terminals: [Terminal; 2],
) -> bool {
    let forward = |port: Port, other: Point| match port.side {
        Side::Left => other.y == port.point.y && other.x < port.point.x,
        Side::Right => other.y == port.point.y && other.x > port.point.x,
        Side::Top => other.x == port.point.x && other.y < port.point.y,
        Side::Bottom => other.x == port.point.x && other.y > port.point.y,
    };
    let corridors = [terminals[0].corridor(start), terminals[1].corridor(end)];
    path.len() >= 2
        && path[0] == start.point
        && path[path.len() - 1] == end.point
        && forward(start, path[1])
        && forward(end, path[path.len() - 2])
        && (path.len() == 2
            || (start.point.x.abs_diff(path[1].x) + start.point.y.abs_diff(path[1].y)
                >= terminals[0].clearance
                && end.point.x.abs_diff(path[path.len() - 2].x)
                    + end.point.y.abs_diff(path[path.len() - 2].y)
                    >= terminals[1].clearance))
        && (path.len() != 2
            || terminals.iter().all(|terminal| terminal.span == 0)
            || start.point.x.abs_diff(end.point.x) + start.point.y.abs_diff(end.point.y)
                >= terminals.iter().map(|terminal| terminal.clearance).sum())
        && path.windows(2).enumerate().all(|(index, p)| {
            (p[0].x == p[1].x || p[0].y == p[1].y)
                && obstacles.iter().all(|&b| !crosses(p[0], p[1], b))
                && corridors.iter().enumerate().all(|(endpoint, corridor)| {
                    (endpoint == 0 && index == 0)
                        || (endpoint == 1 && index + 2 == path.len())
                        || corridor.is_none_or(|bounds| !crosses(p[0], p[1], bounds))
                })
        })
}

#[cfg(test)]
#[path = "../../../tests/unit/layout/routing_shared/ports.rs"]
mod tests;
