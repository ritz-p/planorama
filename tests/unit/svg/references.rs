use crate::{layout::Layout, plan, semantic, svg, view};
use serde_json::{Value, json};

const CONSUMER: &str = concat!(
    "aws_instance.consumer_with_a_very_long_name_that_exercises_more_than_two_address_lines_",
    "even_on_content_sized_cards_and_keeps_its_uses_dependencies_and_footer_clear_",
    "while_preserving_every_character"
);

fn fixture(nested: bool, regional: bool) -> (Value, Vec<String>) {
    let addresses = vec![
        format!(
            "module.{}[\"production\"].aws_iam_policy.shared[0]",
            "long_module_".repeat(30)
        ),
        "module.first.aws_iam_policy.shared".into(),
        "module.second.aws_iam_policy.shared".into(),
        "module.third[\"日本語.with  spaces&<>\"].aws_iam_policy.shared".into(),
        "aws_iam_policy.fourth".into(),
        "aws_iam_policy.fifth".into(),
    ];
    let mut resources: Vec<_> = addresses.iter().map(|address| json!({"address":address,"type":"aws_iam_policy","change":{"actions":["create"]}})).collect();
    resources.extend([
        json!({"address":CONSUMER,"type":"aws_instance","change":{"actions":["update"],"after":{"secret":"PRIVATE_VALUE"}}}),
        json!({"address":"aws_instance.peer","type":"aws_instance"}),
        json!({"address":"aws_iam_role.annotation","type":"aws_iam_role"}),
        json!({"address":"aws_iam_role.z_peer","type":"aws_iam_role"}),
    ]);
    let expressions = json!({
        "policies":{"references":addresses.iter().map(|a| format!("{a}.arn")).collect::<Vec<_>>()},
        "iam_instance_profile":{"references":["aws_iam_instance_profile.missing.name"]}
    });
    let mut config = vec![
        json!({"address":CONSUMER,"provider_config_key":"aws","depends_on":[addresses[1]],"expressions":expressions}),
        json!({"address":"aws_instance.peer","provider_config_key":"aws","expressions":{"user_data":{"references":[format!("{CONSUMER}.id")]}}}),
        json!({"address":"aws_iam_role.annotation","expressions":expressions}),
    ];
    if nested {
        resources.extend([
            json!({"address":"aws_vpc.main","type":"aws_vpc"}),
            json!({"address":"aws_subnet.main","type":"aws_subnet"}),
        ]);
        config.extend([
            json!({"address":"aws_vpc.main","provider_config_key":"aws"}),
            json!({"address":"aws_subnet.main","provider_config_key":"aws","expressions":{"vpc_id":{"references":["aws_vpc.main.id"]}}}),
        ]);
        for node in config.iter_mut().take(2) {
            node["expressions"]["subnet_id"] = json!({"references":["aws_subnet.main.id"]});
        }
    }
    let mut value = json!({"format_version":"1.2","resource_changes":resources,"configuration":{"root_module":{"resources":config}}});
    value["configuration"]["provider_config"]["aws"] =
        json!({"full_name":"registry.terraform.io/hashicorp/aws"});
    if regional {
        value["configuration"]["provider_config"]["aws"] = json!({"full_name":"registry.terraform.io/hashicorp/aws","expressions":{"region":{"constant_value":"us-east-1"}}});
    }
    (value, addresses)
}

fn visible_text(node: roxmltree::Node<'_, '_>) -> String {
    node.children()
        .filter(|n| n.has_tag_name("text"))
        .filter_map(|n| n.text())
        .collect()
}

