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
    pub fn new(
        graph: &Graph,
        layout: &Layout<'_>,
        order: &[usize],
        offsets: (&[usize], &[usize]),
        slots: (&[[usize; 4]], &[[usize; 4]]),
        quality: bool,
    ) -> Self {
        let ends = relationship_markers::ends(graph);
        let clearance = relationship_markers::clearance(graph);
        let ports: Vec<_> = graph
            .edges
            .iter()
            .enumerate()
            .map(|(i, e)| {
                [
                    layout.bounds[e.from].port(Side::Right, offsets.0[i]),
                    layout.bounds[e.to].port(Side::Right, offsets.1[i]),
                ]
            })
            .collect();
        let clearances: Vec<_> = ends
            .iter()
            .map(|ends| ends.map(|numbered| if numbered { clearance } else { 16 }))
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
        for &index in order {
            let edge = &graph.edges[index];
            if !ends[index][1] || represented_by_nesting(edge, &layout.containment) {
                continue;
            }
            let barriers = result.barriers(layout, graph, index);
            let mut scorer = Scorer::default();
            scorer.set_soft(soft_obstacles(layout, edge));
            let old = result.ports[index];
            let mut cost = None;
            for (a, b) in ports::facing_pairs(layout.bounds[edge.from], layout.bounds[edge.to]) {
                let pair = [
                    layout.bounds[edge.from].port(a, slots.0[index][a as usize]),
                    layout.bounds[edge.to].port(b, slots.1[index][b as usize]),
                ];
                let corridors =
                    ports::terminal_corridors(pair[0], pair[1], result.clearances[index]);
                let mut protected = barriers.clone();
                protected.extend(soft_obstacles(layout, edge));
                if corridors
                    .iter()
                    .any(|&a| protected.iter().any(|&b| overlaps(a, b)))
                {
                    continue;
                }
                if let Some(path) = ports::connect_with_clearances(
                    pair[0],
                    pair[1],
                    &barriers,
                    Some(&scorer),
                    result.clearances[index],
                ) {
                    let cost = cost.get_or_insert_with(|| {
                        ports::connect_with_clearances(
                            old[0],
                            old[1],
                            &barriers,
                            Some(&scorer),
                            result.clearances[index],
                        )
                        .map_or(u128::MAX, |p| scorer.readability_cost(&p))
                    });
                    let candidate = scorer.readability_cost(&path);
                    if candidate <= *cost {
                        *cost = candidate;
                        result.ports[index] = pair;
                        result.reservations[index] = corridors;
                    }
                }
            }
        }
        result
    }

    pub fn barriers(&self, layout: &Layout<'_>, graph: &Graph, index: usize) -> Vec<Bounds> {
        let mut barriers = obstacles(layout, &graph.edges[index]);
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

#[test]
fn numbered_relationships_choose_facing_source_and_target_sides() {
    use crate::layout::Point;
    let graph = crate::semantic::transform(
        &crate::plan::parse(include_str!(
            "../../../../tests/fixtures/association-plan.json"
        ))
        .unwrap(),
    )
    .0;
    for (positions, sides) in [
        ([(100, 100), (600, 100)], [Side::Right, Side::Left]),
        ([(600, 100), (100, 100)], [Side::Left, Side::Right]),
        ([(100, 100), (100, 600)], [Side::Bottom, Side::Top]),
    ] {
        let mut layout = Layout::new(&graph);
        let bounds: Vec<_> = positions
            .map(|(x, y)| Bounds {
                origin: Point { x, y },
                width: 200,
                height: 120,
            })
            .into();
        layout.bounds[graph.edges[0].from] = bounds[0];
        layout.bounds[graph.edges[0].to] = bounds[1];
        let terminals = Terminals::new(
            &graph,
            &layout,
            &[0],
            (&[60], &[60]),
            (&[[60, 60, 100, 100]], &[[60, 60, 100, 100]]),
            true,
        );
        assert_eq!(terminals.ports[0].map(|p| p.side), sides);
        let [start, end] = terminals.ports[0];
        let barriers = terminals.barriers(&layout, &graph, 0);
        let path =
            ports::connect_with_clearances(start, end, &barriers, None, terminals.clearances[0])
                .unwrap();
        assert!(ports::valid_with_clearances(
            &path,
            start,
            end,
            &barriers,
            terminals.clearances[0]
        ));
    }
}
