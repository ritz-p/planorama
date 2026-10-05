use super::*;

#[test]
fn symbols_are_deduplicated_and_used_locally_on_cards_and_containers() {
    let raw = crate::plan::parse(
        r#"{"format_version":"1.2","resource_changes":[
        {"address":"aws_vpc.main","type":"aws_vpc"},
        {"address":"aws_instance.a","type":"aws_instance"},
        {"address":"data.aws_instance.b","type":"aws_instance","mode":"data"},
        {"address":"other_widget.main","type":"other_widget"}
    ]}"#,
    )
    .unwrap();
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
        assert_eq!(svg.matches("<image ").count(), 2);
        assert_eq!(svg.matches("href=\"data:image/svg+xml,").count(), 2);
        for (node, position) in graph.nodes.iter().zip(&layout.positions) {
            let x = position.x;
            let y = position.y;
            let (inset, width) = match for_node(node) {
                Some(icon) => {
                    assert!(svg.contains(&render(icon, x + 12, y + 28)));
                    (44, 33)
                }
                None => (12, 35),
            };
            let selected = match format {
                crate::svg::AddressFormat::Qualified => node.resource_address().qualified(),
                crate::svg::AddressFormat::Terraform => node.address.as_str().into(),
            };
            let first: String = selected.chars().take(width).collect();
            assert!(svg.contains(&format!(
                "<text x=\"{}\" y=\"{}\" font-size=\"13\" fill=\"#0f172a\">{first}</text>",
                x + inset,
                y + 45
            )));
        }
        assert_eq!(svg, crate::svg::render_with_format(&graph, &layout, format));
    }
}

#[test]
fn official_assets_are_encoded_losslessly_and_shared_by_service() {
    let sample = "<svg id=\"x\">#&%\n日本語</svg>";
    let encoded = data_uri(sample);
    let mut decoded = Vec::new();
    let mut bytes = encoded.strip_prefix("data:image/svg+xml,").unwrap().bytes();
    while let Some(byte) = bytes.next() {
        match byte {
            b'%' => {
                let hex = [bytes.next().unwrap(), bytes.next().unwrap()];
                decoded.push(u8::from_str_radix(std::str::from_utf8(&hex).unwrap(), 16).unwrap());
            }
            _ => decoded.push(byte),
        }
    }
    assert_eq!(decoded, sample.as_bytes());
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
