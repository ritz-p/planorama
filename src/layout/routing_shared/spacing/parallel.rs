use super::*;

pub(super) struct Index<'a> {
    paths: &'a [Vec<Point>],
    edge: usize,
    segments: BTreeMap<(bool, usize), Vec<Segment>>,
}

impl<'a> Index<'a> {
    pub(super) fn new(paths: &'a [Vec<Point>], edge: usize) -> Self {
        let mut segments = segments(paths);
        segments.retain(|_, group| {
            group.retain(|segment| segment.edge != edge);
            !group.is_empty()
        });
        Self {
            paths,
            edge,
            segments,
        }
    }

    pub(super) fn deficit(&self, graph: &Graph, path: &[Point]) -> usize {
        path_segments(self.edge, path)
            .map(|a| {
                self.segments
                    .range(
                        (a.horizontal, a.coordinate.saturating_sub(PREFERRED - 1))
                            ..=(a.horizontal, a.coordinate.saturating_add(PREFERRED - 1)),
                    )
                    .flat_map(|(_, group)| group)
                    .map(|&b| {
                        pair_length(graph, [path, &self.paths[b.edge]], a, b)
                            * (PREFERRED - a.coordinate.abs_diff(b.coordinate))
                    })
                    .sum::<usize>()
            })
            .sum()
    }

    pub(super) fn axes(
        &self,
        corridor: &plan::Corridor,
        endpoints: [Point; 2],
        preferred: [usize; 2],
        frames: impl IntoIterator<Item = usize>,
        extent: usize,
    ) -> Vec<usize> {
        let axis = |point: Point| {
            if corridor.horizontal {
                point.y
            } else {
                point.x
            }
        };
        let [start, end] = endpoints;
        let low = axis(start).min(axis(end)).saturating_sub(128);
        let high = axis(start).max(axis(end)).saturating_add(128);
        let mut coordinates: Vec<_> = self
            .segments
            .range((corridor.horizontal, low)..=(corridor.horizontal, high))
            .filter(|(_, group)| {
                group.iter().any(|s| {
                    s.high
                        .min(corridor.high)
                        .saturating_sub(s.low.max(corridor.low))
                        >= RUN
                })
            })
            .flat_map(|(&(_, coordinate), _)| {
                [
                    coordinate.checked_sub(PREFERRED),
                    coordinate.checked_add(PREFERRED),
                ]
                .into_iter()
                .flatten()
            })
            .filter(|axis| low <= *axis && *axis <= high)
            .chain(frames)
            .collect();
        coordinates.sort_unstable();
        coordinates.dedup();
        coordinates.sort_by_cached_key(|&coordinate| {
            (
                self.deficit_at(corridor, coordinate),
                preferred[0]
                    .abs_diff(coordinate)
                    .min(preferred[1].abs_diff(coordinate)),
                coordinate,
            )
        });
        let mut seen = std::collections::BTreeSet::new();
        preferred
            .into_iter()
            .chain(coordinates)
            .filter(|&coordinate| coordinate < extent && seen.insert(coordinate))
            .take(32)
            .collect()
    }

    fn deficit_at(&self, corridor: &plan::Corridor, coordinate: usize) -> usize {
        self.segments
            .range(
                (
                    corridor.horizontal,
                    coordinate.saturating_sub(PREFERRED - 1),
                )
                    ..=(
                        corridor.horizontal,
                        coordinate.saturating_add(PREFERRED - 1),
                    ),
            )
            .flat_map(|(_, group)| group)
            .map(|segment| {
                let length = segment
                    .high
                    .min(corridor.high)
                    .saturating_sub(segment.low.max(corridor.low));
                if length >= RUN {
                    length * (PREFERRED - segment.coordinate.abs_diff(coordinate))
                } else {
                    0
                }
            })
            .sum()
    }
}
