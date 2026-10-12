use super::{
    Bounds, Layout, Port, Scorer, Side, obstacles, represented_by_nesting, soft_obstacles,
};
use crate::layout::relationship_markers::{self, Terminal};
use crate::layout::routing_shared::ports;
use crate::model::architecture::Graph;

pub(super) struct Terminals {
    pub ports: Vec<[Port; 2]>,
    pub clearances: Vec<[Terminal; 2]>,
    reservations: Vec<Vec<Bounds>>,
}

impl Terminals {
    #[cfg(test)]
    pub fn new(
        graph: &Graph,
        layout: &Layout<'_>,
        order: &[usize],
        offsets: (&[usize], &[usize]),
        slots: (&[[usize; 4]], &[[usize; 4]]),
        quality: bool,
    ) -> Self {
        Self::with_sides(graph, layout, order, offsets, slots, quality, None)
    }

    pub fn with_sides(
        graph: &Graph,
        layout: &Layout<'_>,
        order: &[usize],
        offsets: (&[usize], &[usize]),
        slots: (&[[usize; 4]], &[[usize; 4]]),
        quality: bool,
        sides: Option<&[[Side; 2]]>,
    ) -> Self {
        let numbers = relationship_markers::numbers(graph);
        let ports: Vec<_> = graph
            .edges
            .iter()
            .enumerate()
            .map(|(i, e)| {
                [
                    layout.bounds[e.from]
                        .port(sides.map_or(Side::Right, |s| s[i][0]), offsets.0[i]),
                    layout.bounds[e.to].port(sides.map_or(Side::Right, |s| s[i][1]), offsets.1[i]),
                ]
            })
            .collect();
        let clearances: Vec<_> = numbers
            .iter()
            .enumerate()
            .map(|(i, &numbers)| {
                relationship_markers::terminals(graph, i, numbers, ports[i].map(|p| p.side))
            })
            .collect();
        let reservations = ports
            .iter()
            .enumerate()
            .map(|(i, p)| {
                if represented_by_nesting(&graph.edges[i], &layout.containment) {
                    Vec::new()
                } else {
                    ports::terminal_corridors(p[0], p[1], clearances[i])
                }
            })
            .collect();
        let mut result = Self {
            ports,
            clearances,
            reservations,
        };
        if !quality {
            return result;
        }
        let alternative_slots = sides.map(|_| {
            crate::layout::routing_shared::slots::assign(
                graph,
                &layout.bounds,
                &super::incidents(graph, &layout.containment),
            )
        });
        let slots = alternative_slots
            .as_ref()
            .map_or(slots, |s| (s.0.as_slice(), s.1.as_slice()));
        let mut scorer = Scorer::default();
        scorer.set_layout(graph, layout);
        let mut selected = vec![Vec::new(); graph.edges.len()];
        for _ in &graph.edges {
            scorer.insert(Vec::new());
        }
        let mut reconsidered = 0;
        for sweep in 0..2 {
            for &index in order {
                let edge = &graph.edges[index];
                if numbers[index][1].is_none() || represented_by_nesting(edge, &layout.containment)
                {
                    continue;
                }
                if sweep == 1 {
                    if selected[index].len() == 2 {
                        continue;
                    }
                    if reconsidered >= 32 {
                        break;
                    }
                    reconsidered += 1;
                }
                scorer.remove(index);
                let barriers = result.barriers(layout, graph, index);
                scorer.set_kind(edge.kind);
                scorer.set_soft(soft_obstacles(layout, edge));
                scorer.set_peers(layout.containment.routing_peers(
                    edge.from,
                    edge.to,
                    &layout.bounds,
                ));
                let old = result.ports[index];
                let mut candidates = Vec::new();
                let mut limit = u128::MAX;
                if result.fits(
                    graph,
                    layout,
                    index,
                    old,
                    result.clearances[index],
                    numbers[index],
                ) && ports::valid_with_clearances(
                    &selected[index],
                    old[0],
                    old[1],
                    &barriers,
                    result.clearances[index],
                ) {
                    candidates.push((
                        usize::MAX,
                        selected[index].clone(),
                        old,
                        result.clearances[index],
                    ));
                    limit = scorer.readability_cost(&selected[index]);
                }
                'batches: for batch in
                    ports::pair_batches(layout.bounds[edge.from], layout.bounds[edge.to])
                {
                    let mut options: Vec<_> = batch
                        .into_iter()
                        .flat_map(|(rank, (a, b))| {
                            let pair = [
                                if a == old[0].side {
                                    old[0]
                                } else {
                                    layout.bounds[edge.from].port(a, slots.0[index][a as usize])
                                },
                                if b == old[1].side {
                                    old[1]
                                } else {
                                    layout.bounds[edge.to].port(b, slots.1[index][b as usize])
                                },
                            ];
                            let clearances = relationship_markers::terminals(
                                graph,
                                index,
                                numbers[index],
                                [a, b],
                            );
                            aligned_pairs(pair).into_iter().enumerate().map(
                                move |(offset, pair)| {
                                    (
                                        ports::cost_bound(pair[0], pair[1], clearances),
                                        rank * 4 + offset,
                                        pair,
                                        clearances,
                                    )
                                },
                            )
                        })
                        .collect();
                    options.sort_by_key(|&(bound, rank, _, _)| (bound, rank));
                    for (bound, rank, pair, clearances) in options {
                        if bound > limit {
                            break;
                        }
                        if !result.fits(graph, layout, index, pair, clearances, numbers[index]) {
                            continue;
                        }
                        if pair.iter().zip(old).any(|(candidate, previous)| {
                            candidate.point != previous.point
                                && result.ports.iter().enumerate().any(|(other, ports)| {
                                    other != index
                                        && !represented_by_nesting(
                                            &graph.edges[other],
                                            &layout.containment,
                                        )
                                        && ports.iter().any(|port| port.point == candidate.point)
                                })
                        }) {
                            continue;
                        }
                        let corridors = ports::terminal_corridors(pair[0], pair[1], clearances);
                        let mut protected = barriers.clone();
                        protected
                            .extend(soft_obstacles(layout, edge).into_iter().map(|o| o.bounds));
                        if corridors
                            .iter()
                            .any(|&a| protected.iter().any(|&b| overlaps(a, b)))
                        {
                            continue;
                        }
                        let mut optimal = false;
                        for path in ports::candidates_with_clearances(
                            pair[0],
                            pair[1],
                            &barriers,
                            Some(&scorer),
                            clearances,
                        ) {
                            optimal = scorer.readability_cost(&path)
                                == ports::distance_bound(
                                    layout.bounds[edge.from],
                                    layout.bounds[edge.to],
                                );
                            candidates.push((rank, path, pair, clearances));
                            if optimal {
                                break;
                            }
                        }
                        if optimal {
                            break 'batches;
                        }
                        if let Some(best) = scorer.best_index(
                            candidates.iter().map(|(_, p, _, _)| p.as_slice()),
                            &barriers,
                        ) {
                            limit = scorer.readability_cost(&candidates[best].1);
                        }
                    }
                }
                candidates.sort_by_key(|(rank, _, _, _)| *rank);
                if let Some(best) = scorer.best_index(
                    candidates.iter().map(|(_, p, _, _)| p.as_slice()),
                    &barriers,
                ) {
                    let (_, path, pair, clearances) = candidates.swap_remove(best);
                    result.ports[index] = pair;
                    result.clearances[index] = clearances;
                    result.reservations[index] =
                        ports::terminal_corridors(pair[0], pair[1], clearances);
                    selected[index] = path;
                }
                scorer.replace(index, selected[index].clone());
            }
        }
        result
    }

