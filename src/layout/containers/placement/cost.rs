use crate::layout::Point;
use crate::layout::routing_shared::scoring::CROSSING_COST;
use std::collections::BTreeMap;

pub(super) fn estimate(
    relationships: &[(usize, usize)],
    sizes: &[(usize, usize)],
    offsets: &[Point],
) -> u128 {
    let center = |node: usize| {
        (
            offsets[node].x as i128 * 2 + sizes[node].0 as i128,
            offsets[node].y as i128 * 2 + sizes[node].1 as i128,
        )
    };
    let mut distance = 0;
    let mut segments = BTreeMap::new();
    for &(from, to) in relationships {
        let (a, b) = (center(from), center(to));
        distance += a.0.abs_diff(b.0) + a.1.abs_diff(b.1);
        *segments.entry((a.min(b), a.max(b))).or_insert(0u128) += 1;
    }
    let step = segments.len().div_ceil(256).max(1);
    let sampled: Vec<_> = segments.into_iter().step_by(step).collect();
    let orientation = |a: (i128, i128), b: (i128, i128), c: (i128, i128)| {
        ((b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0)).signum()
    };
    let mut crossings = 0;
    for (i, &((a, b), weight)) in sampled.iter().enumerate() {
        for &((c, d), other_weight) in &sampled[i + 1..] {
            if orientation(a, b, c) * orientation(a, b, d) < 0
                && orientation(c, d, a) * orientation(c, d, b) < 0
            {
                crossings += weight * other_weight;
            }
        }
    }
    distance / 2 + crossings * CROSSING_COST
}

#[cfg(test)]
#[path = "../../../../tests/unit/layout/containers/packing_cost.rs"]
mod tests;
