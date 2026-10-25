use crate::{icons, layout::Layout, plan, semantic, svg, view};

#[test]
fn resource_labels_distinguish_targets_and_reserve_right_aligned_header_space() {
    let raw = plan::parse(include_str!("../../fixtures/service-labels-plan.json")).unwrap();
    let source = semantic::transform(&raw);
    let graph = view::single(&source, &view::Options::default()).unwrap();
    let layout = Layout::new(&graph);
    let output = svg::render(&graph, &layout);
    let doc = roxmltree::Document::parse(&output).unwrap();
    for (address, expected, category) in [
        ("aws_vpc.main", "VPC", "network"),
        ("aws_subnet.application", "Subnet", "network"),
        ("data.aws_subnet.external", "Subnet", "network"),
        ("aws_route_table.application", "Route Table", "network"),
        ("aws_nat_gateway.egress", "NAT Gateway", "network"),
        ("aws_iam_user.operator", "IAM User", "iam"),
        ("aws_iam_role.worker", "IAM Role", "iam"),
        ("aws_iam_group.operators", "IAM Group", "iam"),
        ("aws_iam_policy.access", "IAM Policy", "iam"),
        (
            "aws_iam_role_policy_attachment.long",
            "IAM Role Policy Attachment",
            "iam",
        ),
    ] {
        let row = doc
            .descendants()
            .find(|n| n.attribute("data-terraform-address") == Some(address))
            .unwrap();
        let badge = row
            .children()
            .find(|n| n.attribute("data-service-badge").is_some())
            .unwrap();
        assert_eq!(badge.attribute("data-display-name"), Some(expected));
        assert_eq!(badge.attribute("data-service-category"), Some(category));
        let number =
            |n: roxmltree::Node, key: &str| n.attribute(key).unwrap().parse::<usize>().unwrap();
        assert!(number(badge, "height") >= 32);
        let index = graph
            .nodes
            .iter()
            .position(|n| n.address == address)
            .unwrap();
        if graph.nodes[index].placement == crate::model::Placement::Spatial {
            assert_eq!(
                number(badge, "x") + number(badge, "width"),
                layout.bounds[index].origin.x + layout.bounds[index].width - 8
            );
            let action = row
                .children()
                .find(|n| n.has_tag_name("text") && n.attribute("font-size") == Some("11"))
                .unwrap();
            assert_eq!(number(action, "y"), number(badge, "y") + 20);
            assert!(
                number(badge, "y") + number(badge, "height")
                    < layout.bounds[index].origin.y
                        + view::card::measure(&graph.nodes[index]).header_height
            );
        }
    }
    for address in ["aws_subnet.foreign", "aws_unclassified.example"] {
        let row = doc
            .descendants()
            .find(|n| n.attribute("data-terraform-address") == Some(address))
            .unwrap();
        assert!(
            !row.descendants()
                .any(|n| n.attribute("data-service-badge").is_some())
        );
    }
}

#[test]
fn provider_mapping_supplies_service_names_without_resource_prefix_inference() {
    for (kind, expected) in [
        ("aws_instance", "Amazon EC2"),
        ("aws_s3_bucket", "Amazon S3"),
        ("aws_lambda_function", "AWS Lambda"),
        ("aws_vpc", "Amazon VPC"),
        ("aws_subnet", "Amazon VPC"),
        ("aws_vpc_endpoint", "Amazon VPC"),
        ("aws_vpc_endpoint_route_table_association", "Amazon VPC"),
        ("aws_iam_role", "AWS Identity and Access Management"),
    ] {
        assert_eq!(
            icons::icon_for(icons::Provider::Aws, kind)
                .unwrap()
                .service_name(),
            expected
        );
        assert!(icons::icon_for(icons::Provider::Other, kind).is_none());
    }
    assert!(icons::icon_for(icons::Provider::Aws, "aws_vpc_unsupported").is_none());
    for kind in ["terraform_data", "terraform_remote_state"] {
        assert_eq!(
            icons::icon_for(icons::Provider::Terraform, kind)
                .unwrap()
                .service_name(),
            "Terraform built-in"
        );
    }
}

