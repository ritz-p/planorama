use super::*;

#[test]
fn member_scope_summaries_preserve_mixed_evidence_and_measure_annotations() {
    use crate::model::{
        AwsAccountScope, ProviderIdentity, RegionScope, architecture::DeploymentScope,
    };
    let mut graph = graph();
    for node in &mut graph.nodes {
        node.entity.scope = Some(DeploymentScope {
            provider: "AWS".into(),
            region: Some(RegionScope("ap-northeast-1".into())),
            account: AwsAccountScope::new("111111111111"),
            global: false,
        });
    }
    let summary = |graph: &Graph| {
        let card = cards(graph).remove(0);
        let doc = roxmltree::Document::parse(&card.content).unwrap();
        for member in doc
            .descendants()
            .filter(|n| n.attribute("data-component-member").is_some())
        {
            let underline = member
                .descendants()
                .find(|n| n.attribute("data-member-underline").is_some())
                .unwrap();
            for scope in member
                .descendants()
                .filter(|n| n.attribute("data-member-scope").is_some())
            {
                assert!(number(scope, "y") + 4 < number(underline, "y1"));
            }
        }
        doc.descendants()
            .filter(|n| n.attribute("data-component-scope").is_some())
            .filter_map(|n| n.text())
            .collect::<Vec<_>>()
            .join("")
    };
    assert_eq!(
        summary(&graph),
        "Member scope: AWS Account: 111111111111 · Region: ap-northeast-1"
    );
    let address = graph.components[0].members[0].clone();
    let index = graph
        .nodes
        .iter()
        .position(|n| n.address == address)
        .unwrap();
    let scope = graph.nodes[index].entity.scope.as_mut().unwrap();
    scope.region = Some(RegionScope("us-east-1".into()));
    scope.account = AwsAccountScope::new("222222222222");
    let mixed = summary(&graph);
    assert!(mixed.contains("Multi-region · Multi-account · Mixed member scope"));
    assert!(mixed.find("ap-northeast-1").unwrap() < mixed.find("us-east-1").unwrap());
    graph.nodes[index].entity.scope = None;
    assert!(summary(&graph).contains("Account partially unknown"));
    assert!(summary(&graph).contains("Region: unknown"));
    graph.nodes[index].provider = ProviderIdentity::InferredTerraform;
    assert!(summary(&graph).contains("Region: not applicable"));
    graph.nodes[index].entity.scope = Some(DeploymentScope {
        provider: "AWS".into(),
        region: None,
        account: None,
        global: true,
    });
    assert!(summary(&graph).contains("AWS Global"));
    graph.nodes.remove(index);
    assert!(summary(&graph).contains("Member unavailable in this view"));
    let before = cards(&graph).remove(0).content;
    graph.nodes.reverse();
    graph.components[0].members.reverse();
    assert_eq!(before, cards(&graph).remove(0).content);
    assert!(graph.components[0].entity.scope.is_none());
}

fn graph() -> Graph {
    crate::semantic::transform(
        &crate::plan::parse(include_str!("../../fixtures/components-plan.json")).unwrap(),
    )
    .0
}

fn number(node: roxmltree::Node, attribute: &str) -> usize {
    node.attribute(attribute).unwrap().parse().unwrap()
}

fn member_underline<'a, 'input>(
    member: roxmltree::Node<'a, 'input>,
    action: crate::model::Action,
) -> roxmltree::Node<'a, 'input> {
    let row = member.parent().unwrap();
    assert!(row.has_tag_name("g"));
    assert_eq!(row.attribute("data-component-member"), Some("true"));
    assert!(
        row.descendants()
            .all(|n| n.attribute("data-annotation-card").is_none())
    );
    assert!(
        row.descendants()
            .filter(|n| n.has_tag_name("rect"))
            .all(|n| {
                n.ancestors()
                    .any(|n| n.attribute("data-import-badge") == Some("true"))
            })
    );
    let lines: Vec<_> = row
        .descendants()
        .filter(|n| n.has_tag_name("line"))
        .collect();
    assert_eq!(lines.len(), 1);
    let underline = lines[0];
    assert_eq!(underline.attribute("data-member-underline"), Some("true"));
    assert_eq!(
        underline.attribute("stroke"),
        Some(crate::svg::color(action).1)
    );
    assert_eq!(underline.attribute("stroke-width"), Some("2"));
    assert_eq!(underline.attribute("x1"), Some("0"));
    assert_eq!(underline.attribute("y1"), underline.attribute("y2"));
    let text = member.children().find(|n| n.has_tag_name("text")).unwrap();
    assert_eq!(text.attribute("fill"), Some("#334155"));
    assert!(text.range().end <= underline.range().start);
    assert!(
        text.children()
            .filter(|n| n.has_tag_name("tspan"))
            .all(|n| number(n, "y") + 4 < number(underline, "y1"))
    );
    underline
}