    fn fits(
        &self,
        graph: &Graph,
        layout: &Layout<'_>,
        index: usize,
        pair: [Port; 2],
        clearances: [Terminal; 2],
        numbers: [Option<usize>; 2],
    ) -> bool {
        let edge = &graph.edges[index];
        pair.into_iter()
            .zip([edge.from, edge.to])
            .enumerate()
            .all(|(end, (port, node))| {
                let bounds = layout.bounds[node];
                let vertical = matches!(port.side, Side::Top | Side::Bottom);
                let (position, low, high) = if vertical {
                    (port.point.x, bounds.origin.x, bounds.right())
                } else {
                    (
                        port.point.y,
                        bounds.origin.y,
                        bounds.origin.y + bounds.height,
                    )
                };
                let margin = 12.max(clearances[end].span.div_ceil(2));
                position >= low + margin
                    && position + margin <= high
                    && ports::embedded_marker_fits(graph, layout, node, numbers[end], port)
                    && self.ports.iter().enumerate().all(|(other, ports)| {
                        if other == index
                            || represented_by_nesting(&graph.edges[other], &layout.containment)
                        {
                            return true;
                        }
                        ports.iter().enumerate().all(|(other_end, p)| {
                            let other_node = if other_end == 0 {
                                graph.edges[other].from
                            } else {
                                graph.edges[other].to
                            };
                            if other_node != node || p.side != port.side {
                                return true;
                            }
                            let distance = if vertical {
                                p.point.x.abs_diff(position)
                            } else {
                                p.point.y.abs_diff(position)
                            };
                            let required = (clearances[end].span.max(16)
                                + self.clearances[other][other_end].span.max(16))
                            .div_ceil(2);
                            distance >= required
                        })
                    })
            })
    }

