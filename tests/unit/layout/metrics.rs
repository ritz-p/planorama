use super::*;

fn path(points: &[(usize, usize)]) -> Vec<Point> {
    points.iter().map(|&(x, y)| Point { x, y }).collect()
}

#[test]
fn empty_and_degenerate_paths_have_zero_cost() {
    assert_eq!(measure(&[]), LayoutMetrics::default());
    assert_eq!(
        measure(&[vec![], path(&[(5, 5), (5, 5)])]),
        LayoutMetrics::default()
    );
}

#[test]
fn horizontal_and_vertical_overlaps_are_pairwise_lengths() {
    let paths = vec![
        path(&[(0, 10), (20, 10)]),
        path(&[(15, 10), (5, 10)]),
        path(&[(30, 10), (40, 10)]),
        path(&[(50, 0), (50, 20)]),
        path(&[(50, 15), (50, 5)]),
        path(&[(50, 30), (50, 40)]),
    ];
    assert_eq!(
        measure(&paths),
        LayoutMetrics {
            overlap_distance: 20,
            crossing_count: 0,
            bend_count: 0,
            total_path_length: 80
        }
    );
    let identical = path(&[(0, 0), (10, 0)]);
    assert_eq!(
        measure(&[identical.clone(), identical.clone(), identical]).overlap_distance,
        30
    );
}

#[test]
fn dense_grid_has_sixteen_crossings_and_no_overlap() {
    let paths: Vec<_> = (1..=4)
        .map(|i| path(&[(0, i * 10), (50, i * 10)]))
        .chain((1..=4).map(|i| path(&[(i * 10, 0), (i * 10, 50)])))
        .collect();
    assert_eq!(
        measure(&paths),
        LayoutMetrics {
            overlap_distance: 0,
            crossing_count: 16,
            bend_count: 0,
            total_path_length: 400
        }
    );
}

#[test]
fn endpoint_contacts_are_not_crossings() {
    let paths = vec![
        path(&[(0, 0), (10, 0)]),
        path(&[(10, 0), (10, 10)]),
        path(&[(5, 0), (5, 10)]),
    ];
    assert_eq!(
        measure(&paths),
        LayoutMetrics {
            overlap_distance: 0,
            crossing_count: 0,
            bend_count: 0,
            total_path_length: 30
        }
    );
}

#[test]
fn bends_ignore_duplicate_points_and_straight_subdivisions() {
    let paths = vec![path(&[
        (0, 0),
        (5, 0),
        (10, 0),
        (10, 10),
        (10, 10),
        (20, 10),
    ])];
    assert_eq!(
        measure(&paths),
        LayoutMetrics {
            overlap_distance: 0,
            crossing_count: 0,
            bend_count: 2,
            total_path_length: 30
        }
    );
}

#[test]
fn subdivisions_and_reversal_do_not_change_metrics() {
    let a = path(&[(0, 10), (20, 10)]);
    let b = path(&[(5, 0), (5, 20)]);
    let expected = measure(&[a.clone(), b.clone()]);
    assert_eq!(expected.crossing_count, 1);
    assert_eq!(
        expected,
        measure(&[path(&[(0, 10), (5, 10), (20, 10)]), b.clone()])
    );
    assert_eq!(
        expected,
        measure(&[a.into_iter().rev().collect(), b.into_iter().rev().collect()])
    );
}

#[test]
fn self_intersections_are_excluded_from_edge_pair_metrics() {
    let metrics = measure(&[path(&[(0, 10), (20, 10), (20, 0), (10, 0), (10, 20)])]);
    assert_eq!(metrics.overlap_distance, 0);
    assert_eq!(metrics.crossing_count, 0);
    assert_eq!(metrics.bend_count, 3);
    assert_eq!(metrics.total_path_length, 60);
}

#[test]
#[should_panic(expected = "orthogonal paths")]
fn diagonal_segments_are_rejected() {
    measure(&[path(&[(0, 0), (10, 10)])]);
}
