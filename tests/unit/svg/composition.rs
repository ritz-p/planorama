use super::*;

#[test]
fn capped_legend_keeps_offsets_cards_and_routing_paths_on_wider_canvases() {
    let raw = crate::plan::parse(include_str!("../../fixtures/association-plan.json")).unwrap();
    let graph = crate::semantic::transform(&raw);
    let mut layout = Layout::new(&graph);
    assert!(layout.width <= 1100);
    let mut previous = None;
    for width in [1100, 2166] {
        layout.width = width;
        let index = relationship_index::Index::new(&graph, &layout);
        let sections = Sections::new(&graph, &layout, &index, AddressFormat::Qualified);
        let svg = render(&graph, &layout);
        let document = roxmltree::Document::parse(&svg).unwrap();
        let root = document.root_element();
        assert_eq!(number(root, "width"), width);
        assert_eq!(number(root, "height"), layout.height + sections.height());
        assert_eq!(
            root.attribute("viewBox").unwrap(),
            format!("0 0 {width} {}", layout.height + sections.height())
        );
        assert_eq!(root.attribute("role"), Some("img"));
        assert_eq!(root.attribute("aria-labelledby"), Some("title description"));
        let legend = document
            .descendants()
            .find(|node| node.attribute("id") == Some("relationship-legend"))
            .unwrap();
        let rectangle = legend
            .children()
            .find(|node| node.has_tag_name("rect"))
            .unwrap();
        assert_eq!(number(rectangle, "width"), 960);
        let legend_bottom = translated_y(rectangle) + number(rectangle, "height");
        let annotation = document
            .descendants()
            .find(|node| node.attribute("data-annotation-section").is_some())
            .unwrap();
        let annotation_top = translated_y(annotation);
        assert!(annotation_top > legend_bottom);
        let architecture = document
            .descendants()
            .find(|node| node.attribute("data-architecture") == Some("true"))
            .unwrap();
        let architecture_offset = translated_y(architecture);
        assert_eq!(architecture_offset, sections.height());
        let cards: Vec<_> = architecture
            .descendants()
            .filter(|node| node.attribute("data-terraform-address").is_some())
            .map(|node| {
                let id = node.attribute("id").unwrap();
                let rectangle = node
                    .children()
                    .find(|child| child.has_tag_name("rect"))
                    .or_else(|| {
                        architecture
                            .descendants()
                            .find(|outline| {
                                outline.attribute("data-container") == Some(id)
                                    || outline.attribute("data-route-table") == Some(id)
                            })
                            .and_then(|outline| {
                                outline.children().find(|child| child.has_tag_name("rect"))
                            })
                    })
                    .unwrap();
                (
                    node.attribute("data-terraform-address").unwrap().to_owned(),
                    ["x", "y", "width", "height"].map(|name| number(rectangle, name)),
                )
            })
            .collect();
        let paths: Vec<_> = architecture
            .descendants()
            .filter(|node| node.has_tag_name("path") && node.attribute("data-edge-kind").is_some())
            .map(|node| node.attribute("d").unwrap().to_owned())
            .collect();
        assert!(!cards.is_empty() && !paths.is_empty());
        if let Some((height, annotation_y, architecture_y, old_cards, old_paths)) = previous {
            assert_eq!(height, sections.legend.height);
            assert_eq!(annotation_y, annotation_top);
            assert_eq!(architecture_y, architecture_offset);
            assert_eq!(old_cards, cards);
            assert_eq!(old_paths, paths);
        }
        previous = Some((
            sections.legend.height,
            annotation_top,
            architecture_offset,
            cards,
            paths,
        ));
    }
}

fn number(node: roxmltree::Node<'_, '_>, attribute: &str) -> usize {
    node.attribute(attribute).unwrap().parse().unwrap()
}

fn translated_y(node: roxmltree::Node<'_, '_>) -> usize {
    node.ancestors()
        .filter_map(|parent| parent.attribute("transform"))
        .map(|value| {
            value
                .strip_prefix("translate(")
                .unwrap()
                .trim_end_matches(')')
                .split_whitespace()
                .nth(1)
                .unwrap()
                .parse::<usize>()
                .unwrap()
        })
        .sum()
}
