#[cfg(test)]
#[path = "../../../tests/unit/layout/routing/coverage.rs"]
mod tests;

#[derive(Clone, Default)]
pub(super) struct Coverage {
    total: u128,
    covering: u128,
    left: Option<Box<Self>>,
    right: Option<Box<Self>>,
}

impl Coverage {
    pub(super) fn insert(&mut self, from: usize, to: usize) {
        self.insert_in(from.min(to), from.max(to), 0, usize::MAX);
    }

    pub(super) fn overlap(&self, from: usize, to: usize) -> u128 {
        self.overlap_in(from.min(to), from.max(to), 0, usize::MAX)
    }

    fn insert_in(&mut self, from: usize, to: usize, start: usize, end: usize) {
        let from = from.max(start);
        let to = to.min(end);
        if from >= to {
            return;
        }
        self.total += (to - from) as u128;
        if from == start && to == end {
            self.covering += 1;
            return;
        }
        let mid = start + (end - start) / 2;
        if from < mid {
            self.left
                .get_or_insert_with(Default::default)
                .insert_in(from, to, start, mid);
        }
        if to > mid {
            self.right
                .get_or_insert_with(Default::default)
                .insert_in(from, to, mid, end);
        }
    }

    fn overlap_in(&self, from: usize, to: usize, start: usize, end: usize) -> u128 {
        let from = from.max(start);
        let to = to.min(end);
        match (from, to) {
            (from, to) if from >= to => return 0,
            (from, to) if from == start && to == end => return self.total,
            _ => {}
        }
        let mid = start + (end - start) / 2;
        let left = match &self.left {
            Some(child) if from < mid => child.overlap_in(from, to, start, mid),
            _ => 0,
        };
        let right = match &self.right {
            Some(child) if to > mid => child.overlap_in(from, to, mid, end),
            _ => 0,
        };
        self.covering * (to - from) as u128 + left + right
    }
}
