use super::{Point, Scorer, occlusion};

pub(super) struct Grid<'a> {
    scorer: &'a Scorer,
    columns: usize,
    segments: Vec<[u128; 2]>,
    junctions: Vec<[u128; 2]>,
    occlusion: occlusion::Grid,
    #[cfg(test)]
    evaluations: [usize; 2],
}

impl<'a> Grid<'a> {
    pub fn new(xs: &[usize], ys: &[usize], scorer: &'a Scorer) -> Self {
        Self {
            scorer,
            columns: xs.len(),
            segments: vec![[u128::MAX; 2]; xs.len() * ys.len()],
            junctions: vec![[u128::MAX; 2]; xs.len() * ys.len()],
            occlusion: occlusion::Grid::new(xs, ys, scorer.soft()),
            #[cfg(test)]
            evaluations: [0; 2],
        }
    }

    pub fn segment(&mut self, from: usize, to: usize, a: Point, b: Point) -> u128 {
        let vertical = usize::from(from / self.columns != to / self.columns);
        let cost = &mut self.segments[from.min(to)][vertical];
        if *cost == u128::MAX {
            *cost = self.scorer.conflict_cost(a, b) + self.occlusion.cost(from, to);
            #[cfg(test)]
            {
                self.evaluations[0] += 1;
            }
        }
        *cost
    }

    pub fn junction(&mut self, node: usize, point: Point, horizontal: bool) -> u128 {
        let cost = &mut self.junctions[node][usize::from(!horizontal)];
        if *cost == u128::MAX {
            *cost = self.scorer.junction_cost(point, horizontal);
            #[cfg(test)]
            {
                self.evaluations[1] += 1;
            }
        }
        *cost
    }
}

#[test]
fn repeated_grid_states_share_exact_segment_and_junction_costs() {
    use crate::layout::Bounds;
    let xs = [10, 20, 30, 40];
    let ys = [10, 20, 30, 40];
    let mut scorer = Scorer::default();
    for _ in 0..2 {
        scorer.insert(vec![Point { x: 10, y: 20 }, Point { x: 40, y: 20 }]);
    }
    scorer.insert(vec![Point { x: 25, y: 10 }, Point { x: 25, y: 40 }]);
    scorer.set_soft(vec![Bounds {
        origin: Point { x: 15, y: 15 },
        width: 20,
        height: 20,
    }]);
    let mut grid = Grid::new(&xs, &ys, &scorer);
    let point = |i: usize| Point {
        x: xs[i % xs.len()],
        y: ys[i / xs.len()],
    };
    for _ in 0..3 {
        for node in 0..xs.len() * ys.len() {
            for to in [
                (node % xs.len() + 1 < xs.len()).then_some(node + 1),
                (node / xs.len() + 1 < ys.len()).then_some(node + xs.len()),
            ]
            .into_iter()
            .flatten()
            {
                let expected = scorer.segment_cost(point(node), point(to));
                assert_eq!(grid.segment(node, to, point(node), point(to)), expected);
                assert_eq!(grid.segment(to, node, point(to), point(node)), expected);
            }
            for horizontal in [false, true] {
                assert_eq!(
                    grid.junction(node, point(node), horizontal),
                    scorer.junction_cost(point(node), horizontal)
                );
            }
        }
    }
    assert_eq!(grid.evaluations, [24, 32]);
}
