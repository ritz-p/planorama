use super::*;

fn points(values: &[(usize, usize)]) -> Vec<Point> {
    values.iter().map(|&(x, y)| Point { x, y }).collect()
}

#[test]
fn rounds_all_horizontal_and_vertical_turn_directions() {
    for (before, after, expected) in [
        ((0, 20), (20, 40), "M 0 20 L 12 20 Q 20 20 20 28 L 20 40 "),
        ((0, 20), (20, 0), "M 0 20 L 12 20 Q 20 20 20 12 L 20 0 "),
        ((40, 20), (20, 40), "M 40 20 L 28 20 Q 20 20 20 28 L 20 40 "),
        ((40, 20), (20, 0), "M 40 20 L 28 20 Q 20 20 20 12 L 20 0 "),
        ((20, 0), (40, 20), "M 20 0 L 20 12 Q 20 20 28 20 L 40 20 "),
        ((20, 0), (0, 20), "M 20 0 L 20 12 Q 20 20 12 20 L 0 20 "),
        ((20, 40), (40, 20), "M 20 40 L 20 28 Q 20 20 28 20 L 40 20 "),
        ((20, 40), (0, 20), "M 20 40 L 20 28 Q 20 20 12 20 L 0 20 "),
    ] {
        assert_eq!(rounded(&points(&[before, (20, 20), after])), expected);
    }
}

#[test]
fn clamps_both_sides_of_short_segments_and_preserves_arrow_tangents() {
    assert_eq!(
        rounded(&points(&[(0, 0), (3, 0), (3, 1), (6, 1)])),
        "M 0 0 L 2.5 0 Q 3 0 3 0.5 L 3 0.5 Q 3 1 3.5 1 L 6 1 "
    );
    assert_eq!(
        rounded(&points(&[(0, 0), (1, 0), (1, 30)])),
        "M 0 0 L 0.5 0 Q 1 0 1 0.5 L 1 30 "
    );
    assert_eq!(
        rounded(&points(&[(0, 0), (30, 0), (30, 1)])),
        "M 0 0 L 29.5 0 Q 30 0 30 0.5 L 30 1 "
    );
}

#[test]
fn straight_degenerate_and_reversing_paths_do_not_gain_curves() {
    for (input, expected) in [
        (vec![], ""),
        (vec![(2, 4)], "M 2 4 "),
        (vec![(2, 4), (2, 4)], "M 2 4 "),
        (vec![(0, 0), (10, 0), (20, 0)], "M 0 0 L 10 0 L 20 0 "),
        (vec![(5, 20), (5, 10), (5, 0)], "M 5 20 L 5 10 L 5 0 "),
        (vec![(0, 0), (10, 0), (0, 0)], "M 0 0 L 10 0 L 0 0 "),
    ] {
        assert_eq!(rounded(&points(&input)), expected);
    }
    assert_eq!(
        rounded(&points(&[(0, 0), (20, 0), (20, 0), (20, 20)])),
        "M 0 0 L 12 0 Q 20 0 20 8 L 20 20 "
    );
}

#[test]
fn svg_rounding_keeps_layout_points_and_arrow_markers_intact() {
    let raw = crate::plan::parse(include_str!("../../../examples/bundling-plan.json")).unwrap();
    let graph = crate::semantic::transform(&raw);
    let layout = crate::layout::Layout::new(&graph);
    let original = layout.paths.clone();
    let output = crate::svg::render(&graph, &layout);
    assert!(output.contains(" Q "));
    assert_eq!(output.matches("marker-end=").count(), graph.edges.len());
    assert_eq!(layout.paths, original);
    assert_eq!(output, crate::svg::render(&graph, &layout));
}
