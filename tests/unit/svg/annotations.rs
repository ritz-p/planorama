use super::*;

#[test]
fn ordered_sections_wrap_children_and_omit_empty_sections_without_gaps() {
    for (width, count) in [400, 2200, 12000]
        .into_iter()
        .flat_map(|width| [1, 2, 7].map(|count| (width, count)))
    {
        for mask in 0..8 {
            let sections = [
                ("Logical Resources", crate::svg::style::LOGICAL_RESOURCES),
                ("Logical Components", crate::svg::style::LOGICAL_COMPONENTS),
                (
                    "Relationship Resources",
                    crate::svg::style::RELATIONSHIP_RESOURCES,
                ),
            ]
            .into_iter()
            .enumerate()
            .map(|(i, (name, palette))| {
                let cards = (0..if mask & (1 << i) == 0 { 0 } else { count })
                    .map(|n| Card {
                        width: 360 + n * 20,
                        height: 80 + n * 30,
                        content: String::new(),
                    })
                    .collect();
                (name, palette, cards)
            })
            .collect();
            let area = Area::sections(sections, width);
            assert_eq!(
                area.width,
                if mask == 0 {
                    width
                } else {
                    width.max(360 + (count - 1) * 20 + 112)
                }
            );
            if mask == 0 {
                assert_eq!(area.height, 0);
                assert!(area.render().is_empty());
                continue;
            }
            let rendered = area.render();
            let doc = roxmltree::Document::parse(&rendered).unwrap();
            let sections: Vec<_> = doc
                .descendants()
                .filter(|n| n.attribute("data-annotation-section").is_some())
                .collect();
            let expected: Vec<_> = [
                "Logical Resources",
                "Logical Components",
                "Relationship Resources",
            ]
            .into_iter()
            .enumerate()
            .filter(|(i, _)| mask & (1 << i) != 0)
            .map(|(_, name)| name)
            .collect();
            assert_eq!(
                sections
                    .iter()
                    .map(|n| n.attribute("data-annotation-section").unwrap())
                    .collect::<Vec<_>>(),
                expected
            );
            let number = |node: roxmltree::Node, attribute| {
                node.attribute(attribute).unwrap().parse::<usize>().unwrap()
            };
            let position = |node: roxmltree::Node| {
                let values: Vec<usize> = node
                    .attribute("transform")
                    .unwrap()
                    .trim_start_matches("translate(")
                    .trim_end_matches(')')
                    .split_whitespace()
                    .map(|v| v.parse().unwrap())
                    .collect();
                (values[0], values[1])
            };
            let mut bottom = 140;
            for section in sections {
                let parent = section.parent().unwrap();
                let (x, y) = position(parent);
                assert_eq!(x, 40);
                assert_eq!(y, bottom + 20);
                let rect = section.children().find(|n| n.has_tag_name("rect")).unwrap();
                let (w, h) = (number(rect, "width"), number(rect, "height"));
                bottom = y + h;
                let mut bounds = Vec::<(usize, usize, usize, usize)>::new();
                for child in section
                    .children()
                    .filter(|n| n.attribute("data-annotation-card").is_some())
                {
                    let (cx, cy) = position(child);
                    let rect = child.children().find(|n| n.has_tag_name("rect")).unwrap();
                    let (cw, ch) = (number(rect, "width"), number(rect, "height"));
                    assert!(cx >= 16 && cy >= 48 && cx + cw <= w - 16 && cy + ch <= h - 16);
                    for &(px, py, pw, ph) in &bounds {
                        assert!(cx >= px + pw || px >= cx + cw || cy >= py + ph || py >= cy + ch);
                    }
                    bounds.push((cx, cy, cw, ch));
                }
                assert_eq!(bounds.len(), count);
                assert_eq!(
                    w,
                    bounds.iter().map(|(x, _, w, _)| x + w).max().unwrap() + 16
                );
                assert_eq!(
                    h,
                    bounds.iter().map(|(_, y, _, h)| y + h).max().unwrap() + 16
                );
                if width == 12000 {
                    assert!(bounds.iter().all(|(_, y, _, _)| *y == 48));
                    assert!(w < width - 80);
                }
            }
            assert_eq!(area.height, bottom + 24 - 160);
        }
    }
}

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
        let area = Area::flow(cards, 720, Point { x: 40, y: 160 }, 40, 24);
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
    let area = Area::flow(
        vec![Card {
            width: 800,
            height: 80,
            content: String::new(),
        }],
        400,
        Point { x: 40, y: 160 },
        40,
        24,
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
    let wide = Area::flow(make_cards(), 1200, Point { x: 40, y: 160 }, 40, 24);
    assert_eq!(
        wide.cards.iter().map(|(_, b)| b.origin).collect::<Vec<_>>(),
        vec![
            Point { x: 40, y: 160 },
            Point { x: 420, y: 160 },
            Point { x: 800, y: 160 },
        ]
    );
    assert_eq!(wide.height, 424);
    let narrow = Area::flow(make_cards(), 820, Point { x: 40, y: 160 }, 40, 24);
    assert_eq!(narrow.cards[1].1.origin, Point { x: 420, y: 160 });
    assert_eq!(narrow.cards[2].1.origin, Point { x: 40, y: 580 });
    assert_eq!(narrow.height, 524);
    let one_per_row = Area::flow(make_cards(), 819, Point { x: 40, y: 160 }, 40, 24);
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
        let legend = doc
            .descendants()
            .find(|n| n.attribute("id") == Some("relationship-legend"))
            .unwrap();
        let number = |node: roxmltree::Node, name| {
            node.attribute(name)
                .unwrap_or("0")
                .parse::<usize>()
                .unwrap()
        };
        let translated_y = |node: roxmltree::Node| {
            node.ancestors()
                .filter_map(|parent| parent.attribute("transform"))
                .map(|value| {
                    value
                        .strip_prefix("translate(")
                        .unwrap()
                        .strip_suffix(')')
                        .unwrap()
                        .split_whitespace()
                        .nth(1)
                        .unwrap()
                        .parse::<usize>()
                        .unwrap()
                })
                .sum::<usize>()
        };
        let legend_rect = legend.children().find(|n| n.has_tag_name("rect")).unwrap();
        let legend_bottom = translated_y(legend_rect) + number(legend_rect, "height");
        let sections: Vec<_> = area
            .descendants()
            .filter(|n| n.attribute("data-annotation-section").is_some())
            .map(|section| section.children().find(|n| n.has_tag_name("rect")).unwrap())
            .collect();
        let area_top = sections.iter().map(|n| translated_y(*n)).min().unwrap();
        let area_bottom = sections
            .iter()
            .map(|n| translated_y(*n) + number(*n, "height"))
            .max()
            .unwrap();
        assert!(legend_bottom < area_top);
        let checks = doc
            .descendants()
            .find(|n| n.attribute("id") == Some("plan-checks"));
        assert_eq!(checks.is_some(), !graph.checks.is_empty());
        if let Some(checks) = checks {
            let text = checks.children().find(|n| n.has_tag_name("text")).unwrap();
            let baseline = translated_y(text) + number(text, "y");
            assert!(baseline - number(text, "font-size") > legend_bottom);
            assert!(baseline < area_top);
        }
        let status = doc
            .descendants()
            .find(|n| n.attribute("id") == Some("plan-status"));
        assert_eq!(status.is_some(), graph.status != Default::default());
        if let Some(status) = status {
            let text = status.children().find(|n| n.has_tag_name("text")).unwrap();
            assert!(number(text, "y") < translated_y(legend_rect));
        }
        let cards: Vec<_> = architecture
            .descendants()
            .filter(|n| n.attribute("data-terraform-address").is_some())
            .flat_map(|node| node.children().filter(|n| n.has_tag_name("rect")))
            .collect();
        assert!(!cards.is_empty());
        for card in cards {
            let top = translated_y(card) + number(card, "y");
            assert!(top > area_bottom);
            assert!(top + number(card, "height") <= number(doc.root_element(), "height"));
        }
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
