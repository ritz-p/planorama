use super::*;

fn points(values: &[(usize, usize)]) -> Vec<Point> {
    values.iter().map(|&(x, y)| Point { x, y }).collect()
}

#[test]
fn longer_clear_route_beats_shorter_overlapping_route() {
    let mut scorer = Scorer::default();
    scorer.insert(points(&[(20, 50), (80, 50)]));
    let short = points(&[(0, 50), (100, 50)]);
    let clear = points(&[(0, 50), (0, 100), (100, 100), (100, 50)]);
    assert!(scorer.score(&clear, &[]) < scorer.score(&short, &[]));
}

#[test]
fn crossings_are_more_expensive_than_bends_and_length() {
    let mut scorer = Scorer::default();
    scorer.insert(points(&[(50, 20), (50, 80)]));
    let crossing = points(&[(0, 50), (100, 50)]);
    let clear = points(&[(0, 50), (0, 100), (100, 100), (100, 50)]);
    assert_eq!(scorer.score(&crossing, &[]).crossings, 1);
    assert!(scorer.score(&clear, &[]) < scorer.score(&crossing, &[]));
}

#[test]
fn card_interiors_take_priority_over_edge_overlap() {
    let mut scorer = Scorer::default();
    let around = points(&[(0, 150), (0, 250), (500, 250), (500, 150)]);
    scorer.insert(around.clone());
    let through = points(&[(0, 150), (500, 150)]);
    let nodes = [Point { x: 100, y: 100 }];
    assert!(scorer.score(&around, &nodes) < scorer.score(&through, &nodes));
}

#[test]
fn overlap_is_direction_independent_and_contacts_do_not_cross() {
    let mut scorer = Scorer::default();
    scorer.insert(points(&[(10, 20), (100, 20)]));
    assert_eq!(
        scorer.score(&points(&[(80, 20), (30, 20)]), &[]).overlap,
        50
    );
    assert_eq!(
        scorer.score(&points(&[(50, 20), (50, 80)]), &[]).crossings,
        0
    );
    assert_eq!(
        scorer.score(&points(&[(50, 20), (50, 80)]), &[]),
        scorer.score(&points(&[(50, 20), (50, 80)]), &[])
    );
}

#[test]
fn equal_overlap_and_crossings_prefer_fewer_bends_then_shorter_paths() {
    let scorer = Scorer::default();
    let direct = points(&[(0, 10), (100, 10)]);
    let detour = points(&[(0, 10), (0, 30), (100, 30), (100, 10)]);
    assert!(scorer.score(&direct, &[]) < scorer.score(&detour, &[]));
    assert!(scorer.score(&direct, &[]) < scorer.score(&points(&[(0, 10), (200, 10)]), &[]));
}