#[test]
fn every_remote_reference_keeps_its_full_state_qualified_label_and_link() {
    use crate::model::architecture::{
        Architecture, CrossStateEdge, CrossStateProvenance, Endpoint, StateId,
    };
    let (value, addresses) = fixture(false, false);
    let graph = semantic::transform(&plan::parse(&value.to_string()).unwrap());
    let producer = StateId::new("producer").unwrap();
    let consumer = StateId::new("consumer").unwrap();
    let mut architecture = Architecture {
        states: [(producer.clone(), graph.clone()), (consumer.clone(), graph)].into(),
        relationships: addresses
            .iter()
            .enumerate()
            .map(|(i, address)| CrossStateEdge {
                from: Endpoint {
                    state: producer.clone(),
                    address: address.clone(),
                },
                to: Endpoint {
                    state: consumer.clone(),
                    address: CONSUMER.into(),
                },
                provenance: CrossStateProvenance::TerraformRemoteState {
                    remote: "data.terraform_remote_state.policies".into(),
                    output: format!("policy{i}"),
                    output_action: None,
                },
            })
            .collect(),
    };
    let render = |architecture: &Architecture| {
        svg::render_states(
            &view::apply(architecture, &Default::default()).unwrap(),
            Default::default(),
        )
    };
    let output = render(&architecture);
    let doc = roxmltree::Document::parse(&output).unwrap();
    for address in &addresses {
        let link = doc
            .descendants()
            .find(|n| {
                n.has_tag_name("a") && visible_text(*n) == format!("References: producer:{address}")
            })
            .unwrap();
        let href = link.attribute("href").unwrap().strip_prefix('#').unwrap();
        let target = doc
            .descendants()
            .find(|n| n.attribute("id") == Some(href))
            .unwrap();
        assert_eq!(
            target.attribute("data-terraform-address"),
            Some(address.as_str())
        );
        assert!(
            target
                .ancestors()
                .any(|n| n.attribute("data-state-id") == Some("producer"))
        );
    }
    architecture.relationships.reverse();
    assert_eq!(output, render(&architecture));
}

