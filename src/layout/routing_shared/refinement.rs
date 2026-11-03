use super::{crosses, ports, scoring::Scorer, shortcuts, slots};
use crate::layout::relationship_markers::{self, Terminal};
use crate::layout::{Bounds, Layout, Point, Port, Side};
use crate::model::Graph;
use std::collections::{BTreeMap, VecDeque};

type Exchange = [(usize, usize); 2];
const EXCHANGE_BUDGET: usize = 32;
const ADDITIONAL_EXCHANGE_BUDGET: usize = 8;

fn improves_conflicts<'a>(
    old: [&[Point]; 2],
    new: [&[Point]; 2],
    peers: impl Iterator<Item = &'a [Point]>,
    preserve_pairs: bool,
) -> bool {
    let mut totals = [0; 4];
    let mut compare = |old_a: &[Point], old_b: &[Point], new_a: &[Point], new_b: &[Point]| {
        let before = crossings(old_a, old_b);
        let after = crossings(new_a, new_b);
        let before_overlap = overlap(old_a, old_b);
        let after_overlap = overlap(new_a, new_b);
        for (total, value) in totals
            .iter_mut()
            .zip([before, after, before_overlap, after_overlap])
        {
            *total += value;
        }
        !preserve_pairs || (after <= before && after_overlap <= before_overlap)
    };
    if !compare(old[0], old[1], new[0], new[1]) {
        return false;
    }
    for peer in peers {
        for (old, new) in old.into_iter().zip(new) {
            if !compare(old, peer, new, peer) {
                return false;
            }
        }
    }
    totals[1] < totals[0] && totals[3] <= totals[2]
}

async fn exchanges_step<'a>(
    paths: &[Vec<Point>],
    groups: impl Iterator<Item = &'a Vec<(usize, usize)>>,
) -> Vec<Exchange> {
    let mut queues = Vec::new();
    for ends in groups {
        let mut candidates = Vec::new();
        for (left, &a) in ends.iter().enumerate() {
            for &b in ends.iter().skip(left + 1).take(8) {
                crate::work::checkpoint().await;
                if a.0 == b.0 {
                    continue;
                }
                let count = crossings(&paths[a.0], &paths[b.0]);
                if count > 0 {
                    candidates.push((count, [a, b]));
                }
            }
        }
        candidates.sort_by_key(|&(count, _)| std::cmp::Reverse(count));
        candidates.truncate(EXCHANGE_BUDGET);
        if !candidates.is_empty() {
            queues.push(candidates);
        }
    }
    queues.sort_by_key(|queue| std::cmp::Reverse(queue[0].0));
    let mut exchanges = Vec::new();
    for round in 0..EXCHANGE_BUDGET {
        for queue in &queues {
            crate::work::checkpoint().await;
            if let Some(&(_, pair)) = queue.get(round) {
                exchanges.push(pair);
            }
        }
    }
    exchanges
}

pub(in crate::layout) fn terminal(path: &[Point], end: usize) -> Port {
    let (point, next) = if end == 0 {
        (path[0], path[1])
    } else {
        (path[path.len() - 1], path[path.len() - 2])
    };
    Port {
        point,
        side: if next.x < point.x {
            Side::Left
        } else if next.x > point.x {
            Side::Right
        } else if next.y < point.y {
            Side::Top
        } else {
            Side::Bottom
        },
    }
}

fn length(path: &[Point]) -> usize {
    path.windows(2)
        .map(|p| p[0].x.abs_diff(p[1].x) + p[0].y.abs_diff(p[1].y))
        .sum()
}

fn crossings(a: &[Point], b: &[Point]) -> usize {
    a.windows(2)
        .map(|a| {
            b.windows(2)
                .filter(|b| {
                    let (h, v) = if a[0].y == a[1].y && b[0].x == b[1].x {
                        (a, *b)
                    } else if b[0].y == b[1].y && a[0].x == a[1].x {
                        (*b, a)
                    } else {
                        return false;
                    };
                    h[0].x.min(h[1].x) < v[0].x
                        && v[0].x < h[0].x.max(h[1].x)
                        && v[0].y.min(v[1].y) < h[0].y
                        && h[0].y < v[0].y.max(v[1].y)
                })
                .count()
        })
        .sum()
}

pub(super) fn overlap(a: &[Point], b: &[Point]) -> usize {
    a.windows(2)
        .map(|a| {
            b.windows(2)
                .map(|b| {
                    let (a0, a1, b0, b1) =
                        if a[0].y == a[1].y && a[0].y == b[0].y && b[0].y == b[1].y {
                            (a[0].x, a[1].x, b[0].x, b[1].x)
                        } else if a[0].x == a[1].x && a[0].x == b[0].x && b[0].x == b[1].x {
                            (a[0].y, a[1].y, b[0].y, b[1].y)
                        } else {
                            return 0;
                        };
                    a0.max(a1)
                        .min(b0.max(b1))
                        .saturating_sub(a0.min(a1).max(b0.min(b1)))
                })
                .sum::<usize>()
        })
        .sum()
}

