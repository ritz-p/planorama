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

#[cfg(test)]
#[path = "../../../../tests/unit/layout/routing_shared/search/costs.rs"]
mod tests;