#[test]
fn underlined_members_preserve_actions_unique_targets_and_state_identity() {
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
        member_underline(member, node.action);
        let badge = member
            .children()
            .find(|n| n.attribute("data-import-badge").is_some());
        assert_eq!(badge.is_some(), node.metadata.import.is_some());
        if let Some(badge) = badge {
            let badge_rect = badge.children().find(|n| n.has_tag_name("rect")).unwrap();
            let bottom = number(badge_rect, "y") + number(badge_rect, "height");
            let first = member
                .descendants()
                .find(|n| n.has_tag_name("tspan"))
                .unwrap();
            assert!(bottom + 10 < number(first, "y"));
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
    member_underline(missing, Action::Unchanged);
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
fn empty_components_and_ambiguous_member_addresses_do_not_invent_links() {
    let mut graph = graph();
    let duplicate = graph
        .nodes
        .iter()
        .find(|n| n.address == "aws_lb_listener.https")
        .unwrap()
        .clone();
    graph.nodes.push(duplicate);
    let output = cards(&graph).remove(0);
    let doc = roxmltree::Document::parse(&output.content).unwrap();
    let member = doc
        .descendants()
        .find(|n| n.attribute("data-source-address") == Some("aws_lb_listener.https"))
        .unwrap();
    assert!(member.attribute("href").is_none());
    assert_eq!(member.attribute("data-member-status"), Some("unavailable"));
    assert!(member.attribute("data-import").is_none());
    assert_eq!(
        member
            .descendants()
            .filter(|n| n.has_tag_name("tspan"))
            .filter_map(|n| n.text())
            .collect::<String>(),
        "aws_lb_listener.https"
    );
    member_underline(member, crate::model::Action::Unchanged);
    assert!(
        member
            .descendants()
            .all(|n| n.attribute("data-import-badge").is_none())
    );
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
    let mut template = graph.components[0].clone();
    let mut imported = graph
        .nodes
        .iter()
        .find(|n| n.metadata.import.is_some())
        .unwrap()
        .clone();
    imported.address = format!(
        r#"module.{}.aws_lb_listener.named["  支払い & \"blue\" <edge>  "]"#,
        "長いモジュール".repeat(24)
    );
    template.members.push(imported.address.clone());
    graph.nodes.push(imported);
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
        component.members.push(component.members[0].clone());
        graph.components.push(component);
    }
    let cards = cards(&graph);
    assert_eq!(cards.len(), 5);
    for card in &cards {
        assert!(card.width <= 640);
        let doc = roxmltree::Document::parse(&card.content).unwrap();
        let panel = doc.root_element();
        let component = graph
            .components
            .iter()
            .find(|c| panel.attribute("data-component-id") == Some(c.id.as_str()))
            .unwrap();
        let expected_members: std::collections::BTreeSet<_> =
            component.members.iter().map(String::as_str).collect();
        let members: Vec<_> = doc
            .descendants()
            .filter(|n| n.attribute("data-source-address").is_some())
            .collect();
        assert_eq!(
            members
                .iter()
                .map(|n| n.attribute("data-source-address").unwrap())
                .collect::<Vec<_>>(),
            expected_members.into_iter().collect::<Vec<_>>()
        );
        let headings: Vec<_> = panel
            .children()
            .filter(|n| n.has_tag_name("text") && n.attribute("data-component-scope").is_none())
            .collect();
        assert!(headings.len() > 1);
        assert_eq!(
            headings.iter().filter_map(|n| n.text()).collect::<String>(),
            component.label
        );
        let mut bottom = number(*headings.last().unwrap(), "y");
        for member in members {
            let address = member.attribute("data-source-address").unwrap();
            let node = graph.nodes.iter().find(|n| n.address == address);
            let underline = member_underline(
                member,
                node.map_or(crate::model::Action::Unchanged, |node| node.action),
            );
            let position: Vec<usize> = member
                .parent()
                .unwrap()
                .attribute("transform")
                .unwrap()
                .strip_prefix("translate(")
                .unwrap()
                .strip_suffix(')')
                .unwrap()
                .split_whitespace()
                .map(|n| n.parse().unwrap())
                .collect();
            assert_eq!(position.len(), 2);
            assert_eq!(position[0], 16);
            assert!(position[1] > bottom);
            assert_eq!(number(underline, "x2"), card.width - 32);
            bottom = position[1] + number(underline, "y1");
            assert!(bottom < card.height - 16);
            let lines: Vec<_> = member
                .descendants()
                .filter(|n| n.has_tag_name("tspan"))
                .collect();
            let visible: String = lines.iter().filter_map(|n| n.text()).collect();
            let expected = node.map_or_else(
                || address.to_string(),
                |node| {
                    format!(
                        "{}: {address}",
                        crate::svg::operations::label(node.action, &node.metadata)
                    )
                },
            );
            assert_eq!(visible, expected);
            if address.contains("長いモジュール") {
                assert!(lines.len() > 1);
            }
            for pair in lines.windows(2) {
                assert_eq!(number(pair[1], "y") - number(pair[0], "y"), 20);
            }
            for line in lines {
                assert_eq!(number(line, "x"), 0);
                assert_eq!(
                    line.attribute(("http://www.w3.org/XML/1998/namespace", "space")),
                    Some("preserve")
                );
                let width: usize = line
                    .text()
                    .unwrap()
                    .chars()
                    .map(|c| if c.is_ascii() { 8 } else { 16 })
                    .sum();
                assert!(position[0] + width <= card.width - 16);
            }
        }
    }
    let expected = crate::svg::annotations::Area::new(cards, 1500).render();
    graph.components.reverse();
    graph.nodes.reverse();
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