#[test]
fn service_labels_preserve_provider_boundaries_and_measured_geometry() {
    let address = format!("module.{}.aws_iam_role.long", "application".repeat(15));
    let input = serde_json::json!({"format_version":"1.2","resource_changes":[
        {"address":"aws_vpc.main","type":"aws_vpc"},
        {"address":address,"type":"aws_iam_role","change":{"actions":["create"],"importing":{}}},
        {"address":"data.aws_iam_role.external","type":"aws_iam_role","mode":"data"},
        {"address":"terraform_data.main","type":"terraform_data"},
        {"address":"aws_instance.foreign","type":"aws_instance","provider_name":"example.com/other/aws"},
        {"address":"aws_unknown.main","type":"aws_unknown"}
    ]});
    let raw = plan::parse(&input.to_string()).unwrap();
    let source = semantic::transform(&raw);
    let original = source.clone();
    let graph = view::single(
        &source,
        &view::Options {
            icon_mode: icons::IconMode::ServiceName,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(source, original);
    assert_eq!(graph.edges, source.edges);
    assert_eq!(graph.relationships, source.relationships);
    for (node, original) in graph.nodes.iter().zip(&source.nodes) {
        let mut restored = node.clone();
        restored.service_name = None;
        assert_eq!(&restored, original);
        assert_eq!(
            node.service_name,
            icons::for_node(original).map(|i| i.service_name())
        );
    }
    for format in [svg::AddressFormat::Qualified, svg::AddressFormat::Terraform] {
        let layout = Layout::new(&graph);
        let output = svg::render_with_format(&graph, &layout, format);
        let doc = roxmltree::Document::parse(&output).unwrap();
        assert!(!output.contains("planorama-icon"));
        for node in &graph.nodes {
            let row = doc
                .descendants()
                .find(|n| n.attribute("data-terraform-address") == Some(&node.address))
                .unwrap();
            let services: Vec<_> = row
                .children()
                .filter(|n| n.attribute("data-service-name") == Some("true"))
                .collect();
            assert_eq!(
                services.iter().filter_map(|n| n.text()).collect::<String>(),
                crate::services::display_name(node).unwrap_or_default()
            );
            if services.is_empty() {
                continue;
            }
            let badge = row
                .children()
                .find(|n| n.attribute("data-service-badge") == Some("true"))
                .unwrap();
            let number =
                |n: roxmltree::Node, key: &str| n.attribute(key).unwrap().parse::<usize>().unwrap();
            for text in &services {
                assert_eq!(text.attribute("font-weight"), Some("700"));
                assert_eq!(text.attribute("font-size"), Some("18"));
                assert!(number(*text, "x") >= number(badge, "x") + 10);
                assert!(
                    number(*text, "x") + crate::view::text::width(text.text().unwrap()) * 11
                        <= number(badge, "x") + number(badge, "width") - 10
                );
                assert!(number(*text, "y") > number(badge, "y"));
                assert!(number(*text, "y") < number(badge, "y") + number(badge, "height"));
            }
            let last_y: usize = services
                .last()
                .unwrap()
                .attribute("y")
                .unwrap()
                .parse()
                .unwrap();
            let address_text = row
                .children()
                .filter(|n| n.has_tag_name("text") && n.attribute("data-service-name").is_none())
                .find(|n| {
                    n.attribute("font-size") == Some("13")
                        || n.attribute("font-weight") == Some("700")
                })
                .unwrap();
            let address_y: usize = address_text.attribute("y").unwrap().parse().unwrap();
            assert!(address_y >= last_y + 20);
            if node.placement != crate::model::Placement::Logical {
                let metrics = view::card::measure(node);
                let index = graph.nodes.iter().position(|n| n == node).unwrap();
                assert!(last_y < layout.positions[index].y + metrics.header_height);
                assert!(metrics.service_height >= 16);
            }
        }
    }
}

#[test]
fn long_service_name_wraps_with_card_header_measurement() {
    let raw = plan::parse(r#"{"format_version":"1.2","resource_changes":[{"address":"aws_iam_role.main","type":"aws_iam_role"}]}"#).unwrap();
    let mut graph = semantic::transform(&raw);
    let node = &mut graph.0.nodes[0];
    node.resource_type = "aws_unclassified".into();
    let before = view::card::measure(node);
    node.service_name = Some(
        "A very long service name requiring several complete lines of text without truncation",
    );
    let after = view::card::measure(node);
    assert!(after.service_height >= 40);
    assert!(after.header_height >= before.header_height + 40);
    assert!(view::card::width(node) > crate::layout::NODE_WIDTH);
    let rendered = super::service_name(node, 12 + after.line_limit * 8, 45, after.line_limit * 8);
    let wrapped = format!("<g>{rendered}</g>");
    let doc = roxmltree::Document::parse(&wrapped).unwrap();
    assert_eq!(
        doc.descendants()
            .filter(|n| n.has_tag_name("text"))
            .filter_map(|n| n.text())
            .collect::<String>(),
        node.service_name.unwrap()
    );
}
