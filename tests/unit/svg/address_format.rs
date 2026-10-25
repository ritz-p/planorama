use crate::{html, layout::Layout, plan, semantic, svg, view};
use std::collections::BTreeSet;

fn visible(node: roxmltree::Node<'_, '_>) -> String {
    node.descendants()
        .filter(|n| {
            n.has_tag_name("tspan")
                || (n.has_tag_name("text")
                    && !n.children().any(|child| child.has_tag_name("tspan")))
        })
        .filter_map(|n| n.text())
        .collect()
}

fn identities(doc: &roxmltree::Document<'_>) -> BTreeSet<(String, String)> {
    doc.descendants()
        .flat_map(|node| {
            node.attributes()
                .filter(|a| {
                    matches!(
                        a.name(),
                        "id" | "href"
                            | "data-source-address"
                            | "data-source-identity"
                            | "data-terraform-address"
                            | "data-qualified-address"
                    )
                })
                .map(|a| (a.name().to_owned(), a.value().to_owned()))
        })
        .collect()
}

#[test]
fn known_addresses_use_the_selected_format_without_changing_navigation() {
    for input in [
        include_str!("../../fixtures/components-plan.json"),
        include_str!("../../fixtures/routes-plan.json"),
        include_str!("../../fixtures/logical-actions-plan.json"),
        include_str!("../../fixtures/iam-plan.json"),
        include_str!("../../fixtures/efs-plan.json"),
        include_str!("../../fixtures/deposed-plan.json"),
    ] {
        let source = semantic::transform(&plan::parse(input).unwrap());
        let graph = view::single(&source, &Default::default()).unwrap();
        let layout = Layout::new(&graph);
        let mut previous = None;
        for format in [svg::AddressFormat::Terraform, svg::AddressFormat::Qualified] {
            let qualified = matches!(format, svg::AddressFormat::Qualified);
            let output = svg::render_with_format(&graph, &layout, format);
            assert!(
                html::render(&graph, &layout, format).contains(&svg::render_content(
                    &graph,
                    &layout,
                    format,
                    svg::Content::ValueFree
                ))
            );
            let doc = roxmltree::Document::parse(&output).unwrap();
            let current = identities(&doc);
            if let Some(previous) = &previous {
                assert_eq!(previous, &current);
            }
            previous = Some(current);
            for row in doc
                .descendants()
                .filter(|n| n.attribute("data-source-address").is_some())
            {
                if row.attribute("data-member-status").is_some() {
                    continue;
                }
                let id = crate::model::TerraformEntityId {
                    address: row.attribute("data-source-address").unwrap().into(),
                    deposed_key: None,
                };
                let expected = id.resource_address().formatted(qualified);
                if !qualified && id.resource_address().qualified() != id.address {
                    assert!(!visible(row).contains(id.resource_address().qualified().as_ref()));
                }
                assert!(
                    visible(row).contains(expected.as_ref()),
                    "{}: {}",
                    expected,
                    visible(row)
                );
            }
            for link in doc.descendants().filter(|n| {
                n.has_tag_name("a")
                    && n.ancestors()
                        .any(|p| p.attribute("data-resource-references").is_some())
            }) {
                let Some(target) = link.attribute("href").and_then(|href| {
                    graph
                        .nodes
                        .iter()
                        .find(|n| format!("#{}", svg::identity::resource(n)) == href)
                }) else {
                    continue;
                };
                if visible(link).contains("AWS Account") {
                    continue;
                }
                assert!(
                    visible(link).contains(target.resource_address().formatted(qualified).as_ref())
                );
            }
            for node in &graph.nodes {
                let Some(element) = doc
                    .descendants()
                    .find(|n| n.attribute("id") == Some(svg::identity::resource(node).as_str()))
                else {
                    continue;
                };
                assert!(
                    visible(element)
                        .contains(node.resource_address().formatted(qualified).as_ref())
                );
                if !qualified && node.module == "root" {
                    assert!(
                        !visible(element).contains(node.resource_address().qualified().as_ref())
                    );
                }
                if let Some(previous) = &node.previous_address {
                    assert!(visible(element).contains(&view::text::moved_from(previous)));
                }
            }
        }
    }
}

#[test]
fn unavailable_component_members_remain_verbatim_and_unlinked() {
    let mut graph = semantic::transform(
        &plan::parse(include_str!("../../fixtures/components-plan.json")).unwrap(),
    )
    .0;
    graph.components[0]
        .members
        .push("custom_resource.unavailable[0]".into());
    let output = svg::render(&graph, &Layout::new(&graph));
    let doc = roxmltree::Document::parse(&output).unwrap();
    let row = doc
        .descendants()
        .find(|n| n.attribute("data-member-status").is_some())
        .unwrap();
    assert!(visible(row).starts_with("custom_resource.unavailable[0]"));
    assert!(!visible(row).contains("module.root"));
    assert!(row.attribute("href").is_none());
}

#[test]
fn named_states_keep_equal_native_addresses_separate_in_both_formats() {
    use crate::model::architecture::{Architecture, StateId};
    let graph =
        semantic::transform(&plan::parse(include_str!("../../fixtures/routes-plan.json")).unwrap());
    let architecture = Architecture {
        states: [
            (StateId::new("one").unwrap(), graph.clone()),
            (StateId::new("two").unwrap(), graph),
        ]
        .into(),
        relationships: Vec::new(),
    };
    let viewed = view::apply(&architecture, &Default::default()).unwrap();
    let mut previous = None;
    for format in [svg::AddressFormat::Terraform, svg::AddressFormat::Qualified] {
        let output = svg::render_states(&viewed, format);
        assert!(
            html::render_states(&viewed, format).contains(&svg::render_states_content(
                &viewed,
                format,
                svg::Content::ValueFree
            ))
        );
        let doc = roxmltree::Document::parse(&output).unwrap();
        let ids: Vec<_> = doc
            .descendants()
            .filter_map(|n| n.attribute("id"))
            .collect();
        assert_eq!(ids.len(), ids.iter().collect::<BTreeSet<_>>().len());
        let routes: Vec<_> = doc
            .descendants()
            .filter(|n| n.attribute("data-terraform-address") == Some("aws_route.default"))
            .collect();
        assert_eq!(routes.len(), 2);
        assert_ne!(routes[0].attribute("id"), routes[1].attribute("id"));
        for route in routes {
            assert!(
                visible(route).contains(if matches!(format, svg::AddressFormat::Qualified) {
                    "module.root.aws_route.default"
                } else {
                    "aws_route.default"
                })
            );
        }
        let current = identities(&doc);
        if let Some(previous) = &previous {
            assert_eq!(previous, &current);
        }
        previous = Some(current);
    }
}
