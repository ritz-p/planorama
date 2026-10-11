use super::*;

#[test]
fn empty_single_and_wrapped_cards_have_deterministic_bounds() {
    for count in [0, 1, 7] {
        let cards = (0..count)
            .map(|i| Card {
                width: 300,
                height: 80 + i * 10,
                content: format!("<text>{i}</text>"),
            })
            .collect();
        let area = Area::new(cards, 720);
        assert_eq!(area.width, 720);
        if count == 0 {
            assert_eq!(area.height, 0);
            assert!(area.render().is_empty());
            continue;
        }
        for (i, (_, bounds)) in area.cards.iter().enumerate() {
            assert_eq!(bounds.origin.x, 40 + i % 2 * 320);
            assert!(bounds.right() <= area.width - 40);
            assert!(bounds.origin.y + bounds.height <= 160 + area.height - 24);
            if i >= 2 {
                assert!(
                    bounds.origin.y > area.cards[i - 2].1.origin.y + area.cards[i - 2].1.height
                );
            }
        }
        assert_eq!(area.render(), area.render());
    }
    let area = Area::new(
        vec![Card {
            width: 800,
            height: 80,
            content: String::new(),
        }],
        400,
    );
    assert_eq!(area.width, 880);
    assert_eq!(area.height, 104);
}

#[test]
fn card_rows_use_the_tallest_content_and_wrap_only_when_width_requires_it() {
    let make_cards = || {
        [120, 400, 80]
            .into_iter()
            .map(|height| Card {
                width: 360,
                height,
                content: String::new(),
            })
            .collect()
    };
    let wide = Area::new(make_cards(), 1200);
    assert_eq!(
        wide.cards.iter().map(|(_, b)| b.origin).collect::<Vec<_>>(),
        vec![
            Point { x: 40, y: 160 },
            Point { x: 420, y: 160 },
            Point { x: 800, y: 160 },
        ]
    );
    assert_eq!(wide.height, 424);
    let narrow = Area::new(make_cards(), 820);
    assert_eq!(narrow.cards[1].1.origin, Point { x: 420, y: 160 });
    assert_eq!(narrow.cards[2].1.origin, Point { x: 40, y: 580 });
    assert_eq!(narrow.height, 524);
    let one_per_row = Area::new(make_cards(), 819);
    assert_eq!(one_per_row.cards[1].1.origin, Point { x: 40, y: 300 });
    assert_eq!(one_per_row.cards[2].1.origin, Point { x: 40, y: 720 });
}

#[test]
fn annotations_precede_architecture_without_mutating_routing() {
    for fixture in [
        include_str!("../../fixtures/association-plan.json"),
        include_str!("../../fixtures/components-plan.json"),
    ] {
        let graph = crate::semantic::transform(&crate::plan::parse(fixture).unwrap()).0;
        let layout = crate::layout::Layout::new(&graph);
        let bounds = layout.bounds.clone();
        let paths = layout.paths.clone();
        let svg = crate::svg::render(&graph, &layout);
        let doc = roxmltree::Document::parse(&svg).unwrap();
        let area = doc
            .descendants()
            .find(|n| n.attribute("data-annotation-area").is_some())
            .unwrap();
        let architecture = doc
            .descendants()
            .find(|n| n.attribute("data-architecture").is_some())
            .unwrap();
        assert!(area.range().end < architecture.range().start);
        assert!(
            area.descendants()
                .all(|n| n.attribute("data-terraform-address").is_none())
        );
        assert_eq!(layout.bounds, bounds);
        assert_eq!(layout.paths, paths);
        assert!(architecture.attribute("transform").unwrap() != "translate(0 0)");
    }
}
