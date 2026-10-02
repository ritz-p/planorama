use super::*;

fn node_count(coverage: &Coverage) -> usize {
    1 + coverage.left.as_deref().map_or(0, node_count)
        + coverage.right.as_deref().map_or(0, node_count)
}

#[test]
fn indexed_overlap_matches_a_brute_force_coverage_map() {
    let mut coverage = Coverage::default();
    let mut expected = [0u128; 128];
    for i in 0..512 {
        let a = i * 37 % 129;
        let b = i * 61 % 129;
        coverage.insert(a, b);
        for value in &mut expected[a.min(b)..a.max(b)] {
            *value += 1;
        }
        let from = i * 43 % 129;
        let to = i * 71 % 129;
        assert_eq!(
            coverage.overlap(from, to),
            expected[from.min(to)..from.max(to)].iter().sum()
        );
    }
    for from in 0..=128 {
        for to in from..=128 {
            assert_eq!(coverage.overlap(from, to), expected[from..to].iter().sum());
        }
    }
}

#[test]
fn repeated_intervals_are_aggregated_without_growing_the_index() {
    let mut coverage = Coverage::default();
    coverage.insert(20, 100);
    let nodes = node_count(&coverage);
    for _ in 1..20_000 {
        coverage.insert(20, 100);
    }
    assert_eq!(node_count(&coverage), nodes);
    assert_eq!(coverage.overlap(30, 50), 400_000);
    assert_eq!(coverage.overlap(0, 200), 1_600_000);
}

#[test]
fn many_distinct_intervals_can_be_queried_as_an_aggregate() {
    let mut coverage = Coverage::default();
    for i in 0..10_000 {
        coverage.insert(i, i + 100);
    }
    assert_eq!(coverage.overlap(0, 10_100), 1_000_000);
    assert_eq!(coverage.overlap(2_000, 6_000), 400_000);
    assert_eq!(coverage.overlap(10_100, 20_000), 0);
    let nodes = node_count(&coverage);
    for _ in 0..1_000 {
        assert_eq!(coverage.overlap(2_000, 6_000), 400_000);
    }
    assert_eq!(node_count(&coverage), nodes);
}

#[test]
fn full_coordinate_domain_and_zero_length_ranges_are_supported() {
    let mut coverage = Coverage::default();
    coverage.insert(0, usize::MAX);
    coverage.insert(usize::MAX, usize::MAX - 10);
    assert_eq!(coverage.overlap(0, usize::MAX), usize::MAX as u128 + 10);
    assert_eq!(coverage.overlap(usize::MAX - 5, usize::MAX), 10);
    let nodes = node_count(&coverage);
    coverage.insert(20, 20);
    assert_eq!(coverage.overlap(20, 20), 0);
    assert_eq!(node_count(&coverage), nodes);
}
