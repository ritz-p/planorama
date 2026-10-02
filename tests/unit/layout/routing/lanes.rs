use super::*;

#[test]
fn preview_does_not_reserve_lanes() {
    let mut lanes = VerticalLanes::default();
    lanes.allocate(0, 100, 200);
    let next = lanes.preview(0, 150, 250);
    assert_eq!(next.index, 1);
    assert_eq!(lanes.preview(0, 150, 250), next);
    assert_eq!(lanes.allocate(0, 150, 250), next);
    assert_eq!(lanes.preview(0, 200, 300).index, 0);
}

#[test]
fn overlapping_ranges_allocate_more_than_five_lanes() {
    let mut lanes = VerticalLanes::default();
    for index in 0..12 {
        assert_eq!(lanes.allocate(0, 100, 200).index, index);
    }
    assert_eq!(lanes.width(0), 158);
}

#[test]
fn disjoint_and_touching_ranges_reuse_the_first_lane() {
    let mut lanes = VerticalLanes::default();
    let first = lanes.allocate(0, 100, 200);
    assert_eq!(lanes.allocate(0, 300, 400), first);
    assert_eq!(lanes.allocate(0, 200, 300), first);
    assert_eq!(lanes.allocate(0, 150, 250).index, 1);
}

#[test]
fn gutters_are_independent_and_reversed_ranges_are_normalized() {
    let mut lanes = VerticalLanes::default();
    assert_eq!(lanes.allocate(0, 200, 100).index, 0);
    assert_eq!(lanes.allocate(1, 100, 200).index, 0);
    assert_eq!(lanes.allocate(0, 150, 250).index, 1);
}

#[test]
fn allocations_are_deterministic() {
    let allocate = || {
        let mut lanes = VerticalLanes::default();
        (0..20)
            .map(|i| lanes.allocate(i % 3, i * 5, i * 5 + 100))
            .collect::<Vec<_>>()
    };
    assert_eq!(allocate(), allocate());
}

#[test]
fn zero_length_segments_reuse_a_lane_without_occupying_it() {
    let mut lanes = VerticalLanes::default();
    let first = lanes.allocate(0, 100, 200);
    assert_eq!(lanes.allocate(0, 150, 150), first);
    assert_eq!(lanes.allocate(0, 300, 300), first);
    assert_eq!(lanes.allocate(0, 250, 350), first);
}
