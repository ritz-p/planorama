use crate::{html, layout::Layout, model, plan, semantic, svg, view};

fn number(node: roxmltree::Node<'_, '_>, key: &str) -> usize {
    node.attribute(key).unwrap().parse().unwrap()
}

#[test]
fn specific_labels_remove_duplicate_types_but_service_fallbacks_keep_them() {
    let input = serde_json::json!({"format_version":"1.2", "resource_changes":[
        {"address":"module.db.aws_db_instance.main", "type":"aws_db_instance", "change":{"actions":["create"]}}
    ]});
    let source = semantic::transform(&plan::parse(&input.to_string()).unwrap());
    let original = view::single(&source, &Default::default()).unwrap();
    for (resource_type, service, label, expected) in [
        (
            "aws_db_instance",
            Some("Amazon RDS"),
            Some("RDS Instance"),
            "create",
        ),
        (
            "aws_db_option_group",
            Some("Amazon RDS"),
            Some("RDS"),
            "aws_db_option_group · create",
        ),
        (
            "custom_unknown",
            Some("RDS Instance"),
            Some("RDS Instance"),
            "custom_unknown · create",
        ),
        ("aws_db_instance", None, None, "aws_db_instance · create"),
    ] {
        let mut graph = (*original).clone();
        let node = &mut graph.nodes[0];
        node.resource_type = resource_type.into();
        node.service_name = service;
        let layout = Layout::new(&graph);
        for output in [
            svg::render(&graph, &layout),
            svg::render_content(
                &graph,
                &layout,
                svg::Options::default(),
                svg::Content::ValueFree,
            ),
        ] {
            let svg = &output[output.find("<svg").unwrap()..output.rfind("</svg>").unwrap() + 6];
            let doc = roxmltree::Document::parse(svg).unwrap();
            let card = doc
                .descendants()
                .find(|n| n.attribute("data-terraform-address").is_some())
                .unwrap();
            assert_eq!(
                card.attribute("data-terraform-address"),
                Some("module.db.aws_db_instance.main")
            );
            let heading: String = card
                .children()
                .filter(|n| n.attribute("data-resource-type-heading").is_some())
                .filter_map(|n| n.text())
                .collect();
            assert_eq!(heading, expected);
            assert_eq!(
                card.children()
                    .find_map(|n| n.attribute("data-display-name")),
                label
            );
        }
    }
}

