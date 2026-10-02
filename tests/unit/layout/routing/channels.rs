use super::*;

#[test]
fn overlapping_ranges_use_another_channel_even_when_it_is_farther() {
    let mut channels = HorizontalChannels::new(&[100, 120, 500]);
    assert_eq!(channels.allocate(20, 80, 100, 100), 100);
    assert_eq!(channels.allocate(20, 80, 100, 100), 120);
    assert_eq!(channels.allocate(60, 90, 100, 100), 500);
}

#[test]
fn disjoint_and_touching_ranges_reuse_the_nearest_channel() {
    let mut channels = HorizontalChannels::new(&[100, 120]);
    assert_eq!(channels.allocate(20, 80, 100, 100), 100);
    assert_eq!(channels.allocate(100, 160, 100, 100), 100);
    assert_eq!(channels.allocate(80, 100, 100, 100), 100);
}

#[test]
fn exhausted_channels_minimize_overlap_before_distance() {
    let mut channels = HorizontalChannels::new(&[100, 200]);
    assert_eq!(channels.allocate(0, 100, 100, 100), 100);
    assert_eq!(channels.allocate(90, 110, 200, 200), 200);
    assert_eq!(channels.allocate(0, 100, 100, 100), 200);
}

#[test]
fn reversed_ranges_and_zero_length_segments_are_handled() {
    let mut channels = HorizontalChannels::new(&[100, 120]);
    assert_eq!(channels.allocate(80, 20, 100, 100), 100);
    assert_eq!(channels.allocate(30, 40, 100, 100), 120);
    assert_eq!(channels.allocate(90, 90, 100, 100), 100);
    assert_eq!(channels.allocate(85, 95, 100, 100), 100);
}

#[test]
fn allocations_are_deterministic() {
    let allocate = || {
        let mut channels = HorizontalChannels::new(&[100, 200, 300]);
        (0..12)
            .map(|i| channels.allocate(i * 10, i * 10 + 60, 150, 250))
            .collect::<Vec<_>>()
    };
    assert_eq!(allocate(), allocate());
}

#[test]
fn indexed_allocation_preserves_the_scan_based_selection() {
    let ys = [10, 50, 100, 200];
    let mut channels = HorizontalChannels::new(&ys);
    let mut history = vec![Vec::<(usize, usize)>::new(); ys.len()];
    for i in 0..1_000usize {
        let a = i * 37 % 513;
        let b = i * 61 % 513;
        let (from, to) = (a.min(b), a.max(b));
        let (source, target) = (i * 13 % 201, i * 17 % 201);
        let index = (0..ys.len())
            .min_by_key(|&index| {
                let overlap: u128 = history[index]
                    .iter()
                    .map(|&(a, b)| b.min(to).saturating_sub(a.max(from)) as u128)
                    .sum();
                (
                    overlap,
                    source.abs_diff(ys[index]) as u128 + target.abs_diff(ys[index]) as u128,
                    index,
                )
            })
            .unwrap();
        assert_eq!(channels.allocate(a, b, source, target), ys[index]);
        if from != to {
            history[index].push((from, to));
        }
    }
}
