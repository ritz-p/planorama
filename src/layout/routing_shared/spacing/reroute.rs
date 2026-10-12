use super::*;
use crate::layout::relationship_markers::{self, Terminal};
use crate::layout::routing_shared::{
    scoring::{Occluder, Scorer},
    shortcuts,
};

fn bundled(layout: &Layout<'_>, index: usize) -> bool {
    let path = &layout.paths[index];
    layout.junctions.iter().any(|j| {
        path.windows(2).any(|p| {
            (p[0].x == j.x
                && p[1].x == j.x
                && (p[0].y.min(p[1].y)..=p[0].y.max(p[1].y)).contains(&j.y))
                || (p[0].y == j.y
                    && p[1].y == j.y
                    && (p[0].x.min(p[1].x)..=p[0].x.max(p[1].x)).contains(&j.x))
        })
    }) || layout.paths.iter().enumerate().any(|(i, p)| {
        i != index && !p.is_empty() && (p.first() == path.first() || p.last() == path.last())
    })
}

pub(super) fn movable(layout: &Layout<'_>, index: usize) -> bool {
    layout.paths[index].len() >= 2 && !bundled(layout, index)
}

fn lane_candidates(
    path: &[Point],
    corridor: &plan::Corridor,
    coordinate: usize,
    clearances: [Terminal; 2],
) -> Vec<Vec<Point>> {
    let start = terminal(path, 0);
    let end = terminal(path, 1);
    let first = start.outward(clearances[0].clearance);
    let last = end.outward(clearances[1].clearance);
    let on_lane = |p: Point| {
        if corridor.horizontal {
            Point {
                x: p.x,
                y: coordinate,
            }
        } else {
            Point {
                x: coordinate,
                y: p.y,
            }
        }
    };
    let mut candidates = vec![
        simplify(
            [
                start.point,
                on_lane(start.point),
                on_lane(end.point),
                end.point,
            ],
            Simplification::PreserveReversals,
        ),
        simplify(
            [
                start.point,
                first,
                on_lane(first),
                on_lane(last),
                last,
                end.point,
            ],
            Simplification::PreserveReversals,
        ),
    ];
    for index in 1..path.len().saturating_sub(2) {
        let (a, b) = (path[index], path[index + 1]);
        let (horizontal, low, high) = if a.y == b.y {
            (true, a.x.min(b.x), a.x.max(b.x))
        } else {
            (false, a.y.min(b.y), a.y.max(b.y))
        };
        if horizontal != corridor.horizontal
            || high
                .min(corridor.high)
                .saturating_sub(low.max(corridor.low))
                < if corridor.border { 1 } else { RUN }
        {
            continue;
        }
        let mut candidate = path.to_vec();
        candidate[index] = on_lane(a);
        candidate[index + 1] = on_lane(b);
        candidates.push(simplify(candidate, Simplification::PreserveReversals));
    }
    candidates
}

pub(super) fn preserves_overlap(
    before: &[Vec<Point>],
    after: &[Vec<Point>],
    order: &[usize],
) -> bool {
    order.iter().all(|&i| {
        let overlap = |paths: &[Vec<Point>]| {
            paths
                .iter()
                .enumerate()
                .filter(|(j, _)| *j != i)
                .map(|(_, p)| super::super::refinement::overlap(&paths[i], p))
                .sum::<usize>()
        };
        overlap(after) <= overlap(before)
    })
}

