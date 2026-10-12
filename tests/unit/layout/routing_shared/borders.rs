use super::*;
use crate::model::EdgeKind;

fn frame(radius: usize, stroke: usize) -> Frame {
    Frame {
        bounds: Bounds {
            origin: Point { x: 100, y: 100 },
            width: 240,
            height: 240,
        },
        radius,
        stroke,
        node: None,
    }
}

fn parallel(horizontal: bool, coordinate: usize, from: usize, to: usize) -> [Point; 2] {
    if horizontal {
        [
            Point {
                x: from,
                y: coordinate,
            },
            Point {
                x: to,
                y: coordinate,
            },
        ]
    } else {
        [
            Point {
                x: coordinate,
                y: from,
            },
            Point {
                x: coordinate,
                y: to,
            },
        ]
    }
}

#[test]
fn short_and_long_contacts_measure_empty_stroke_gap_for_every_frame_and_edge() {
    for frame in [
        frame(geometry::CARD_RADIUS, geometry::CARD_STROKE),
        frame(geometry::CONTAINER_RADIUS, geometry::CONTAINER_STROKE),
        frame(geometry::ROUTE_RADIUS, geometry::ROUTE_STROKE),
    ] {
        let index = Index::new(vec![frame]);
        for kind in [
            EdgeKind::Dependency,
            EdgeKind::Association,
            EdgeKind::Connection,
        ] {
            let stroke = geometry::relationship_stroke(kind);
            let half_strokes = ((stroke + frame.stroke) / 2) as i64;
            for horizontal in [false, true] {
                for length in [4, 12, 32, 63, 64, 160] {
                    for offset in [0, 1, 2, 4, 8] {
                        for coordinate in [100 - offset, 100 + offset] {
                            let path = parallel(horizontal, coordinate, 140, 140 + length);
                            let metrics = index.path(&path, stroke, &[]);
                            let gap = (offset * 4) as i64 - half_strokes;
                            assert_eq!(metrics.minimum, Some(gap), "{frame:?} {kind:?} {path:?}");
                            assert_eq!(metrics.close_length, length);
                            assert_eq!(
                                metrics.minimum_deficit,
                                (16 - gap).max(0) as u128 * length as u128
                            );
                            assert_eq!(
                                metrics.preferred_deficit,
                                (32 - gap) as u128 * length as u128
                            );
                            assert!(metrics.cost() > 0);
                            let contacts = index.contacts(&path, stroke, &[]);
                            assert_eq!(contacts.len(), 1);
                            assert_eq!((contacts[0].low, contacts[0].high), (140, 140 + length));
                            assert_eq!(contacts[0].minimum, frame.clearance(stroke, MINIMUM));
                            assert_eq!(contacts[0].preferred, frame.clearance(stroke, PREFERRED));
                        }
                    }
                    for gap in [MINIMUM, PREFERRED] {
                        let offset = frame.clearance(stroke, gap);
                        let path = parallel(horizontal, 100 - offset, 140, 140 + length);
                        let metrics = index.path(&path, stroke, &[]);
                        assert_eq!(metrics.minimum_deficit, 0);
                        if gap == PREFERRED {
                            assert_eq!(metrics.cost(), 0);
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn local_tracks_convert_visible_gaps_to_centerline_offsets() {
    let card = frame(geometry::CARD_RADIUS, geometry::CARD_STROKE);
    let container = frame(geometry::CONTAINER_RADIUS, geometry::CONTAINER_STROKE);
    assert_eq!(card.clearance(geometry::DEPENDENCY_STROKE, MINIMUM), 6);
    assert_eq!(card.clearance(geometry::CONNECTION_STROKE, PREFERRED), 10);
    assert_eq!(container.clearance(geometry::CONNECTION_STROKE, MINIMUM), 7);
    assert_eq!(
        container.clearance(geometry::CONNECTION_STROKE, PREFERRED),
        11
    );
    let start = Point { x: 80, y: 100 };
    let end = Point { x: 200, y: 100 };
    let local = Index::new(vec![card]).axes(start, end, geometry::DEPENDENCY_STROKE);
    for coordinate in [90, 94, 106, 110] {
        assert!(local.contains(&(true, coordinate)));
    }
    let mut frames = vec![card];
    frames.extend((0..256).map(|i| Frame {
        bounds: Bounds {
            origin: Point {
                x: 2000 + i * 500,
                y: 100,
            },
            ..card.bounds
        },
        ..card
    }));
    assert_eq!(
        Index::new(frames).axes(start, end, geometry::DEPENDENCY_STROKE),
        local
    );
}

#[test]
fn rounded_corner_contacts_are_short_positive_and_subdivision_invariant() {
    let index = Index::new(vec![frame(geometry::CARD_RADIUS, geometry::CARD_STROKE)]);
    let stroke = geometry::DEPENDENCY_STROKE;
    for horizontal in [false, true] {
        for coordinate in [96, 99, 100, 104] {
            let [a, b] = parallel(horizontal, coordinate, 86, 120);
            let whole = index.segment(a, b, stroke, &[]);
            assert!(whole.cost() > 0);
            assert_eq!(whole, index.segment(b, a, stroke, &[]));
            for split in 87..120 {
                let midpoint = if horizontal {
                    Point {
                        x: split,
                        y: coordinate,
                    }
                } else {
                    Point {
                        x: coordinate,
                        y: split,
                    }
                };
                let mut pieces = index.segment(a, midpoint, stroke, &[]);
                pieces.merge(index.segment(midpoint, b, stroke, &[]));
                assert_eq!(whole, pieces);
            }
        }
        let corner = parallel(horizontal, 100, 100, 108);
        let contacts = index.contacts(&corner, stroke, &[]);
        assert!(!contacts.is_empty());
        assert!(
            contacts
                .iter()
                .all(|contact| contact.high - contact.low <= 8)
        );
        assert!(index.path(&corner, stroke, &[]).minimum_deficit > 0);
    }
}

#[test]
fn preferred_tracks_stay_clear_across_straight_and_rounded_frame_joins() {
    for frame in [
        frame(geometry::CARD_RADIUS, geometry::CARD_STROKE),
        frame(geometry::CONTAINER_RADIUS, geometry::CONTAINER_STROKE),
        frame(geometry::ROUTE_RADIUS, geometry::ROUTE_STROKE),
    ] {
        let index = Index::new(vec![frame]);
        for kind in [
            EdgeKind::Dependency,
            EdgeKind::Association,
            EdgeKind::Connection,
        ] {
            let stroke = geometry::relationship_stroke(kind);
            let offset = frame.clearance(stroke, PREFERRED);
            for horizontal in [false, true] {
                let path = parallel(horizontal, 100 - offset, 80, 360);
                let metrics = index.path(&path, stroke, &[]);
                assert_eq!(metrics.preferred_deficit, 0, "{frame:?} {kind:?} {path:?}");
                assert_eq!(metrics.minimum_deficit, 0);
                assert_eq!(metrics.cost(), 0);
            }
        }
    }
}

#[test]
fn perpendicular_boundary_crossings_and_normal_endpoint_stubs_are_free() {
    let index = Index::new(vec![frame(
        geometry::CONTAINER_RADIUS,
        geometry::CONTAINER_STROKE,
    )]);
    for path in [
        [Point { x: 220, y: 80 }, Point { x: 220, y: 360 }],
        [Point { x: 80, y: 220 }, Point { x: 360, y: 220 }],
        [Point { x: 100, y: 220 }, Point { x: 80, y: 220 }],
        [Point { x: 220, y: 100 }, Point { x: 220, y: 80 }],
    ] {
        assert_eq!(
            index.path(&path, geometry::CONNECTION_STROKE, &[]).cost(),
            0
        );
        assert!(
            index
                .contacts(&path, geometry::CONNECTION_STROKE, &[])
                .is_empty()
        );
    }
}

#[test]
fn masks_hide_covered_path_pieces_without_exempting_visible_exterior_borders() {
    let container = frame(geometry::CONTAINER_RADIUS, geometry::CONTAINER_STROKE);
    let index = Index::new(vec![container]);
    let mask = Occluder {
        bounds: container.bounds,
        cost: 1,
        step: 2,
    };
    for horizontal in [false, true] {
        let covered = parallel(horizontal, 100, 140, 300);
        let outside = parallel(horizontal, 96, 140, 300);
        assert_eq!(
            index
                .path(&covered, geometry::CONNECTION_STROKE, &[mask])
                .cost(),
            0
        );
        assert!(
            index
                .contacts(&covered, geometry::CONNECTION_STROKE, &[mask])
                .is_empty()
        );
        assert!(
            index
                .path(&outside, geometry::CONNECTION_STROKE, &[mask])
                .cost()
                > 0
        );
        let [a, b] = parallel(horizontal, 100, 86, 140);
        let midpoint = if horizontal {
            Point { x: 110, y: 100 }
        } else {
            Point { x: 100, y: 110 }
        };
        let whole = index.segment(a, b, geometry::CONNECTION_STROKE, &[mask]);
        let mut pieces = index.segment(a, midpoint, geometry::CONNECTION_STROKE, &[mask]);
        pieces.merge(index.segment(midpoint, b, geometry::CONNECTION_STROKE, &[mask]));
        assert_eq!(whole, pieces);
    }
    let child = Frame {
        bounds: Bounds {
            origin: Point { x: 160, y: 160 },
            width: 100,
            height: 100,
        },
        ..Frame::card(container.bounds)
    };
    let index = Index::new(vec![container, child]);
    let behind = parallel(true, 160, 180, 240);
    assert!(index.path(&behind, geometry::DEPENDENCY_STROKE, &[]).cost() > 0);
    assert_eq!(
        index
            .path(&behind, geometry::DEPENDENCY_STROKE, &[mask])
            .cost(),
        0
    );
}

#[test]
fn ordinary_card_fill_hides_interiors_but_lane_reservations_do_not() {
    let card = frame(geometry::CARD_RADIUS, geometry::CARD_STROKE);
    let index = Index::new(vec![card]);
    let inside = parallel(true, 104, 140, 300);
    let outside = parallel(true, 96, 140, 300);
    assert_eq!(
        index
            .path(&inside, geometry::DEPENDENCY_STROKE, &[card.bounds.into()])
            .cost(),
        0
    );
    assert!(
        index
            .path(&outside, geometry::DEPENDENCY_STROKE, &[card.bounds.into()])
            .cost()
            > 0
    );
    let lane = Occluder {
        bounds: card.bounds,
        cost: 1,
        step: 1,
    };
    assert!(
        index
            .path(&inside, geometry::DEPENDENCY_STROKE, &[lane])
            .cost()
            > 0
    );
}

#[test]
fn final_metrics_measure_the_rendered_quadratic_corner() {
    let square = Frame {
        bounds: Bounds {
            origin: Point { x: 95, y: 95 },
            width: 10,
            height: 10,
        },
        radius: 0,
        ..Frame::card(Bounds {
            origin: Point { x: 0, y: 0 },
            width: 10,
            height: 10,
        })
    };
    let index = Index::new(vec![square]);
    let path = [
        Point { x: 120, y: 90 },
        Point { x: 90, y: 90 },
        Point { x: 90, y: 120 },
    ];
    let mut straight = index.segment(path[0], path[1], geometry::DEPENDENCY_STROKE, &[]);
    straight.merge(index.segment(path[1], path[2], geometry::DEPENDENCY_STROKE, &[]));
    let rounded = index.path(&path, geometry::DEPENDENCY_STROKE, &[]);
    assert!(rounded.minimum < straight.minimum);
    assert!(rounded.minimum_deficit > 0);
    let mask = Occluder {
        bounds: Bounds {
            origin: Point { x: 70, y: 70 },
            width: 100,
            height: 100,
        },
        cost: 1,
        step: 2,
    };
    assert_eq!(
        index
            .path(&path, geometry::DEPENDENCY_STROKE, &[mask])
            .cost(),
        0
    );
}

#[test]
fn curve_only_deficits_request_room_for_the_adjoining_leg_and_corner() {
    let container = frame(geometry::CONTAINER_RADIUS, geometry::CONTAINER_STROKE);
    let index = Index::new(vec![container]);
    let path = [
        Point { x: 220, y: 80 },
        Point { x: 220, y: 112 },
        Point { x: 250, y: 112 },
    ];
    let stroke = geometry::CONNECTION_STROKE;
    assert!(
        path.windows(2)
            .all(|p| index.segment(p[0], p[1], stroke, &[]).minimum_deficit == 0)
    );
    assert!(index.path(&path, stroke, &[]).minimum_deficit > 0);
    let contacts = index.contacts(&path, stroke, &[]);
    let contact = contacts
        .iter()
        .find(|contact| contact.segment == 1 && contact.side == Side::Top)
        .unwrap();
    assert_eq!((contact.low, contact.high), (220, 228));
    assert_eq!((contact.minimum, contact.preferred), (15, 19));
    let moved = [path[0], Point { x: 220, y: 119 }, Point { x: 250, y: 119 }];
    assert_eq!(index.path(&moved, stroke, &[]).minimum_deficit, 0);
}

#[test]
fn preferred_only_curve_contacts_are_not_urgent() {
    let index = Index::new(vec![frame(
        geometry::CONTAINER_RADIUS,
        geometry::CONTAINER_STROKE,
    )]);
    let path = [
        Point { x: 220, y: 80 },
        Point { x: 220, y: 116 },
        Point { x: 250, y: 116 },
    ];
    let stroke = geometry::CONNECTION_STROKE;
    assert!(
        path.windows(2)
            .all(|p| index.segment(p[0], p[1], stroke, &[]).preferred_deficit == 0)
    );
    let metrics = index.path(&path, stroke, &[]);
    assert_eq!(metrics.minimum_deficit, 0);
    assert!(metrics.preferred_deficit > 0);
    let contacts = index.contacts(&path, stroke, &[]);
    assert!(!contacts.is_empty());
    assert!(contacts.iter().all(|c| !c.urgent));
    assert!(
        contacts
            .iter()
            .any(|c| c.segment == 1 && c.side == Side::Top)
    );
}

#[test]
fn inline_route_frames_match_rendered_cards_and_charge_nearby_paths() {
    let mut value: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/routes-plan.json")).unwrap();
    value["configuration"]["root_module"]["resources"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::json!({
            "address": "aws_route_table.public",
            "expressions": {"route": {"constant_value": [{"cidr_block": "0.0.0.0/0"}]}}
        }));
    let graph = crate::semantic::transform(&crate::plan::parse(&value.to_string()).unwrap()).0;
    let layout = Layout::new(&graph);
    let frames = frames(&graph, &layout);
    let svg = crate::svg::render(&graph, &layout);
    let doc = roxmltree::Document::parse(&svg).unwrap();
    let rect = doc
        .descendants()
        .find(|n| n.attribute("data-route-entry") == Some("inline"))
        .unwrap()
        .children()
        .find(|n| n.has_tag_name("rect"))
        .unwrap();
    let number = |name| rect.attribute(name).unwrap().parse::<usize>().unwrap();
    let bounds = Bounds {
        origin: Point {
            x: number("x"),
            y: number("y"),
        },
        width: number("width"),
        height: number("height"),
    };
    let frame = frames.iter().find(|f| f.bounds == bounds).unwrap();
    assert_eq!(
        (frame.radius, frame.stroke, frame.node),
        (geometry::ROUTE_RADIUS, geometry::ROUTE_STROKE, None)
    );
    let index = Index::new(vec![*frame]);
    let path = parallel(
        true,
        bounds.origin.y - 2,
        bounds.origin.x + 20,
        bounds.right() - 20,
    );
    assert!(
        index
            .path(&path, geometry::CONNECTION_STROKE, &[])
            .minimum_deficit
            > 0
    );
}
