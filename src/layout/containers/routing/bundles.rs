use super::{Bounds, Layout, Point, Port, Scorer, Side, obstacles};
use crate::layout::containers::affinity::Group;
use crate::layout::routing_shared::{Simplification, ports, simplify};
use crate::model::architecture::Graph;

pub(super) struct Bundle {
    pub paths: Vec<(usize, Vec<Point>)>,
    pub junctions: Vec<Point>,
}

pub(super) fn try_bundle(
    graph: &Graph,
    layout: &Layout<'_>,
    group: &Group,
    sources: &[usize],
    targets: &[usize],
    scorer: &Scorer,
    reservations: &[Bounds],
) -> Option<Bundle> {
    let target = layout.bounds[group.target];
    let first = *group.sources.first()?;
    let pairs = ports::facing_pairs(layout.bounds[first], target);
    let mut best = None;
    for pair in pairs {
        if !group
            .sources
            .iter()
            .all(|&source| ports::facing_pairs(layout.bounds[source], target).contains(&pair))
        {
            continue;
        }
        for clearance in [16, 24, 8] {
            if let Some((bundle, cost)) = candidate(
                graph,
                layout,
                group,
                (sources, targets),
                scorer,
                reservations,
                (pair, clearance),
            ) {
                if best.as_ref().is_none_or(|(_, old)| cost < *old) {
                    best = Some((bundle, cost));
                }
            }
        }
    }
    best.map(|(bundle, _)| bundle)
}

fn endpoint(bounds: Bounds, side: Side, offset: usize) -> Port {
    let offset = match side {
        Side::Left | Side::Right => offset,
        Side::Top | Side::Bottom => offset * bounds.width / bounds.height.max(1),
    };
    bounds.port(side, offset)
}

fn candidate(
    graph: &Graph,
    layout: &Layout<'_>,
    group: &Group,
    offsets: (&[usize], &[usize]),
    scorer: &Scorer,
    reservations: &[Bounds],
    choice: ((Side, Side), usize),
) -> Option<(Bundle, u128)> {
    let ((source_side, target_side), clearance) = choice;
    let target = layout.bounds[group.target];
    let end = endpoint(
        target,
        target_side,
        group.edges.iter().map(|&i| offsets.1[i]).min()?,
    );
    let trunk = end.outward(clearance);
    if trunk == end.point {
        return None;
    }
    let horizontal = matches!(source_side, Side::Left | Side::Right);
    let along = |p: Point| if horizontal { p.y } else { p.x };
    let mut paths = Vec::new();
    let mut coordinates = vec![along(end.point)];
    let mut bundled_cost = 0;
    let mut independent_cost = 0;
    let mut independent_overlaps = false;
    let mut independent_group = Scorer::default();
    for &index in &group.edges {
        let edge = &graph.edges[index];
        let start = endpoint(layout.bounds[edge.from], source_side, offsets.0[index]);
        let branch = if horizontal {
            Point {
                x: trunk.x,
                y: start.point.y,
            }
        } else {
            Point {
                x: start.point.x,
                y: trunk.y,
            }
        };
        let path = simplify(
            [start.point, branch, trunk, end.point],
            Simplification::PreserveReversals,
        );
        let mut barriers = obstacles(layout, edge);
        barriers.extend_from_slice(reservations);
        if !ports::valid(&path, start, end, &barriers) || scorer.overlaps(&path) {
            return None;
        }
        let independent_end = endpoint(target, target_side, offsets.1[index]);
        let independent = ports::connect(start, independent_end, &barriers, Some(scorer))?;
        independent_overlaps |= scorer.overlaps(&independent);
        bundled_cost += scorer.readability_cost(&path);
        independent_cost += scorer.readability_cost(&independent);
        independent_cost += independent_group.readability_cost(&independent)
            - Scorer::default().readability_cost(&independent);
        independent_group.insert(independent);
        coordinates.push(along(start.point));
        paths.push((index, path));
    }
    if !independent_overlaps && bundled_cost > independent_cost {
        return None;
    }
    coordinates.sort_unstable();
    coordinates.dedup();
    let junctions = coordinates
        .iter()
        .skip(1)
        .take(coordinates.len().saturating_sub(2))
        .map(|&coordinate| {
            if horizontal {
                Point {
                    x: trunk.x,
                    y: coordinate,
                }
            } else {
                Point {
                    x: coordinate,
                    y: trunk.y,
                }
            }
        })
        .collect();
    Some((Bundle { paths, junctions }, bundled_cost))
}

#[test]
fn bundles_use_all_facing_directions_and_fall_back_when_blocked() {
    use crate::model::architecture::{Edge, EdgeKind};
    let raw = crate::plan::parse(r#"{"format_version":"1.2","resource_changes":[{"address":"test.a","type":"test"},{"address":"test.b","type":"test"},{"address":"test.target","type":"test"}]}"#).unwrap();
    let mut graph = crate::semantic::transform(&raw).0;
    graph.edges = [(0, 2), (1, 2)]
        .map(|pair| Edge {
            kind: EdgeKind::Connection,
            ..Edge::from(pair)
        })
        .into();
    for (mirror, vertical) in [(false, false), (true, false), (false, true), (true, true)] {
        let mut layout = Layout::new(&graph);
        layout.bounds = [(100, 100), (100, 300), (500, 200)]
            .map(|(mut x, mut y)| {
                if mirror {
                    x = 800 - x - 100;
                }
                if vertical {
                    std::mem::swap(&mut x, &mut y);
                }
                Bounds {
                    origin: Point { x, y },
                    width: 100,
                    height: 100,
                }
            })
            .into();
        layout.header_heights = vec![100; 3];
        let group = Group {
            sources: vec![0, 1],
            target: 2,
            edges: vec![0, 1],
        };
        let scorer = Scorer::default();
        let bundle =
            try_bundle(&graph, &layout, &group, &[50, 50], &[50, 50], &scorer, &[]).unwrap();
        assert!(!bundle.junctions.is_empty());
        let pair = ports::facing_pairs(layout.bounds[0], layout.bounds[2])[0];
        for (index, path) in &bundle.paths {
            assert_eq!(
                path.first(),
                Some(
                    &layout.bounds[graph.edges[*index].from]
                        .port(pair.0, 50)
                        .point
                )
            );
            assert_eq!(path.last(), Some(&layout.bounds[2].port(pair.1, 50).point));
        }
        let mut reversed = graph.clone();
        reversed.edges.reverse();
        let again = try_bundle(
            &reversed,
            &layout,
            &group,
            &[50, 50],
            &[50, 50],
            &scorer,
            &[],
        )
        .unwrap();
        assert_eq!(bundle.junctions, again.junctions);
        assert_eq!(bundle.paths[0].1, again.paths[1].1);
        let end = layout.bounds[2].port(pair.1, 50).outward(8);
        let blocker = Bounds {
            origin: Point {
                x: end.x - 4,
                y: end.y - 4,
            },
            width: 8,
            height: 8,
        };
        assert!(
            try_bundle(
                &graph,
                &layout,
                &group,
                &[50, 50],
                &[50, 50],
                &scorer,
                &[blocker]
            )
            .is_none()
        );
    }
}