#[cfg(test)]
pub(in crate::layout) fn refine(graph: &Graph, layout: &mut Layout<'_>) {
    crate::work::complete(async {
        refine_step(graph, layout).await;
        extend_step(graph, layout).await;
    })
}

pub(in crate::layout) async fn refine_step(graph: &Graph, layout: &mut Layout<'_>) {
    refine_pass_step(graph, layout, false).await;
}

pub(in crate::layout) async fn extend_step(graph: &Graph, layout: &mut Layout<'_>) {
    refine_pass_step(graph, layout, true).await;
}

async fn refine_pass_step(graph: &Graph, layout: &mut Layout<'_>, preserve_pairs: bool) {
    crate::work::checkpoint().await;
    let numbers = relationship_markers::numbers(graph);
    let clearances: Vec<_> = layout
        .paths
        .iter()
        .enumerate()
        .map(|(i, path)| {
            let sides = if path.len() >= 2 {
                [terminal(path, 0).side, terminal(path, 1).side]
            } else {
                [Side::Right; 2]
            };
            relationship_markers::terminals(graph, i, numbers[i], sides)
        })
        .collect();
    let mut groups = BTreeMap::<_, Vec<_>>::new();
    for (i, (edge, path)) in graph.edges.iter().zip(&layout.paths).enumerate() {
        crate::work::checkpoint().await;
        if path.len() < 2 {
            continue;
        }
        if layout.junctions.iter().any(|j| {
            path.windows(2).any(|p| {
                (p[0].x == j.x
                    && p[1].x == j.x
                    && (p[0].y.min(p[1].y)..=p[0].y.max(p[1].y)).contains(&j.y))
                    || (p[0].y == j.y
                        && p[1].y == j.y
                        && (p[0].x.min(p[1].x)..=p[0].x.max(p[1].x)).contains(&j.x))
            })
        }) {
            continue;
        }
        for (end, node) in [edge.from, edge.to].into_iter().enumerate() {
            crate::work::checkpoint().await;
            if layout.containers.contains(&node) {
                continue;
            }
            let port = terminal(path, end);
            groups
                .entry((graph.nodes[node].entity.id.as_str(), port.side as usize))
                .or_default()
                .push((i, end));
        }
    }
    let sort_ends = |ends: &mut Vec<(usize, usize)>, paths: &[Vec<Point>]| {
        ends.sort_by_key(|&(i, end)| {
            let p = terminal(&paths[i], end);
            let e = &graph.edges[i];
            (
                p.point.x,
                p.point.y,
                graph.nodes[e.from].entity.id.as_str(),
                graph.nodes[e.to].entity.id.as_str(),
                e.kind,
                end,
            )
        });
    };
    let mut attempts = 0;
    let mut scorer = None;
    let mut exchanges = if preserve_pairs {
        for ends in groups.values_mut() {
            crate::work::checkpoint().await;
            sort_ends(ends, &layout.paths);
        }
        VecDeque::from(exchanges_step(&layout.paths, groups.values()).await)
    } else {
        VecDeque::new()
    };
    let mut remaining_groups = groups.values_mut();
    let budget = if preserve_pairs {
        ADDITIONAL_EXCHANGE_BUDGET
    } else {
        EXCHANGE_BUDGET
    };
    loop {
        crate::work::checkpoint().await;
        let pair = match exchanges.pop_front() {
            Some(pair) => pair,
            None if preserve_pairs => break,
            None => {
                let Some(ends) = remaining_groups.next() else {
                    break;
                };
                sort_ends(ends, &layout.paths);
                exchanges.extend(ends.windows(2).map(|pair| [pair[0], pair[1]]));
                continue;
            }
        };
        let [(a, ae), (b, be)] = [pair[0], pair[1]];
        if a == b || crossings(&layout.paths[a], &layout.paths[b]) == 0 {
            continue;
        }
        if attempts >= budget {
            return;
        }
        attempts += 1;
        let mut terminals: Vec<_> = layout
            .paths
            .iter()
            .map(|p| (p.len() >= 2).then(|| [terminal(p, 0), terminal(p, 1)]))
            .collect();
        let ap = terminals[a].unwrap()[ae];
        let bp = terminals[b].unwrap()[be];
        if ap.point == bp.point
            || terminals.iter().enumerate().any(|(i, p)| {
                i != a
                    && i != b
                    && p.is_some_and(|p| {
                        p.iter().any(|p| p.point == ap.point || p.point == bp.point)
                    })
            })
        {
            continue;
        }
        terminals[a].as_mut().unwrap()[ae] = bp;
        terminals[b].as_mut().unwrap()[be] = ap;
        if scorer.is_none() {
            let mut initial = Scorer::default();
            initial.set_layout(graph, layout);
            for path in &layout.paths {
                crate::work::checkpoint().await;
                initial.insert(path.clone());
            }
            scorer = Some(initial);
        }
        let scorer = scorer.as_mut().unwrap();
        scorer.remove(a);
        scorer.remove(b);
        let mut replacements = Vec::new();
        for i in [a, b] {
            crate::work::checkpoint().await;
            let edge = &graph.edges[i];
            scorer.set_kind(edge.kind);
            scorer.set_soft(crate::layout::containers::routing::soft_obstacles(
                layout, edge,
            ));
            let mut obstacles = crate::layout::containers::routing::obstacles(layout, edge);
            obstacles.extend(
                layout
                    .bounds
                    .iter()
                    .enumerate()
                    .filter(|(n, _)| !layout.containers.contains(n))
                    .map(|(_, b)| *b),
            );
            obstacles.extend(layout.occluders(edge).into_iter().map(|n| layout.bounds[n]));
            let [start, end] = terminals[i].unwrap();
            if ![edge.from, edge.to]
                .into_iter()
                .zip([start, end])
                .enumerate()
                .all(|(end, (node, port))| {
                    ports::embedded_marker_fits(graph, layout, node, numbers[i][end], port)
                })
            {
                break;
            }
            if ports::terminal_corridors(start, end, clearances[i])
                .iter()
                .any(|a| {
                    obstacles.iter().any(|b| {
                        a.origin.x < b.right()
                            && b.origin.x < a.right()
                            && a.origin.y < b.origin.y + b.height
                            && b.origin.y < a.origin.y + a.height
                    })
                })
            {
                break;
            }
            for (other, endpoints) in terminals.iter().enumerate() {
                crate::work::checkpoint().await;
                if other == i {
                    continue;
                }
                if let Some(p) = endpoints {
                    obstacles.extend(ports::terminal_corridors(p[0], p[1], clearances[other]));
                    obstacles.extend(p.iter().map(|p| Bounds {
                        origin: Point {
                            x: p.point.x.saturating_sub(1),
                            y: p.point.y.saturating_sub(1),
                        },
                        width: 2,
                        height: 2,
                    }));
                }
            }
            let Some(path) = ports::connect_with_clearances_step(
                start,
                end,
                &obstacles,
                Some(scorer),
                clearances[i],
            )
            .await
            else {
                break;
            };
            let path =
                shortcuts::simplify_path_with_clearances(path, &obstacles, scorer, clearances[i]);
            if !scorer.safe_border_change(&layout.paths[i], &path)
                || path
                    .iter()
                    .any(|p| p.x >= layout.width || p.y >= layout.height)
            {
                break;
            }
            scorer.replace(i, path.clone());
            replacements.push(path);
        }
        scorer.replace(a, layout.paths[a].clone());
        scorer.replace(b, layout.paths[b].clone());
        if replacements.len() != 2 {
            continue;
        }
        let old = [&layout.paths[a], &layout.paths[b]];
        if replacements.iter().map(|p| length(p)).sum::<usize>()
            > old.iter().map(|p| length(p)).sum::<usize>()
            || replacements.iter().map(Vec::len).sum::<usize>()
                > old.iter().map(|p| p.len()).sum::<usize>()
        {
            continue;
        }
        if !improves_conflicts(
            [old[0], old[1]],
            [&replacements[0], &replacements[1]],
            layout
                .paths
                .iter()
                .enumerate()
                .filter(|(i, _)| *i != a && *i != b)
                .map(|(_, path)| path.as_slice()),
            preserve_pairs,
        ) {
            continue;
        }
        if preserve_pairs
            && !crate::layout::diagnostics::preserves_visible_pairs(
                graph,
                layout,
                [a, b],
                &replacements,
            )
        {
            continue;
        }
        let old_a = std::mem::replace(&mut layout.paths[a], replacements.remove(0));
        let old_b = std::mem::replace(&mut layout.paths[b], replacements.remove(0));
        let sides: Vec<_> = layout
            .paths
            .iter()
            .map(|p| {
                if p.len() < 2 {
                    [Side::Right; 2]
                } else {
                    [terminal(p, 0).side, terminal(p, 1).side]
                }
            })
            .collect();
        if slots::crowded(graph, &layout.paths, &sides)
            || marker_collision(&layout.paths, &terminals, &clearances)
        {
            layout.paths[a] = old_a;
            layout.paths[b] = old_b;
        } else {
            scorer.replace(a, layout.paths[a].clone());
            scorer.replace(b, layout.paths[b].clone());
        }
    }
}

fn marker_collision(
    paths: &[Vec<Point>],
    terminals: &[Option<[Port; 2]>],
    clearances: &[[Terminal; 2]],
) -> bool {
    let reservations: Vec<_> = terminals
        .iter()
        .enumerate()
        .map(|(i, p)| {
            p.map_or_else(Vec::new, |p| {
                ports::terminal_corridors(p[0], p[1], clearances[i])
            })
        })
        .collect();
    reservations.iter().enumerate().any(|(i, own)| {
        own.iter().any(|a| {
            reservations.iter().skip(i + 1).flatten().any(|b| {
                a.origin.x < b.right()
                    && b.origin.x < a.right()
                    && a.origin.y < b.origin.y + b.height
                    && b.origin.y < a.origin.y + a.height
            })
        }) || paths.iter().enumerate().any(|(other, path)| {
            other != i
                && path
                    .windows(2)
                    .any(|p| own.iter().any(|&b| crosses(p[0], p[1], b)))
        })
    })
}

#[cfg(test)]
#[path = "../../../tests/unit/layout/routing_shared/refinement.rs"]
mod tests;
