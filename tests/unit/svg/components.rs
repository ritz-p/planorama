use super::*;

fn graph() -> Graph {
    crate::semantic::transform(
        &crate::plan::parse(include_str!("../../fixtures/components-plan.json")).unwrap(),
    )
    .0
}

#[test]
fn themed_members_preserve_actions_unique_targets_and_state_identity() {
    use crate::model::{Action, Placement, StateId};
    let mut input: serde_json::Value =
        serde_json::from_str(include_str!("../../fixtures/components-plan.json")).unwrap();
    input["resource_changes"].as_array_mut().unwrap().extend([
        serde_json::json!({"address":"aws_iam_role.app", "type":"aws_iam_role", "previous_address":"aws_iam_role.old", "change":{"actions":["update"], "importing":{"id":"SECRET"}}}),
        serde_json::json!({"address":"data.aws_ami.app", "type":"aws_ami", "mode":"data", "deposed":"old"}),
    ]);
    let mut graph = crate::semantic::transform(&crate::plan::parse(&input.to_string()).unwrap()).0;
    for (node, action) in graph.nodes.iter_mut().zip([
        Action::Create,
        Action::Update,
        Action::Delete,
        Action::Replace,
        Action::Read,
        Action::Unchanged,
        Action::Other,
    ]) {
        node.action = action;
    }
    graph.components[0].members = graph.nodes.iter().map(|n| n.address.clone()).collect();
    graph.components[0].members.push("aws_lb.missing".into());
    let svg = crate::svg::render(&graph, &crate::layout::Layout::new(&graph));
    let doc = roxmltree::Document::parse(&svg).unwrap();
    for (name, background, border) in [
        ("Logical Resources", "#ecfeff", "#0891b2"),
        ("Logical Components", "#f5f3ff", "#a78bfa"),
    ] {
        let section = doc
            .descendants()
            .find(|n| n.attribute("data-annotation-section") == Some(name))
            .unwrap();
        let rect = section.children().find(|n| n.has_tag_name("rect")).unwrap();
        assert_eq!(rect.attribute("fill"), Some(background));
        assert_eq!(rect.attribute("stroke"), Some(border));
    }
    for node in &graph.nodes {
        let member = doc
            .descendants()
            .find(|n| {
                n.attribute("data-source-address") == Some(node.address.as_str())
                    && n.has_tag_name("a")
            })
            .unwrap();
        let rect = member.children().find(|n| n.has_tag_name("rect")).unwrap();
        let (fill, stroke) = crate::svg::color(node.action);
        assert_eq!(rect.attribute("fill"), Some(fill));
        assert_eq!(rect.attribute("stroke"), Some(stroke));
        let badge = member
            .children()
            .find(|n| n.attribute("data-import-badge").is_some());
        assert_eq!(badge.is_some(), node.metadata.import.is_some());
        if let Some(badge) = badge {
            let badge_rect = badge.children().find(|n| n.has_tag_name("rect")).unwrap();
            let bottom = badge_rect.attribute("y").unwrap().parse::<usize>().unwrap()
                + badge_rect
                    .attribute("height")
                    .unwrap()
                    .parse::<usize>()
                    .unwrap();
            let first = member
                .descendants()
                .find(|n| n.has_tag_name("tspan"))
                .unwrap();
            assert!(bottom + 10 < first.attribute("y").unwrap().parse::<usize>().unwrap());
            let height = rect.attribute("height").unwrap().parse::<usize>().unwrap();
            assert!(
                member
                    .descendants()
                    .filter(|n| n.has_tag_name("tspan"))
                    .all(|n| n.attribute("y").unwrap().parse::<usize>().unwrap() + 4 < height)
            );
        }
        let target = member.attribute("href").unwrap().strip_prefix('#').unwrap();
        let targets: Vec<_> = doc
            .descendants()
            .filter(|n| n.attribute("id") == Some(target))
            .collect();
        assert_eq!(targets.len(), 1);
        if node.placement == Placement::Logical {
            assert!(
                targets[0]
                    .ancestors()
                    .any(|n| n.attribute("data-annotation-section") == Some("Logical Resources"))
            );
        }
        let title = member
            .children()
            .find(|n| n.has_tag_name("title"))
            .unwrap()
            .text()
            .unwrap();
        if let Some(previous) = &node.previous_address {
            assert!(title.contains(previous));
        }
        if node.metadata.import.is_some() {
            assert!(title.contains("import / "));
            assert!(
                targets[0]
                    .children()
                    .any(|n| n.attribute("data-import-badge").is_some())
            );
        }
        if let Some(key) = &node.deposed_key {
            assert!(title.contains(&format!("deposed: {key}")));
        }
    }
    assert!(!svg.contains("SECRET"));
    let missing = doc
        .descendants()
        .find(|n| n.attribute("data-source-address") == Some("aws_lb.missing"))
        .unwrap();
    assert!(missing.attribute("href").is_none());
    assert!(
        !missing
            .children()
            .any(|n| n.attribute("data-import-badge").is_some())
    );
    assert!(
        doc.descendants()
            .filter(|n| n.attribute("data-entity-kind") == Some("synthetic"))
            .all(|n| n.attribute("data-import").is_none()
                && !n
                    .children()
                    .any(|c| c.attribute("data-import-badge").is_some()))
    );
    assert_eq!(
        missing
            .children()
            .find(|n| n.has_tag_name("rect"))
            .unwrap()
            .attribute("fill"),
        Some("#f8fafc")
    );
    assert_eq!(
        doc.descendants()
            .filter(|n| n.attribute("data-terraform-address").is_some())
            .count(),
        graph.nodes.len()
    );
    graph.components[0]
        .members
        .retain(|a| a != "aws_lb.missing");
    let architecture = crate::model::architecture::Architecture {
        states: BTreeMap::from([
            (
                StateId::new("first").unwrap(),
                crate::model::ArchitectureGraph(graph.clone()),
            ),
            (
                StateId::new("second").unwrap(),
                crate::model::ArchitectureGraph(graph),
            ),
        ]),
        relationships: vec![],
    };
    let selected = crate::view::apply(&architecture, &Default::default()).unwrap();
    let svg = crate::svg::render_states(&selected, crate::svg::AddressFormat::Qualified);
    let doc = roxmltree::Document::parse(&svg).unwrap();
    let mut ids = std::collections::BTreeSet::new();
    for id in doc.descendants().filter_map(|n| n.attribute("id")) {
        assert!(ids.insert(id));
    }
    for link in doc
        .descendants()
        .filter(|n| n.has_tag_name("a") && n.attribute("data-source-address").is_some())
    {
        let target = doc
            .descendants()
            .find(|n| n.attribute("id") == link.attribute("href").and_then(|v| v.strip_prefix('#')))
            .unwrap();
        let state = |n: roxmltree::Node| {
            n.ancestors()
                .find(|n| n.attribute("data-state-id").is_some())
                .unwrap()
                .attribute("data-state-id")
                .unwrap()
                .to_string()
        };
        assert_eq!(state(link), state(target));
    }
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
                    crate::svg::operations::action_label(node.action, &node.metadata)
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
fn empty_components_and_ambiguous_member_addresses_do_not_invent_links() {
    let mut graph = graph();
    let duplicate = graph
        .nodes
        .iter()
        .find(|n| n.address == "aws_lb.app")
        .unwrap()
        .clone();
    graph.nodes.push(duplicate);
    let output = cards(&graph).remove(0);
    let doc = roxmltree::Document::parse(&output.content).unwrap();
    let member = doc
        .descendants()
        .find(|n| n.attribute("data-source-address") == Some("aws_lb.app"))
        .unwrap();
    assert!(member.attribute("href").is_none());
    assert_eq!(member.attribute("data-member-status"), Some("unavailable"));
    graph.components[0].members.clear();
    let empty = cards(&graph).remove(0);
    assert!(empty.height > 28);
    assert!(empty.height < output.height);
    assert!(!empty.content.contains(">Members</text>"));
    assert!(!empty.content.contains("data-source-address"));
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