#[test]
fn full_reference_rows_fit_flat_nested_and_regional_geometry() {
    for (nested, regional) in [(false, false), (true, false), (true, true)] {
        let (mut value, addresses) = fixture(nested, regional);
        let graph = semantic::transform(&plan::parse(&value.to_string()).unwrap());
        let layout = Layout::new(&graph);
        let output = svg::render(&graph, &layout);
        assert!(!output.contains("PRIVATE_VALUE"));
        assert!(!output.contains("more (see full title)"));
        let doc = roxmltree::Document::parse(&output).unwrap();
        for address in [CONSUMER, "aws_iam_role.annotation"] {
            let card = doc
                .descendants()
                .find(|n| n.attribute("data-terraform-address") == Some(address))
                .unwrap();
            let references = card
                .descendants()
                .find(|n| n.attribute("data-resource-references").is_some())
                .unwrap();
            for target in &addresses {
                assert!(
                    references.children().any(|link| link.has_tag_name("a")
                        && visible_text(link) == format!("References: {target}")),
                    "missing {target}"
                );
            }
            assert!(references.descendants().any(|n| n.text()
                == Some(if address == CONSUMER {
                    "Instance profile: unresolved"
                } else {
                    "References: unresolved"
                })));
            let (rect, row_y) = if card.attribute("data-placement") == Some("logical") {
                let group = card.parent().unwrap();
                assert_eq!(group.attribute("data-logical-group"), Some("true"));
                let outer = group.parent().unwrap();
                assert_eq!(outer.attribute("data-annotation-card"), Some("true"));
                let row_y: usize = card
                    .attribute("transform")
                    .unwrap()
                    .strip_prefix("translate(0 ")
                    .unwrap()
                    .strip_suffix(')')
                    .unwrap()
                    .parse()
                    .unwrap();
                (
                    outer.children().find(|n| n.has_tag_name("rect")).unwrap(),
                    row_y,
                )
            } else {
                (card.children().find(|n| n.has_tag_name("rect")).unwrap(), 0)
            };
            let x: usize = rect.attribute("x").unwrap_or("0").parse().unwrap();
            let y: usize = rect.attribute("y").unwrap_or("0").parse().unwrap();
            let width: usize = rect.attribute("width").unwrap().parse().unwrap();
            let height: usize = rect.attribute("height").unwrap().parse().unwrap();
            assert!(width > crate::layout::NODE_WIDTH);
            for text in references.descendants().filter(|n| n.has_tag_name("text")) {
                let tx: usize = text.attribute("x").unwrap().parse().unwrap();
                let ty: usize = text.attribute("y").unwrap().parse().unwrap();
                let advance: usize = text
                    .text()
                    .unwrap()
                    .chars()
                    .map(|c| if c.is_ascii() { 8 } else { 16 })
                    .sum();
                assert!(tx + advance < x + width);
                assert!(row_y + ty + 4 < y + height);
                assert!(!text.text().unwrap().contains('…'));
            }
            if card.attribute("data-placement") == Some("logical") {
                let underline = card
                    .children()
                    .find(|n| n.attribute("data-resource-underline") == Some("true"))
                    .unwrap();
                let underline_y: usize = underline.attribute("y1").unwrap().parse().unwrap();
                assert!(
                    references
                        .descendants()
                        .filter(|n| n.has_tag_name("text"))
                        .all(|n| n.attribute("y").unwrap().parse::<usize>().unwrap() + 4
                            < underline_y)
                );
                let peer = card
                    .parent()
                    .unwrap()
                    .children()
                    .find(|n| n.attribute("data-terraform-address") == Some("aws_iam_role.z_peer"))
                    .unwrap();
                let peer_y: usize = peer
                    .attribute("transform")
                    .unwrap()
                    .strip_prefix("translate(0 ")
                    .unwrap()
                    .strip_suffix(')')
                    .unwrap()
                    .parse()
                    .unwrap();
                assert!(row_y + underline_y < peer_y);
            }
        }
        let consumer = doc
            .descendants()
            .find(|n| n.attribute("data-terraform-address") == Some(CONSUMER))
            .unwrap();
        let heading: Vec<_> = consumer
            .children()
            .filter(|n| n.has_tag_name("text") && n.attribute("font-size") == Some("13"))
            .collect();
        assert!(heading.len() > 2);
        assert_eq!(
            heading.iter().filter_map(|n| n.text()).collect::<String>(),
            consumer.attribute("data-qualified-address").unwrap()
        );
        let references = consumer
            .children()
            .find(|n| n.attribute("data-resource-references").is_some())
            .unwrap();
        let uses = references
            .children()
            .find(|n| n.has_tag_name("text"))
            .unwrap();
        let y = |n: roxmltree::Node<'_, '_>| n.attribute("y").unwrap().parse::<usize>().unwrap();
        assert!(y(*heading.last().unwrap()) + 10 < y(uses));
        let reference_bottom = references
            .descendants()
            .filter(|n| n.has_tag_name("text"))
            .map(y)
            .max()
            .unwrap();
        for footer in consumer
            .children()
            .filter(|n| n.has_tag_name("text") && n.attribute("font-size") == Some("10"))
        {
            assert!(reference_bottom + 4 < y(footer));
        }
        assert!(
            consumer
                .descendants()
                .any(|n| n.text() == Some("Uses / dependencies"))
        );
        assert!(
            consumer
                .descendants()
                .any(
                    |n| n.attribute("data-reference-attribute") == Some("depends_on")
                        && visible_text(n) == format!("Dependency: {}", addresses[1])
                )
        );
        for (index, bound) in layout
            .bounds
            .iter()
            .enumerate()
            .filter(|(_, b)| b.width > 0)
        {
            if let Some(parent) = layout.containment.parents[index] {
                let parent = layout.bounds[parent];
                assert!(bound.origin.x >= parent.origin.x && bound.right() <= parent.right());
                assert!(
                    bound.origin.y >= parent.origin.y
                        && bound.origin.y + bound.height <= parent.origin.y + parent.height
                );
            }
        }
        let mut routed = 0;
        for (edge, path) in graph
            .edges
            .iter()
            .zip(&layout.paths)
            .filter(|(_, p)| !p.is_empty())
        {
            routed += 1;
            for (node, point) in [(edge.from, path[0]), (edge.to, *path.last().unwrap())] {
                let b = layout.bounds[node];
                assert!(
                    (point.x == b.origin.x || point.x == b.right())
                        && (b.origin.y..=b.origin.y + b.height).contains(&point.y)
                        || (point.y == b.origin.y || point.y == b.origin.y + b.height)
                            && (b.origin.x..=b.right()).contains(&point.x)
                );
            }
        }
        assert!(routed > 0);
        let focused = view::single(
            &graph,
            &view::Options {
                focus: Some(CONSUMER.into()),
                depth: Some(0),
                ..Default::default()
            },
        )
        .unwrap();
        let filtered = svg::render(&focused, &Layout::new(&focused));
        let filtered_doc = roxmltree::Document::parse(&filtered).unwrap();
        for address in &addresses {
            assert!(
                filtered_doc
                    .descendants()
                    .any(
                        |n| n.attribute("data-reference-status") == Some("unavailable")
                            && visible_text(n) == format!("Unavailable — References: {address}")
                    )
            );
        }
        for href in filtered_doc
            .descendants()
            .filter_map(|n| n.attribute("href"))
        {
            assert!(
                filtered_doc
                    .descendants()
                    .any(|n| n.attribute("id") == href.strip_prefix('#'))
            );
        }
        value["resource_changes"].as_array_mut().unwrap().reverse();
        value["configuration"]["root_module"]["resources"]
            .as_array_mut()
            .unwrap()
            .reverse();
        let reordered = semantic::transform(&plan::parse(&value.to_string()).unwrap());
        assert_eq!(output, svg::render(&reordered, &Layout::new(&reordered)));
    }
}
