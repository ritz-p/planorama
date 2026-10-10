use super::{Layout, Point, Scorer, crosses, obstacles, path_between};
use crate::layout::containers::affinity::Group;
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
) -> Option<Bundle> {
    [16, 24, 8].into_iter().find_map(|clearance| {
        let trunk = layout.bounds[group.target]
            .origin
            .x
            .checked_sub(clearance)?;
        candidate(graph, layout, group, sources, targets, scorer, trunk)
    })
}

fn candidate(
    graph: &Graph,
    layout: &Layout<'_>,
    group: &Group,
    sources: &[usize],
    targets: &[usize],
    scorer: &Scorer,
    trunk: usize,
) -> Option<Bundle> {
    let target = layout.bounds[group.target];
    if group
        .sources
        .iter()
        .any(|&source| layout.bounds[source].right() + 8 >= trunk)
    {
        return None;
    }
    let end = Point {
        x: target.origin.x,
        y: target.origin.y + group.edges.iter().map(|&i| targets[i]).min()?,
    };
    let mut paths = Vec::new();
    let mut ys = vec![end.y];
    let mut bundled_cost = 0;
    let mut independent_cost = 0;
    let mut independent_overlaps = false;
    for &index in &group.edges {
        let edge = &graph.edges[index];
        let source = layout.bounds[edge.from];
        let start = Point {
            x: source.right(),
            y: source.origin.y + sources[index],
        };
        let mut path = vec![
            start,
            Point {
                x: trunk,
                y: start.y,
            },
            Point { x: trunk, y: end.y },
            end,
        ];
        path.dedup();
        if scorer.overlaps(&path) {
            return None;
        }
        let obstacles = obstacles(layout, edge);
        if path
            .windows(2)
            .any(|pair| obstacles.iter().any(|&b| crosses(pair[0], pair[1], b)))
        {
            return None;
        }
        let independent_end = Point {
            x: target.right(),
            y: target.origin.y + targets[index],
        };
        let independent = path_between(start, independent_end, &obstacles, Some(scorer));
        independent_overlaps |= scorer.overlaps(&independent);
        bundled_cost += scorer.readability_cost(&path);
        independent_cost += scorer.readability_cost(&independent);
        ys.push(start.y);
        paths.push((index, path));
    }
    if !independent_overlaps && bundled_cost >= independent_cost {
        return None;
    }
    ys.sort_unstable();
    ys.dedup();
    let junctions = ys
        .iter()
        .skip(1)
        .take(ys.len().saturating_sub(2))
        .map(|&y| Point { x: trunk, y })
        .collect();
    Some(Bundle { paths, junctions })
}
