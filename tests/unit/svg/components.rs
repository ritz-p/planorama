use super::*;

fn graph() -> Graph {
    crate::semantic::transform(
        &crate::plan::parse(include_str!("../../fixtures/components-plan.json")).unwrap(),
    )
    .0
}

#[test]
fn component_cards_keep_identity_members_operations_and_links() {
    let graph = graph();
    let layout = crate::layout::Layout::new(&graph);
    let svg = crate::svg::render(&graph, &layout);
    let doc = roxmltree::Document::parse(&svg).unwrap();
    for component in &graph.components {
        let panel = doc
            .descendants()
            .find(|n| n.attribute("data-architecture-id") == Some(component.entity.id.as_str()))
            .unwrap();
        assert_eq!(
            panel.attribute("data-component-kind"),
            Some(component.kind.as_str())
        );
        assert!(
            panel
                .ancestors()
                .any(|n| n.attribute("data-annotation-card").is_some())
        );
        for address in &component.members {
            let node = graph.nodes.iter().find(|n| &n.address == address).unwrap();
            let link = panel
                .descendants()
                .find(|n| n.attribute("data-source-address") == Some(address.as_str()))
                .unwrap();
            assert_eq!(
                link.attribute("href"),
                Some(format!("#{}", crate::svg::identity::resource(node)).as_str())
            );
            let text: String = link
                .descendants()
                .filter(|n| n.has_tag_name("tspan"))
                .filter_map(|n| n.text())
                .collect();
            assert_eq!(
                text,
                format!(
                    "{}: {address}",
                    crate::svg::operations::label(node.action, &node.metadata)
                )
            );
        }
    }
    let mut plain = graph.clone();
    plain.components.clear();
    let plain_layout = crate::layout::Layout::new(&plain);
    assert_eq!(layout.bounds, plain_layout.bounds);
    assert_eq!(layout.paths, plain_layout.paths);
}

#[test]
fn long_member_lists_and_multiple_components_wrap_without_losing_text() {
    let mut graph = graph();
    let template = graph.components[0].clone();
    graph.components.clear();
    for i in 0..5 {
        let mut component = template.clone();
        component.id = format!("component-{i}");
        component.label = format!("Group {i} {}", "長い名前".repeat(24));
        component.entity = crate::model::ArchitectureEntity::synthetic(
            "test",
            &component.id,
            template.entity.provenance.clone(),
        );
        component.members.extend((0..12).map(|member| {
            format!(
                "module.{}.test.member{member}",
                "長いモジュール.".repeat(20)
            )
        }));
        graph.components.push(component);
    }
    let cards = cards(&graph);
    assert_eq!(cards.len(), 5);
    for card in &cards {
        assert!(card.width <= 640);
        let doc = roxmltree::Document::parse(&card.content).unwrap();
        let members: Vec<_> = doc
            .descendants()
            .filter(|n| n.attribute("data-source-address").is_some())
            .collect();
        for member in members {
            let visible: String = member
                .descendants()
                .filter(|n| n.has_tag_name("tspan"))
                .filter_map(|n| n.text())
                .collect();
            assert!(visible.ends_with(member.attribute("data-source-address").unwrap()));
        }
        for row in doc.descendants().filter(|n| n.has_tag_name("tspan")) {
            assert!(row.attribute("y").unwrap().parse::<usize>().unwrap() + 4 < card.height);
            let width: usize = row
                .text()
                .unwrap()
                .chars()
                .map(|c| if c.is_ascii() { 8 } else { 16 })
                .sum();
            assert!(width + 32 <= card.width);
        }
    }
    let expected = crate::svg::annotations::Area::new(cards, 1500).render();
    graph.components.reverse();
    for component in &mut graph.components {
        component.members.reverse();
    }
    assert_eq!(
        crate::svg::annotations::Area::new(super::cards(&graph), 1500).render(),
        expected
    );
    let doc = roxmltree::Document::parse(&expected).unwrap();
    let rows: std::collections::BTreeSet<_> = doc
        .descendants()
        .filter(|n| n.attribute("data-annotation-card").is_some())
        .map(|n| {
            n.attribute("transform")
                .unwrap()
                .split_whitespace()
                .last()
                .unwrap()
        })
        .collect();
    assert!(rows.len() > 1);
}
