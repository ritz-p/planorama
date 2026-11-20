pub(crate) struct Window<T> {
    remaining: usize,
    pub(crate) best: Option<(u128, T)>,
}

impl<T> Window<T> {
    pub(crate) fn new(budget: usize) -> Self {
        Self {
            remaining: budget,
            best: None,
        }
    }

    pub(crate) fn consider(&mut self, candidate: Option<(u128, T)>) -> bool {
        if self.remaining == 0 {
            return true;
        }
        if let Some((cost, route)) = candidate {
            if self.best.as_ref().is_none_or(|(best, _)| cost < *best) {
                self.best = Some((cost, route));
            }
        }
        if self.best.is_some() {
            self.remaining -= 1;
        }
        self.remaining == 0
    }
}

#[test]
fn shared_selection_cases() {
    for line in include_str!("cases.txt").lines() {
        let fields: Vec<_> = line.split_whitespace().collect();
        let [name, original, budget, expected, entries @ ..] = fields.as_slice() else {
            panic!("invalid case {line}");
        };
        let original = original.parse::<u128>().unwrap();
        let mut window = Window::new(budget.parse().unwrap());
        for (index, entry) in entries.iter().enumerate() {
            let (cost, eligible) = entry.split_once('/').unwrap();
            let cost = cost.parse().unwrap();
            if window.consider((eligible == "yes" && cost < original).then_some((cost, index + 1)))
            {
                break;
            }
        }
        assert_eq!(
            window.best.map_or(0, |(_, index)| index),
            expected.parse::<usize>().unwrap(),
            "{name}"
        );
    }
}

#[test]
fn selection_matches_the_first_minimum_in_the_examined_window() {
    use proptest::prelude::*;
    proptest!(|(
        costs in proptest::collection::vec((any::<bool>(), 0u8..12), 0..40),
        original in 0u8..12,
        budget in 0usize..12,
    )| {
        let improving = |&(eligible, cost): &(bool, u8)| eligible && cost < original;
        let expected = costs.iter().position(improving).and_then(|start| {
            costs.iter().enumerate().skip(start).take(budget)
                .filter(|(_, pair)| improving(pair))
                .min_by_key(|(index, (_, cost))| (*cost, *index))
                .map(|(index, _)| index)
        });
        let mut window = Window::new(budget);
        for (index, pair) in costs.iter().enumerate() {
            if window.consider(improving(pair).then_some((u128::from(pair.1), index))) {
                break;
            }
        }
        prop_assert_eq!(window.best.map(|(_, index)| index), expected);
    });
}
