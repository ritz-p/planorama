use crate::layout::{Bounds, Side};
use crate::model::architecture::Graph;

pub(in crate::layout) fn assign(
    graph: &Graph,
    bounds: &[Bounds],
    incident: &[Vec<(usize, bool)>],
) -> (Vec<[usize; 4]>, Vec<[usize; 4]>) {
    let mut sources = vec![[0; 4]; graph.edges.len()];
    let mut targets = sources.clone();
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
            for (slot, &(edge, source)) in ordered.iter().enumerate() {
                let offset =
                    20.min(size / 2) + (slot + 1) * size.saturating_sub(40) / (ordered.len() + 1);
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
