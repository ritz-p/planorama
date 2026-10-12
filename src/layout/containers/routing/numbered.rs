use super::{
    Bounds, Layout, Port, Scorer, Side, obstacles, represented_by_nesting, soft_obstacles,
};
use crate::layout::relationship_markers;
use crate::layout::routing_shared::ports;
use crate::model::architecture::Graph;

pub(super) struct Terminals {
    pub ports: Vec<[Port; 2]>,
    pub clearances: Vec<[usize; 2]>,
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
        let ends = relationship_markers::ends(graph);
        let clearance = relationship_markers::clearance(graph);
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
        let clearances: Vec<_> = ends
            .iter()
            .enumerate()
            .map(|(i, ends)| {
                let mut result = ends.map(|numbered| if numbered { clearance } else { 16 });
                if crate::view::routes::owner(graph, &graph.nodes[graph.edges[i].from]).is_some() {
                    result[0] = 20;
                }
                result
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
        if !quality || sides.is_some() {
            return result;
        }
        let mut scorer = Scorer::default();
        scorer.set_borders(&layout.bounds);
        for &index in order {
            let edge = &graph.edges[index];
            if !ends[index][1] || represented_by_nesting(edge, &layout.containment) {
                continue;
            }
            let barriers = result.barriers(layout, graph, index);
            scorer.set_soft(soft_obstacles(layout, edge));
            scorer.set_peers(
                layout
                    .containment
                    .routing_peers(edge.from, edge.to, &layout.bounds),
            );
            let old = result.ports[index];
            let mut candidates = Vec::new();
            let mut pairs = Vec::new();
            let mut limit = u128::MAX;
            'batches: for batch in
                ports::pair_batches(layout.bounds[edge.from], layout.bounds[edge.to])
            {
                let mut options: Vec<_> = batch
                    .into_iter()
                    .map(|(rank, (a, b))| {
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
                        (
                            ports::cost_bound(pair[0], pair[1], result.clearances[index]),
                            rank,
                            pair,
                        )
                    })
                    .collect();
                options.sort_by_key(|&(bound, rank, _)| (bound, rank));
                for (bound, rank, pair) in options {
                    if bound > limit {
                        break;
                    }
                    if pair
                        .into_iter()
                        .zip([edge.from, edge.to])
                        .any(|(port, node)| {
                            let bounds = layout.bounds[node];
                            match port.side {
                                Side::Left | Side::Right => {
                                    port.point.y <= bounds.origin.y
                                        || port.point.y >= bounds.origin.y + bounds.height
                                }
                                Side::Top | Side::Bottom => {
                                    port.point.x <= bounds.origin.x
                                        || port.point.x >= bounds.right()
                                }
                            }
                        })
                    {
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
                    let corridors =
                        ports::terminal_corridors(pair[0], pair[1], result.clearances[index]);
                    let mut protected = barriers.clone();
                    protected.extend(soft_obstacles(layout, edge).into_iter().map(|o| o.bounds));
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
                        result.clearances[index],
                    ) {
                        optimal = scorer.readability_cost(&path)
                            == ports::distance_bound(
                                layout.bounds[edge.from],
                                layout.bounds[edge.to],
                            );
                        candidates.push((rank, path));
                        pairs.push(pair);
                        if optimal {
                            break;
                        }
                    }
                    if optimal {
                        break 'batches;
                    }
                    if let Some(best) =
                        scorer.best_index(candidates.iter().map(|(_, p)| p.as_slice()), &barriers)
                    {
                        limit = scorer.readability_cost(&candidates[best].1);
                    }
                }
            }
            candidates.sort_by_key(|(rank, _)| *rank);
            if let Some(path) =
                scorer.choose(candidates.into_iter().map(|(_, p)| p).collect(), &barriers)
            {
                if let Some(pair) = pairs.into_iter().find(|p| {
                    Some(&p[0].point) == path.first()
                        && Some(&p[1].point) == path.last()
                        && ports::valid_with_clearances(
                            &path,
                            p[0],
                            p[1],
                            &barriers,
                            result.clearances[index],
                        )
                }) {
                    result.ports[index] = pair;
                    result.reservations[index] =
                        ports::terminal_corridors(pair[0], pair[1], result.clearances[index]);
                }
                scorer.insert(path);
            }
        }
        result
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

#[cfg(test)]
#[path = "../../../../tests/unit/layout/containers/routing/numbered.rs"]
mod tests;