    pub fn barriers(&self, layout: &Layout<'_>, graph: &Graph, index: usize) -> Vec<Bounds> {
        let mut barriers = obstacles(layout, &graph.edges[index]);
        if let Some(parent) =
            crate::view::routes::owner(graph, &graph.nodes[graph.edges[index].from])
        {
            barriers.extend(
                layout.containment.children[parent]
                    .iter()
                    .filter(|&&child| child != graph.edges[index].from)
                    .map(|&child| layout.bounds[child]),
            );
            let mut y = layout.bounds[parent].origin.y
                + crate::view::card::measure(&graph.nodes[parent]).routes_y
                + 12;
            for card in crate::view::routes::cards(graph, parent) {
                if card.node.is_some() {
                    break;
                }
                let height = card.height();
                barriers.push(Bounds {
                    origin: crate::layout::Point {
                        x: layout.bounds[parent].origin.x + 12,
                        y,
                    },
                    width: crate::view::routes::WIDTH,
                    height,
                });
                y += height + 12;
            }
        }
        let reserved = self
            .reservations
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != index)
            .flat_map(|(_, b)| b.iter().copied())
            .collect();
        barriers.extend(relationship_markers::merge_corridors(reserved));
        barriers
    }

    pub fn all(&self) -> Vec<Bounds> {
        relationship_markers::merge_corridors(self.reservations.iter().flatten().copied().collect())
    }
}

fn overlaps(a: Bounds, b: Bounds) -> bool {
    a.origin.x < b.right()
        && b.origin.x < a.right()
        && a.origin.y < b.origin.y + b.height
        && b.origin.y < a.origin.y + a.height
}

fn aligned_pairs(pair: [Port; 2]) -> Vec<[Port; 2]> {
    let vertical = |port: Port| matches!(port.side, Side::Top | Side::Bottom);
    if vertical(pair[0]) != vertical(pair[1]) {
        return vec![pair];
    }
    let axis = |p: Port| if vertical(p) { p.point.x } else { p.point.y };
    let set_axis = |mut p: Port, coordinate| {
        if vertical(p) {
            p.point.x = coordinate;
        } else {
            p.point.y = coordinate;
        }
        p
    };
    let a = axis(pair[0]);
    let b = axis(pair[1]);
    let middle = a.min(b) + a.abs_diff(b) / 2;
    let mut result = vec![pair];
    for candidate in [
        [set_axis(pair[0], b), pair[1]],
        [pair[0], set_axis(pair[1], a)],
        [set_axis(pair[0], middle), set_axis(pair[1], middle)],
    ] {
        if !result.contains(&candidate) {
            result.push(candidate);
        }
    }
    result
}

#[cfg(test)]
#[path = "../../../../tests/unit/layout/containers/routing/numbered.rs"]
mod tests;
