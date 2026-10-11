use crate::layout::{Bounds, Side};
use crate::model::architecture::Graph;

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
                    if sides.is_some() { MIN_SPACING } else { 1 },
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

pub(in crate::layout) fn crowded(
    graph: &Graph,
    paths: &[Vec<crate::layout::Point>],
    sides: &[[Side; 2]],
) -> bool {
    let numbered = crate::layout::relationship_markers::ends(graph);
    if opposite_roles_collide(graph, paths) {
        return true;
    }
    let mut ports = vec![[Vec::new(), Vec::new(), Vec::new(), Vec::new()]; graph.nodes.len()];
    for (index, edge) in graph.edges.iter().enumerate() {
        let path = &paths[index];
        if path.len() < 2 {
            continue;
        }
        for (end, node, point) in [(0, edge.from, path[0]), (1, edge.to, *path.last().unwrap())] {
            let side = sides[index][end];
            {
                ports[node][side as usize].push((
                    if matches!(side, Side::Left | Side::Right) {
                        point.y
                    } else {
                        point.x
                    },
                    if numbered[index][end] {
                        if matches!(side, Side::Left | Side::Right) {
                            crate::layout::relationship_markers::PORT_SPACING
                        } else {
                            crate::layout::relationship_markers::clearance(graph)
                                .saturating_sub(crate::layout::relationship_markers::GAP + 8)
                                + 6
                        }
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

pub(in crate::layout) fn opposite_roles_collide(
    graph: &Graph,
    paths: &[Vec<crate::layout::Point>],
) -> bool {
    let mut occupied = std::collections::BTreeMap::new();
    for (index, (edge, path)) in graph.edges.iter().zip(paths).enumerate() {
        if path.len() < 2 {
            continue;
        }
        for (end, node, point) in [(0, edge.from, path[0]), (1, edge.to, *path.last().unwrap())] {
            let roles = occupied
                .entry((node, point.x, point.y))
                .or_insert([None; 2]);
            if roles[1 - end].is_some_and(|other| other != index) {
                return true;
            }
            roles[end] = Some(index);
        }
    }
    false
}

pub(in crate::layout) fn fit_sides(
    graph: &Graph,
    bounds: &[Bounds],
    sides: &mut [[Side; 2]],
    visible: &[bool],
    paths: Option<&[Vec<crate::layout::Point>]>,
) -> bool {
    let numbered = crate::layout::relationship_markers::ends(graph);
    let mut demand = vec![[0; 2]; graph.nodes.len()];
    let mut occupied = std::collections::BTreeSet::new();
    let width = crate::layout::relationship_markers::clearance(graph)
        .saturating_sub(crate::layout::relationship_markers::GAP + 8)
        + 6;
    let mut changed = false;
    for (index, edge) in graph.edges.iter().enumerate().filter(|(i, _)| visible[*i]) {
        for (end, node) in [edge.from, edge.to].into_iter().enumerate() {
            if matches!(sides[index][end], Side::Top | Side::Bottom) {
                if !numbered[index][end] {
                    if let Some(paths) = paths {
                        let point = if end == 0 {
                            paths[index].first()
                        } else {
                            paths[index].last()
                        };
                        if let Some(point) = point {
                            if !occupied.insert((
                                node,
                                sides[index][end] as usize,
                                end,
                                point.x,
                                point.y,
                            )) {
                                continue;
                            }
                        }
                    }
                }
                demand[node][usize::from(sides[index][end] == Side::Bottom)] +=
                    if numbered[index][end] {
                        width
                    } else {
                        MIN_SPACING
                    };
            }
        }
    }
    for (index, edge) in graph.edges.iter().enumerate().filter(|(i, _)| visible[*i]) {
        for (end, node, peer) in [(0, edge.from, edge.to), (1, edge.to, edge.from)] {
            if matches!(sides[index][end], Side::Top | Side::Bottom)
                && 41 + demand[node][usize::from(sides[index][end] == Side::Bottom)]
                    > bounds[node].width
            {
                sides[index][end] = if bounds[peer].origin.x * 2 + bounds[peer].width
                    < bounds[node].origin.x * 2 + bounds[node].width
                {
                    Side::Left
                } else {
                    Side::Right
                };
                changed = true;
            }
        }
    }
    changed
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

#[cfg(test)]
#[path = "../../../tests/unit/layout/routing_shared/slots.rs"]
mod tests;