pub(super) fn apply(
    graph: &Graph,
    layout: &mut Layout<'_>,
    plan: &plan::Corridor,
    before: &expansion::Snapshot,
    baseline: &[Shape],
    expanded: bool,
) -> bool {
    let mut order: Vec<_> = layout
        .paths
        .iter()
        .enumerate()
        .filter(|(i, p)| {
            movable(layout, *i)
                && (plan.lanes.iter().any(|l| l.edge == *i)
                    || (expanded && length(p) != length(&before.paths[*i])))
        })
        .map(|(i, _)| i)
        .collect();
    if order.len() > 32 {
        return false;
    }
    order.sort_by_key(|&i| {
        let e = &graph.edges[i];
        (
            plan.lanes
                .iter()
                .position(|l| l.edge == i)
                .unwrap_or(usize::MAX),
            &graph.nodes[e.from].entity.id,
            &graph.nodes[e.to].entity.id,
            e.kind,
            e.change.as_ref().map(|c| (&c.address, c.action)),
        )
    });
    let numbers = relationship_markers::numbers(graph);
    let mut routed = 0;
    for &i in &order {
        let overlap_before: usize = before
            .paths
            .iter()
            .enumerate()
            .filter(|(j, _)| *j != i)
            .map(|(_, p)| super::super::refinement::overlap(&before.paths[i], p))
            .sum();
        let seed = &layout.paths[i];
        let start = terminal(seed, 0);
        let end = terminal(seed, 1);
        let clearances =
            relationship_markers::terminals(graph, i, numbers[i], [start.side, end.side]);
        let obstacles = if plan.border {
            border_barriers(graph, layout, i)
        } else {
            barriers(graph, layout, i)
        };
        let mut scorer = Scorer::default();
        scorer.set_layout(graph, layout);
        scorer.set_kind(graph.edges[i].kind);
        for (other, path) in layout.paths.iter().enumerate() {
            if other != i {
                scorer.insert(path.clone());
            }
        }
        scorer.routing_axes = plan
            .lanes
            .iter()
            .map(|l| (plan.horizontal, l.coordinate))
            .collect();
        let mut reservations =
            crate::layout::containers::routing::soft_obstacles(layout, &graph.edges[i]);
        reservations.extend(plan.lanes.iter().filter(|l| l.edge != i).map(|l| {
            let low = l.coordinate.saturating_sub(PREFERRED - 1);
            let bounds = if plan.horizontal {
                Bounds {
                    origin: Point {
                        x: plan.low,
                        y: low,
                    },
                    width: plan.high - plan.low,
                    height: 2 * (PREFERRED - 1),
                }
            } else {
                Bounds {
                    origin: Point {
                        x: low,
                        y: plan.low,
                    },
                    width: 2 * (PREFERRED - 1),
                    height: plan.high - plan.low,
                }
            };
            Occluder {
                bounds,
                cost: 1,
                step: 1,
            }
        }));
        scorer.set_soft(reservations);
        let mut candidates = super::super::simple::candidates(start.point, end.point);
        if let Some(lane) = plan.lanes.iter().find(|l| l.edge == i) {
            candidates.extend(lane_candidates(seed, plan, lane.coordinate, clearances));
        }
        if !expanded {
            candidates.push(seed.clone());
        }
        let bound = ports::cost_bound(start, end, clearances);
        if (expanded || plan.border)
            && !candidates.iter().any(|p| {
                let s = Shape::measure(p);
                s.fits(baseline[i])
                    && scorer.attains_bound(p, bound)
                    && ports::valid_with_clearances(p, start, end, &obstacles, clearances)
                    && layout
                        .paths
                        .iter()
                        .enumerate()
                        .filter(|(j, _)| *j != i)
                        .map(|(_, other)| super::super::refinement::overlap(p, other))
                        .sum::<usize>()
                        <= overlap_before
            })
        {
            candidates.extend(ports::candidates_with_clearances(
                start,
                end,
                &obstacles,
                Some(&scorer),
                clearances,
            ));
        }
        let mut best = None;
        for candidate in candidates {
            if !ports::valid_with_clearances(&candidate, start, end, &obstacles, clearances) {
                continue;
            }
            let candidate = shortcuts::simplify_path_with_clearances(
                candidate, &obstacles, &scorer, clearances,
            );
            let shape = Shape::measure(&candidate);
            if !shape.fits(baseline[i])
                || !scorer.safe_border_change(seed, &candidate)
                || candidate
                    .iter()
                    .any(|p| p.x >= layout.width || p.y >= layout.height)
            {
                continue;
            }
            let overlap_after: usize = layout
                .paths
                .iter()
                .enumerate()
                .filter(|(j, _)| *j != i)
                .map(|(_, p)| super::super::refinement::overlap(&candidate, p))
                .sum();
            if overlap_after > overlap_before {
                continue;
            }
            let key = (
                shape.s_jogs,
                shape.offsets,
                shape.reversals,
                shape.bends,
                scorer.readability_cost(&candidate),
            );
            if best.as_ref().is_none_or(|(score, _)| key < *score) {
                best = Some((key, candidate));
            }
        }
        let Some((_, path)) = best else {
            if !expanded {
                continue;
            }
            return false;
        };
        layout.paths[i] = path;
        routed += 1;
    }
    if expanded && routed == 0 {
        return false;
    }
    !super::super::slots::opposite_roles_collide(graph, &layout.paths)
        && preserves_overlap(&before.paths, &layout.paths, &order)
        && layout
            .paths
            .iter()
            .zip(baseline)
            .all(|(p, &s)| Shape::measure(p).fits(s))
}
