use crate::layout::{Bounds, Side};
use crate::model::architecture::Graph;

#[test]
fn numbered_parallel_and_mixed_slots_are_spaced_and_stable() {
    use crate::layout::Point;
    for fixture in [
        include_str!("../../../tests/fixtures/routes-plan.json"),
        include_str!("../../../tests/fixtures/association-plan.json"),
    ] {
        let mut graph = crate::semantic::transform(&crate::plan::parse(fixture).unwrap()).0;
        let edge = graph.edges[0].clone();
        if graph.edges.len() == 1 {
            let mut parallel = edge.clone();
            parallel
                .change
                .as_mut()
                .unwrap()
                .address
                .push_str("_parallel");
            graph.edges.push(parallel);
        }
        graph
            .edges
            .push(crate::model::Edge::from((edge.from, edge.to)));
        let bounds = vec![
            Bounds {
                origin: Point { x: 100, y: 100 },
                width: 400,
                height: 300
            };
            graph.nodes.len()
        ];
        let snapshot = |graph: &Graph| {
            let mut incident = vec![Vec::new(); graph.nodes.len()];
            for (i, edge) in graph.edges.iter().enumerate() {
                incident[edge.from].push((i, true));
                incident[edge.to].push((i, false));
            }
            let (sources, targets) = assign(graph, &bounds, &incident);
            let numbered = crate::layout::relationship_markers::ends(graph);
            for edges in incident {
                for side in 0..4 {
                    let mut offsets: Vec<_> = edges
                        .iter()
                        .map(|&(e, source)| {
                            (
                                if source {
                                    sources[e][side]
                                } else {
                                    targets[e][side]
                                },
                                numbered[e][usize::from(!source)],
                            )
                        })
                        .collect();
                    offsets.sort();
                    for pair in offsets.windows(2) {
                        assert!(pair[0].0 < pair[1].0);
                        if pair[0].1 && pair[1].1 {
                            assert!(pair[1].0 - pair[0].0 >= 24);
                        }
                    }
                }
            }
            (sources, targets)
        };
        let original = snapshot(&graph);
        graph.edges.reverse();
        let reversed = snapshot(&graph);
        assert_eq!(original.0, reversed.0.into_iter().rev().collect::<Vec<_>>());
        assert_eq!(original.1, reversed.1.into_iter().rev().collect::<Vec<_>>());
    }
}

pub(in crate::layout) fn assign(
    graph: &Graph,
    bounds: &[Bounds],
    incident: &[Vec<(usize, bool)>],
) -> (Vec<[usize; 4]>, Vec<[usize; 4]>) {
    let mut sources = vec![[0; 4]; graph.edges.len()];
    let mut targets = sources.clone();
    let numbered = crate::layout::relationship_markers::ends(graph);
    for (node, edges) in incident.iter().enumerate() {
        for side in [Side::Left, Side::Right, Side::Top, Side::Bottom] {
            let mut ordered = edges.clone();
            ordered.sort_by_key(|&(index, source)| {
                let edge = &graph.edges[index];
                let peer = if source { edge.to } else { edge.from };
                let b = bounds[peer];
                let coordinate = match side {
                    Side::Left | Side::Right => b.origin.y * 2 + b.height,
                    Side::Top | Side::Bottom => b.origin.x * 2 + b.width,
                };
                (
                    coordinate,
                    edge.change
                        .as_ref()
                        .map(|c| (b.origin.x, b.origin.y, c.local_address())),
                    graph.nodes[peer].entity.id.as_str(),
                    source,
                    edge.kind,
                    edge.change.as_ref().map(|c| (c.address.as_str(), c.action)),
                )
            });
            let size = match side {
                Side::Left | Side::Right => bounds[node].height,
                Side::Top | Side::Bottom => bounds[node].width,
            };
            let offsets = if ordered
                .iter()
                .any(|&(e, source)| numbered[e][usize::from(!source)])
            {
                let spacing = match side {
                    Side::Left | Side::Right => crate::layout::relationship_markers::PORT_SPACING,
                    Side::Top | Side::Bottom => {
                        crate::layout::relationship_markers::clearance(graph)
                            - crate::layout::relationship_markers::GAP
                            - 8
                            + 6
                    }
                };
                crate::layout::relationship_markers::ports_with_spacing(
                    &ordered, &numbered, size, spacing,
                )
            } else {
                (0..ordered.len())
                    .map(|slot| {
                        20.min(size / 2)
                            + (slot + 1) * size.saturating_sub(40) / (ordered.len() + 1)
                    })
                    .collect()
            };
            for (&(edge, source), offset) in ordered.iter().zip(offsets) {
                if source {
                    sources[edge][side as usize] = offset;
                } else {
                    targets[edge][side as usize] = offset;
                }
            }
        }
    }
    (sources, targets)
}

#[test]
fn incident_slots_follow_peer_geometry_in_both_directions_on_every_side() {
    use crate::layout::Point;
    use crate::model::architecture::Edge;
    let raw = crate::plan::parse(r#"{"format_version":"1.2","resource_changes":[{"address":"test.hub","type":"test"},{"address":"test.z","type":"test"},{"address":"test.a","type":"test"}]}"#).unwrap();
    let mut graph = crate::semantic::transform(&raw).0;
    graph.edges = vec![
        Edge::from((0, 1)),
        Edge::from((0, 2)),
        Edge::from((1, 0)),
        Edge::from((2, 0)),
    ];
    let mut bounds = vec![
        Bounds {
            origin: Point { x: 300, y: 300 },
            width: 200,
            height: 200,
        },
        Bounds::card(Point { x: 100, y: 100 }),
        Bounds::card(Point { x: 600, y: 600 }),
    ];
    let incident = |graph: &Graph| {
        let mut result = vec![Vec::new(); graph.nodes.len()];
        for (i, e) in graph.edges.iter().enumerate() {
            result[e.from].push((i, true));
            result[e.to].push((i, false));
        }
        result
    };
    let (sources, targets) = assign(&graph, &bounds, &incident(&graph));
    for side in 0..4 {
        assert!(sources[0][side] < sources[1][side]);
        assert!(targets[2][side] < targets[3][side]);
    }
    graph.edges.reverse();
    let (s, t) = assign(&graph, &bounds, &incident(&graph));
    assert_eq!(sources, s.into_iter().rev().collect::<Vec<_>>());
    assert_eq!(targets, t.into_iter().rev().collect::<Vec<_>>());
    bounds[2] = bounds[1];
    let tied = assign(&graph, &bounds, &incident(&graph));
    graph.edges.reverse();
    let again = assign(&graph, &bounds, &incident(&graph));
    assert_eq!(tied.0, again.0.into_iter().rev().collect::<Vec<_>>());
    assert_eq!(tied.1, again.1.into_iter().rev().collect::<Vec<_>>());
}
