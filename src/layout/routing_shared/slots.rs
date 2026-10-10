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
    assign_selected(graph, bounds, incident, None)
}

pub(in crate::layout) fn assign_selected(
    graph: &Graph,
    bounds: &[Bounds],
    incident: &[Vec<(usize, bool)>],
    sides: Option<&[[Side; 2]]>,
) -> (Vec<[usize; 4]>, Vec<[usize; 4]>) {
    let mut sources = vec![[0; 4]; graph.edges.len()];
    let mut targets = sources.clone();
    let numbered = crate::layout::relationship_markers::ends(graph);
    for (node, edges) in incident.iter().enumerate() {
        for side in [Side::Left, Side::Right, Side::Top, Side::Bottom] {
            let mut ordered = edges.clone();
            if let Some(sides) = sides {
                ordered.retain(|&(edge, source)| sides[edge][usize::from(!source)] == side);
            }
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
            let offsets = if sides.is_some()
                || ordered
                    .iter()
                    .any(|&(e, source)| numbered[e][usize::from(!source)])
            {
                let spacing = match side {
                    Side::Left | Side::Right => crate::layout::relationship_markers::PORT_SPACING,
                    Side::Top | Side::Bottom => {
                        crate::layout::relationship_markers::clearance(graph)
                            .saturating_sub(crate::layout::relationship_markers::GAP + 8)
                            + 6
                    }
                };
                crate::layout::relationship_markers::ports_with_minimum(
                    &ordered,
                    &numbered,
                    size,
                    spacing,
                    if sides.is_some() && matches!(side, Side::Left | Side::Right) {
                        MIN_SPACING
                    } else {
                        1
                    },
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

pub(in crate::layout) const MIN_SPACING: usize = 16;

#[test]
fn horizontal_demand_uses_the_denser_side_and_marker_widths() {
    let raw = crate::plan::parse(r#"{"format_version":"1.2","resource_changes":[{"address":"test.hub","type":"test"},{"address":"test.peer","type":"test"}]}"#).unwrap();
    let mut graph = crate::semantic::transform(&raw).0;
    graph.edges = vec![crate::model::Edge::from((0, 1)); 8];
    for side in [Side::Left, Side::Right] {
        let sides = vec![[side; 2]; 8];
        assert_eq!(required_heights(&graph, &sides, &[true; 8]), vec![169; 2]);
    }
    let sides: Vec<_> = (0..8)
        .map(|i| [if i < 4 { Side::Left } else { Side::Right }; 2])
        .collect();
    assert_eq!(required_heights(&graph, &sides, &[true; 8]), vec![105; 2]);
    assert_eq!(
        required_heights(&graph, &[[Side::Top; 2]; 8], &[true; 8]),
        vec![41; 2]
    );

    let mut graph = crate::semantic::transform(
        &crate::plan::parse(include_str!(
            "../../../tests/fixtures/association-plan.json"
        ))
        .unwrap(),
    )
    .0;
    let edge = graph.edges[0].clone();
    graph.edges = vec![edge.clone(); 4];
    let heights = required_heights(&graph, &[[Side::Right; 2]; 4], &[true; 4]);
    assert_eq!(heights[edge.from], 137);
    assert_eq!(heights[edge.to], 137);
    graph
        .edges
        .push(crate::model::Edge::from((edge.from, edge.to)));
    let heights = required_heights(&graph, &[[Side::Right; 2]; 5], &[true; 5]);
    assert_eq!(heights[edge.from], 153);
    assert_eq!(heights[edge.to], 153);
}

pub(in crate::layout) fn crowded(
    graph: &Graph,
    paths: &[Vec<crate::layout::Point>],
    sides: &[[Side; 2]],
) -> bool {
    let numbered = crate::layout::relationship_markers::ends(graph);
    let mut ports = vec![[Vec::new(), Vec::new()]; graph.nodes.len()];
    for (index, edge) in graph.edges.iter().enumerate() {
        let path = &paths[index];
        if path.len() < 2 {
            continue;
        }
        for (end, node, point) in [(0, edge.from, path[0]), (1, edge.to, *path.last().unwrap())] {
            let side = sides[index][end];
            if matches!(side, Side::Left | Side::Right) {
                ports[node][side as usize].push((
                    point.y,
                    if numbered[index][end] {
                        crate::layout::relationship_markers::PORT_SPACING
                    } else {
                        MIN_SPACING
                    },
                ));
            }
        }
    }
    ports.iter_mut().flat_map(|s| s.iter_mut()).any(|ports| {
        ports.sort_unstable();
        ports
            .windows(2)
            .any(|p| p[0].0 != p[1].0 && p[1].0 - p[0].0 < (p[0].1 + p[1].1).div_ceil(2))
    })
}

pub(in crate::layout) fn required_heights(
    graph: &Graph,
    sides: &[[Side; 2]],
    visible: &[bool],
) -> Vec<usize> {
    let numbered = crate::layout::relationship_markers::ends(graph);
    let mut demand = vec![[0; 2]; graph.nodes.len()];
    for (index, edge) in graph.edges.iter().enumerate() {
        if !visible[index] {
            continue;
        }
        for (end, node) in [edge.from, edge.to].into_iter().enumerate() {
            let side = sides[index][end];
            if matches!(side, Side::Left | Side::Right) {
                demand[node][side as usize] += if numbered[index][end] {
                    crate::layout::relationship_markers::PORT_SPACING
                } else {
                    MIN_SPACING
                };
            }
        }
    }
    demand.into_iter().map(|s| 41 + s[0].max(s[1])).collect()
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