#[test]
fn spatial_type_headings_fit_with_labels_external_indicators_and_imports() {
    let mut input: serde_json::Value =
        serde_json::from_str(include_str!("../../fixtures/service-labels-plan.json")).unwrap();
    input["resource_changes"].as_array_mut().unwrap().extend([
        serde_json::json!({"address":"aws_elasticache_serverless_cache.long", "type":"aws_elasticache_serverless_cache", "change":{"actions":["forget","create"],"importing":{}}}),
        serde_json::json!({"address":"data.aws_elasticache_serverless_cache.external", "type":"aws_elasticache_serverless_cache", "mode":"data"}),
    ]);
    let source = semantic::transform(&plan::parse(&input.to_string()).unwrap());
    let original = view::single(&source, &Default::default()).unwrap();
    for long in [false, true] {
        let mut graph = (*original).clone();
        for (index, node) in graph.nodes.iter_mut().enumerate() {
            if long && (index % 2 == 0 || node.mode == model::EntityMode::Data) {
                node.resource_type
                    .push_str("_with_a_complete_long_terraform_resource_type_name");
                node.metadata.state_removal = Some(model::StateRemoval::ForgetThenCreate);
                node.metadata.import = Some(Default::default());
            }
        }
        let layout = Layout::new(&graph);
        for content in [svg::Content::Standard, svg::Content::ValueFree] {
            let output = svg::render_content(&graph, &layout, svg::Options::default(), content);
            let doc = roxmltree::Document::parse(&output).unwrap();
            for (index, node) in graph.nodes.iter().enumerate().filter(|(_, node)| {
                node.placement == model::Placement::Spatial
                    && view::routes::owner(&graph, node).is_none()
            }) {
                let card = doc
                    .descendants()
                    .find(|element| {
                        element.attribute("id") == Some(svg::resource_id(node).as_str())
                    })
                    .unwrap();
                let lines: Vec<_> = card
                    .children()
                    .filter(|element| {
                        element.attribute("data-resource-type-heading") == Some("true")
                    })
                    .collect();
                let operation = if node.metadata.state_removal.is_some() {
                    "forget then create".to_owned()
                } else {
                    format!("{:?}", node.action).to_lowercase()
                };
                assert_eq!(
                    lines
                        .iter()
                        .filter_map(|line| line.text())
                        .collect::<String>(),
                    if node.provider.is_aws()
                        && matches!(
                            node.resource_type.as_str(),
                            "aws_vpc"
                                | "aws_subnet"
                                | "aws_route_table"
                                | "aws_nat_gateway"
                                | "aws_iam_role_policy_attachment"
                        )
                    {
                        operation.clone()
                    } else {
                        format!("{} · {operation}", node.resource_type)
                    }
                );
                if node.resource_type.contains("_with_a_complete") {
                    assert!(lines.len() > 1);
                }
                let bounds = layout.bounds[index];
                let badge = card
                    .children()
                    .find(|element| element.attribute("data-service-badge").is_some());
                let external = card
                    .children()
                    .find(|element| element.attribute("data-external").is_some())
                    .and_then(|element| {
                        element.children().find(|child| child.has_tag_name("rect"))
                    });
                for line in &lines {
                    let right = number(*line, "x") + view::text::width(line.text().unwrap()) * 7;
                    assert!(number(*line, "x") >= bounds.origin.x + 12);
                    assert!(right <= bounds.right() - 12, "{}", node.address);
                    assert!(number(*line, "y") < bounds.origin.y + layout.header_heights[index]);
                    if let Some(badge) = badge {
                        assert!(right + 4 <= number(badge, "x"), "{}", node.address);
                        assert_eq!(
                            number(badge, "x") + number(badge, "width"),
                            bounds.right() - 8
                        );
                    }
                    if let Some(external) = external {
                        assert!(right + 12 <= number(external, "x"));
                    }
                }
                let last_y = number(*lines.last().unwrap(), "y");
                let address = card
                    .children()
                    .find(|element| {
                        element.has_tag_name("text") && element.attribute("font-size") == Some("13")
                    })
                    .unwrap();
                assert!(number(address, "y") >= last_y + 20);
                if node.metadata.import.is_some() {
                    let import = card
                        .children()
                        .find(|element| element.attribute("data-import-badge").is_some())
                        .unwrap();
                    let rect = import
                        .children()
                        .find(|element| element.has_tag_name("rect"))
                        .unwrap();
                    assert!(number(rect, "y") >= last_y + 8);
                    assert!(number(address, "y") > number(rect, "y") + number(rect, "height"));
                }
                if node.service_name.is_none() {
                    assert!(badge.is_none());
                }
            }
        }
    }
}

#[test]
fn foreign_types_remain_complete_in_spatial_and_logical_headings() {
    let resource_type =
        "foreign_resource_type_with_a_very_long_distinguishing_suffix_that_must_remain_visible";
    let input = serde_json::json!({"format_version":"1.2", "resource_changes":[
        {"address":format!("{resource_type}.managed"), "type":resource_type, "provider_name":"example.com/foreign/resource"},
        {"address":format!("data.{resource_type}.external"), "type":resource_type, "mode":"data", "provider_name":"example.com/foreign/resource"}
    ]});
    let source = semantic::transform(&plan::parse(&input.to_string()).unwrap());
    let original = view::single(&source, &Default::default()).unwrap();
    let layout = Layout::new(&original);
    assert!(
        html::render(&original, &layout, svg::AddressFormat::Qualified).contains(
            &svg::render_content(
                &original,
                &layout,
                svg::Options::default(),
                svg::Content::ValueFree
            )
        )
    );
    for placement in [model::Placement::Spatial, model::Placement::Logical] {
        let mut graph = (*original).clone();
        for node in &mut graph.nodes {
            node.placement = placement;
        }
        let output = svg::render(&graph, &Layout::new(&graph));
        let doc = roxmltree::Document::parse(&output).unwrap();
        assert!(
            !doc.descendants()
                .any(|element| element.attribute("data-service-badge").is_some())
        );
        let groups: Vec<_> = doc
            .descendants()
            .filter(|element| {
                if placement == model::Placement::Logical {
                    element.attribute("data-logical-group").is_some()
                } else {
                    element.attribute("data-terraform-address").is_some()
                }
            })
            .collect();
        assert_eq!(groups.len(), 2);
        for group in groups {
            let heading: String = group
                .children()
                .filter(|element| {
                    element.has_tag_name("text")
                        && (placement == model::Placement::Logical
                            || element.attribute("data-resource-type-heading").is_some())
                })
                .filter_map(|element| element.text())
                .collect();
            assert!(heading.starts_with(resource_type));
            assert!(!heading.contains('…'));
        }
    }
}
