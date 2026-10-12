use super::*;
#[path = "icon_import.rs"]
mod icon_import;

#[test]
fn symbols_are_deduplicated_and_used_locally_on_cards_and_containers() {
    let address = format!(
        r#"data.aws_instance.b["{}"]"#,
        "日本語.key  with spaces ".repeat(5)
    );
    let input = serde_json::json!({"format_version":"1.2","resource_changes":[
        {"address":"aws_vpc.main","type":"aws_vpc","change":{"actions":["create"],"importing":{}}},
        {"address":"aws_instance.a","type":"aws_instance"},
        {"address":address,"type":"aws_instance","mode":"data","change":{"actions":["no-op"],"importing":{"identity":{"secret":"PRIVATE_IMPORT"},"unknown":true}}},
        {"address":"other_widget.main","type":"other_widget"}
    ]});
    let raw = crate::plan::parse(&input.to_string()).unwrap();
    let graph = crate::semantic::transform(&raw);
    let layout = crate::layout::Layout::new(&graph);
    for format in [
        crate::svg::AddressFormat::Qualified,
        crate::svg::AddressFormat::Terraform,
    ] {
        let svg = crate::svg::render_with_format(&graph, &layout, format);
        assert_eq!(svg.matches("<symbol ").count(), 2);
        assert_eq!(svg.matches("id=\"planorama-icon-aws-ec2\"").count(), 1);
        assert_eq!(svg.matches("href=\"#planorama-icon-aws-ec2\"").count(), 2);
        assert_eq!(svg.matches("href=\"#planorama-icon-aws-vpc\"").count(), 1);
        assert_eq!(svg.matches("planorama-icons-license").count(), 1);
        assert!(svg.contains("AWS Architecture Icons, release 07312026"));
        assert!(!svg.contains("MIT License"));
        assert!(!svg.contains("<image "));
        assert!(!svg.contains("data:image/svg+xml,"));
        assert!(!svg.contains("PRIVATE_IMPORT"));
        assert_eq!(svg.matches("data-import-badge=\"true\"").count(), 2);
        assert!(svg.contains("viewBox=\"0 0 64 64\"><g"));
        let doc = roxmltree::Document::parse(&svg).unwrap();
        for (index, (node, position)) in graph.nodes.iter().zip(&layout.positions).enumerate() {
            let x = position.x;
            let y = position.y;
            let badge_height = if node.metadata.import.is_some() {
                24
            } else {
                0
            };
            let inset = match for_node(node) {
                Some(icon) => {
                    assert!(svg.contains(&render(icon, x + 12, y + 28 + badge_height)));
                    44
                }
                None => 12,
            };
            let selected = match format {
                crate::svg::AddressFormat::Qualified => node.resource_address().qualified(),
                crate::svg::AddressFormat::Terraform => node.address.as_str().into(),
            };
            let card = doc
                .descendants()
                .find(|n| n.attribute("data-terraform-address") == Some(node.address.as_str()))
                .unwrap();
            let lines: Vec<_> = card
                .children()
                .filter(|n| n.has_tag_name("text") && n.attribute("font-size") == Some("13"))
                .collect();
            assert_eq!(
                lines.iter().filter_map(|n| n.text()).collect::<String>(),
                selected.as_ref()
            );
            if node.address == address {
                assert!(lines.len() > 2);
                assert_eq!(card.attribute("data-import-id-present"), Some("false"));
                assert_eq!(card.attribute("data-import-identity-present"), Some("true"));
                assert_eq!(card.attribute("data-import-unknown"), Some("true"));
                assert!(
                    card.children()
                        .any(|n| n.attribute("data-external") == Some("true"))
                );
            }
            let badge = card
                .children()
                .find(|n| n.attribute("data-import-badge").is_some());
            assert_eq!(badge.is_some(), node.metadata.import.is_some());
            if let Some(badge) = badge {
                let rect = badge.children().find(|n| n.has_tag_name("rect")).unwrap();
                let number = |n: roxmltree::Node, key: &str| {
                    n.attribute(key).unwrap().parse::<usize>().unwrap()
                };
                let bottom = number(rect, "y") + number(rect, "height");
                assert!(number(rect, "x") >= x + 12);
                assert!(
                    number(rect, "x") + number(rect, "width") <= layout.bounds[index].right() - 12
                );
                assert!(bottom + 10 < number(lines[0], "y"));
                if let Some(icon) = card.children().find(|n| n.has_tag_name("use")) {
                    assert!(bottom < number(icon, "y"));
                }
                if let Some(external) = card
                    .children()
                    .find(|n| n.attribute("data-external").is_some())
                {
                    let external = external
                        .children()
                        .find(|n| n.has_tag_name("rect"))
                        .unwrap();
                    assert!(number(external, "y") + number(external, "height") < number(rect, "y"));
                    assert_ne!(external.attribute("fill"), rect.attribute("fill"));
                }
            }
            for (line, text) in lines.iter().enumerate() {
                let tx: usize = text.attribute("x").unwrap().parse().unwrap();
                let ty: usize = text.attribute("y").unwrap().parse().unwrap();
                let advance: usize = text
                    .text()
                    .unwrap()
                    .chars()
                    .map(|c| if c.is_ascii() { 8 } else { 16 })
                    .sum();
                assert_eq!(tx, x + inset);
                assert_eq!(ty, y + 45 + badge_height + line * 20);
                assert!(tx + advance <= layout.bounds[index].right() - 12);
                assert!(ty + 4 < y + layout.header_heights[index]);
                assert!(text.ancestors().any(|n| {
                    n.attribute(("http://www.w3.org/XML/1998/namespace", "space"))
                        == Some("preserve")
                }));
            }
        }
        assert_eq!(svg, crate::svg::render_with_format(&graph, &layout, format));
    }
}

#[test]
fn official_assets_are_inline_and_shared_by_service() {
    assert_eq!(ResourceIcon::Vpc.id(), ResourceIcon::Subnet.id());
    assert_eq!(ResourceIcon::Vpc.svg(), ResourceIcon::Subnet.svg());
    assert!(ResourceIcon::Ec2.svg().contains("Arch_Amazon-EC2_48"));
}

#[test]
fn unknown_only_diagrams_do_not_embed_unused_artwork() {
    let raw = crate::plan::parse(r#"{"format_version":"1.2","resource_changes":[{"address":"custom_resource.main","type":"custom_resource"}]}"#).unwrap();
    let graph = crate::semantic::transform(&raw);
    assert_eq!(definitions(&graph.nodes), "");
    let svg = crate::svg::render(&graph, &crate::layout::Layout::new(&graph));
    assert!(!svg.contains("<use"));
    assert!(!svg.contains("<symbol"));
}

#[test]
fn large_example_icons_are_independent_of_roles_and_entity_modes() {
    let raw =
        crate::plan::parse(include_str!("../../../examples/terraform-large/plan.json")).unwrap();
    let graph = crate::semantic::transform(&raw);
    for node in &graph.nodes {
        let expected = for_node(node);
        assert!(expected.is_some(), "{}", node.resource_type);
        let mut modified = node.clone();
        modified.mode = crate::model::EntityMode::Data;
        modified.role = crate::model::ResourceRole::Unknown;
        assert_eq!(expected, for_node(&modified));
    }
}
