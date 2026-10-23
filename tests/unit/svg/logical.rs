use crate::{layout::Layout, model::Placement, plan, semantic, svg, view};

#[test]
fn action_sections_keep_one_type_card_canonical_colors_counts_and_stable_rows() {
    use crate::model::Action;
    let input = include_str!("../../fixtures/logical-actions-plan.json");
    let graph = semantic::transform(&plan::parse(input).unwrap());
    let render = |graph: &crate::model::Graph| svg::render(graph, &Layout::new(graph));
    let output = render(&graph);
    let doc = roxmltree::Document::parse(&output).unwrap();
    let groups: Vec<_> = doc
        .descendants()
        .filter(|n| n.attribute("data-logical-group").is_some())
        .collect();
    assert_eq!(groups.len(), 2);
    let group = groups
        .iter()
        .find(|n| n.attribute("data-resource-type") == Some("aws_iam_policy"))
        .unwrap();
    let sections: Vec<_> = group
        .children()
        .filter(|n| n.attribute("data-action-section").is_some())
        .collect();
    let expected = [
        (Action::Create, "Create", 2),
        (Action::Update, "Update", 1),
        (Action::Delete, "Delete", 1),
        (Action::Replace, "Replace", 1),
        (Action::Read, "Read", 1),
        (Action::Unchanged, "Unchanged", 1),
        (Action::Other, "Other", 1),
    ];
    assert_eq!(sections.len(), expected.len());
    let mut bottom = 0;
    for (section, (action, label, count)) in sections.iter().zip(expected) {
        assert_eq!(
            section.attribute("data-action-section"),
            Some(svg::label(action))
        );
        let rect = section.children().find(|n| n.has_tag_name("rect")).unwrap();
        assert_eq!(rect.attribute("stroke"), Some(svg::color(action).1));
        assert!(number(rect, "y") > bottom);
        bottom = number(rect, "y") + number(rect, "height");
        let heading = section.children().find(|n| n.has_tag_name("text")).unwrap();
        assert_eq!(heading.text(), Some(format!("{label} ({count})").as_str()));
        let rows: Vec<_> = section
            .children()
            .filter(|n| n.attribute("data-placement").is_some())
            .collect();
        assert_eq!(rows.len(), count);
        let ids: Vec<_> = rows
            .iter()
            .map(|n| n.attribute("data-architecture-id").unwrap())
            .collect();
        assert!(ids.windows(2).all(|pair| pair[0] < pair[1]));
        for row in rows {
            assert_logical_row(row, action);
            assert!(row_y(row) > number(heading, "y"));
            assert!(row_y(row) + number(underline(row), "y1") < bottom);
        }
        assert!(section.attribute("id").is_none());
        assert!(section.attribute("data-architecture-id").is_none());
    }
    assert_eq!(
        doc.descendants()
            .filter(|n| n.attribute("data-terraform-address").is_some())
            .count(),
        graph.nodes.len()
    );
    for group in groups {
        let explanation: String = group
            .children()
            .filter(|n| n.attribute("data-role-explanation").is_some())
            .filter_map(|n| n.text())
            .collect();
        assert!(explanation.contains(
            if group.attribute("data-resource-type") == Some("aws_iam_policy") {
                "Role: Policy — access/control rules"
            } else {
                "Role: Controller — manages or configures"
            }
        ));
    }
    let mut reversed: serde_json::Value = serde_json::from_str(input).unwrap();
    reversed["resource_changes"]
        .as_array_mut()
        .unwrap()
        .reverse();
    assert_eq!(
        output,
        render(&semantic::transform(
            &plan::parse(&reversed.to_string()).unwrap()
        ))
    );
    let selected = view::single(
        &graph,
        &view::Options {
            changed_only: true,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(!render(&selected).contains("data-action-section=\"unchanged\""));
    assert!(graph.nodes.iter().any(|n| n.action == Action::Unchanged));
}

fn input() -> serde_json::Value {
    serde_json::json!({"format_version":"1.2", "resource_changes":[
        {"address":"aws_acm_certificate_validation.app", "type":"aws_acm_certificate_validation", "previous_address":"aws_acm_certificate_validation.old", "change":{"actions":["update"],"importing":{"id":"PRIVATE_IMPORT"},"after":{"secret":"PRIVATE_VALUE"}}},
        {"address":"aws_ecs_task_definition.app", "type":"aws_ecs_task_definition"},
        {"address":"aws_ecs_service.app", "type":"aws_ecs_service"},
        {"address":"aws_iam_role.app", "type":"aws_iam_role"},
        {"address":"aws_iam_instance_profile.app", "type":"aws_iam_instance_profile"},
        {"address":"aws_instance.app", "type":"aws_instance"},
        {"address":"data.aws_ami.app", "type":"aws_ami", "mode":"data"},
        {"address":"custom_resource.app", "type":"custom_resource"}
    ],"configuration":{"root_module":{"resources":[
        {"address":"aws_ecs_service.app", "expressions":{"cluster":{"references":["aws_acm_certificate_validation.app.id"]},"task_definition":{"references":["aws_ecs_task_definition.app.arn"]}}},
        {"address":"aws_iam_instance_profile.app", "expressions":{"role":{"references":["aws_iam_role.app.name"]}}},
        {"address":"aws_instance.app", "depends_on":["aws_acm_certificate_validation.app"], "expressions":{"iam_instance_profile":{"references":["aws_iam_instance_profile.app.name"]},"ami":{"references":["data.aws_ami.app.id"]},"unknown":{"references":["aws_iam_role.missing.arn"]}}},
        {"address":"aws_ecs_task_definition.app", "expressions":{"configuration":{"references":["aws_instance.app.id"]}}},
        {"address":"aws_acm_certificate_validation.app", "expressions":{"name":{"constant_value":"example-cluster"}}}
    ]}}})
}

#[test]
fn scope_labels_preserve_rows_measurements_and_html_metadata() {
    let input = include_str!("../../fixtures/logical-regions-plan.json")
        .replace("aws_cloudwatch_log_group", "aws_ssm_parameter");
    let graph = semantic::transform(&plan::parse(&input).unwrap());
    let original = graph.clone();
    for options in [
        view::Options::default(),
        view::Options {
            changed_only: true,
            ..Default::default()
        },
        view::Options {
            focus: Some("aws_ssm_parameter.virginia".into()),
            depth: Some(0),
            ..Default::default()
        },
    ] {
        let selected = view::single(&graph, &options).unwrap();
        let layout = Layout::new(&selected);
        let svg = svg::render(&selected, &layout);
        let html = crate::html::render(&selected, &layout, svg::AddressFormat::Qualified);
        let doc = roxmltree::Document::parse(&svg).unwrap();
        for node in &selected.nodes {
            let row = doc
                .descendants()
                .find(|n| n.attribute("id") == Some(svg::identity::resource(node).as_str()))
                .unwrap();
            let expected = if node.deposed_key.is_some() {
                "Region: unknown"
            } else {
                match node.address.as_str() {
                    "aws_ssm_parameter.tokyo" => "Region: ap-northeast-1",
                    "aws_ssm_parameter.virginia" | "data.aws_ssm_parameter.lookup" => {
                        "Region: us-east-1"
                    }
                    "aws_iam_policy.access" => "AWS Global",
                    _ => "Region: unknown",
                }
            };
            assert_eq!(
                row.children()
                    .filter(|n| n.has_tag_name("text") && n.text() == Some(expected))
                    .count(),
                1
            );
            assert_logical_row(row, node.action);
            assert!(html.contains(expected));
            assert!(html.contains(&format!("id=\"{}\"", svg::identity::resource(node))));
            let next = row.next_sibling_element();
            if let Some(next) = next {
                assert!(row_y(row) + number(underline(row), "y1") < row_y(next));
            }
        }
        assert!(!svg.contains("PRIVATE_") && !html.contains("PRIVATE_"));
        assert!(!svg.contains("eu-west-3") && !html.contains("eu-west-3"));
        assert!(layout.paths.is_empty());
        if !options.changed_only && options.focus.is_none() {
            assert_eq!(super::group_count(&selected), 3);
            assert_eq!(selected.nodes.len(), 7);
        }
    }
    assert_eq!(graph, original);
}

#[test]
fn scope_presentation_distinguishes_inapplicable_unknown_and_generic_global() {
    use crate::model::{ProviderIdentity, architecture::DeploymentScope};
    use crate::view::scope::label;
    for provider in [
        ProviderIdentity::InferredAws,
        ProviderIdentity::Unknown,
        ProviderIdentity::from_source("unknown"),
        ProviderIdentity::from_source(""),
    ] {
        assert_eq!(label(None, &provider), "Region: unknown");
    }
    let foreign = ProviderIdentity::from_source("example.com/custom/aws");
    assert_eq!(label(None, &foreign), "Region: not applicable");
    assert_eq!(
        label(
            Some(&DeploymentScope {
                provider: "Example".into(),
                region: None,
                account: None,
                global: true,
            }),
            &foreign
        ),
        "Example Global"
    );
    let mut graph = semantic::transform(&plan::parse(&input().to_string()).unwrap());
    let node = &mut graph.0.nodes[0];
    node.placement = Placement::Logical;
    node.provider = foreign;
    node.entity.scope = None;
    let output = svg::render(&graph, &Layout::new(&graph));
    assert!(output.contains(">Region: not applicable</text>"));
}

fn number(node: roxmltree::Node, attribute: &str) -> usize {
    node.attribute(attribute).unwrap().parse().unwrap()
}

fn row_y(row: roxmltree::Node) -> usize {
    row.attribute("transform")
        .unwrap()
        .strip_prefix("translate(0 ")
        .unwrap()
        .strip_suffix(')')
        .unwrap()
        .parse()
        .unwrap()
}

fn underline<'a, 'input>(row: roxmltree::Node<'a, 'input>) -> roxmltree::Node<'a, 'input> {
    let lines: Vec<_> = row.children().filter(|n| n.has_tag_name("line")).collect();
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].attribute("data-resource-underline"), Some("true"));
    lines[0]
}

fn assert_logical_row(row: roxmltree::Node, action: crate::model::Action) {
    let section = row.parent().unwrap();
    assert_eq!(
        section.attribute("data-action-section"),
        Some(svg::label(action))
    );
    let group = section.parent().unwrap();
    assert_eq!(group.attribute("data-logical-group"), Some("true"));
    let card = group.parent().unwrap();
    assert_eq!(card.attribute("data-annotation-card"), Some("true"));
    let rect = card.children().find(|n| n.has_tag_name("rect")).unwrap();
    assert_eq!(rect.attribute("fill"), Some("#f8fafc"));
    assert_eq!(rect.attribute("stroke"), Some("#cbd5e1"));
    assert!(row.children().all(|n| !n.has_tag_name("rect")));
    assert!(
        row.descendants()
            .all(|n| n.attribute("data-annotation-card").is_none())
    );
    let line = underline(row);
    assert_eq!(line.attribute("stroke"), Some(svg::color(action).1));
    assert_eq!(line.attribute("stroke-width"), Some("2"));
    assert_eq!(number(line, "x1"), 16);
    assert_eq!(number(line, "x2") + 16, number(rect, "width"));
    assert_eq!(line.attribute("y1"), line.attribute("y2"));
    assert!(row_y(row) + number(line, "y1") < number(rect, "height"));
    for text in row.descendants().filter(|n| n.has_tag_name("text")) {
        assert!(text.range().end <= line.range().start);
        assert!(number(text, "y") + 4 < number(line, "y1"));
    }
    for text in row
        .children()
        .filter(|n| n.has_tag_name("text") && n.attribute("data-service-name").is_none())
    {
        assert_eq!(text.attribute("fill"), Some("#334155"));
        assert_eq!(number(text, "x"), 16);
        let width: usize = text
            .text()
            .unwrap()
            .chars()
            .map(|c| if c.is_ascii() { 8 } else { 16 })
            .sum();
        assert!(width + 32 <= number(rect, "width"));
        assert_eq!(
            text.attribute(("http://www.w3.org/XML/1998/namespace", "space")),
            Some("preserve")
        );
    }
}

#[test]
fn logical_projection_retains_graph_identity_references_and_operations() {
    let raw = plan::parse(&input().to_string()).unwrap();
    let graph = semantic::transform(&raw);
    let original = graph.clone();
    let layout = Layout::new(&graph);
    assert_eq!(graph.nodes.len(), 8);
    assert!(
        graph
            .nodes
            .iter()
            .find(|n| n.address == "aws_acm_certificate_validation.app")
            .unwrap()
            .references
            .is_empty()
    );
    assert_eq!(
        graph
            .nodes
            .iter()
            .filter(|n| n.placement == Placement::Logical)
            .count(),
        5
    );
    assert!(layout.paths.iter().all(Vec::is_empty));
    assert_eq!(graph.edges.len(), 7);
    for (i, node) in graph.nodes.iter().enumerate() {
        assert_eq!(
            layout.bounds[i].width == 0,
            node.placement == Placement::Logical
        );
    }
    let worker = graph
        .nodes
        .iter()
        .find(|n| n.address == "aws_instance.app")
        .unwrap();
    assert!(
        worker
            .references
            .iter()
            .any(|r| r.dependency && r.attribute == "depends_on")
    );
    assert!(
        worker
            .references
            .iter()
            .any(|r| !r.complete && r.attribute == "unknown")
    );
    assert!(
        !worker
            .references
            .iter()
            .flat_map(|r| &r.targets)
            .any(|t| t.id.address == "aws_iam_role.app")
    );
    let output = svg::render(&graph, &layout);
    let doc = roxmltree::Document::parse(&output).unwrap();
    assert_eq!(output.matches(">Logical Resources</text>").count(), 1);
    assert!(!output.contains(">Logical resource</text>"));
    for card in doc
        .descendants()
        .filter(|n| n.attribute("data-placement") == Some("logical"))
    {
        let node = graph
            .nodes
            .iter()
            .find(|n| card.attribute("data-terraform-address") == Some(n.address.as_str()))
            .unwrap();
        assert_logical_row(card, node.action);
        let heading = card.children().find(|n| n.has_tag_name("text")).unwrap();
        assert!(
            card.attribute("data-qualified-address")
                .unwrap()
                .starts_with(heading.text().unwrap())
        );
    }
    assert_eq!(
        doc.descendants()
            .filter(|n| n.attribute("data-placement") == Some("logical"))
            .count(),
        5
    );
    assert_eq!(
        doc.descendants()
            .filter(|n| n.attribute("data-logical-group") == Some("true"))
            .count(),
        5
    );
    assert_eq!(
        doc.descendants()
            .filter(|n| n.attribute("data-terraform-address").is_some())
            .count(),
        8
    );
    let cluster = doc
        .descendants()
        .find(|n| {
            n.attribute("data-terraform-address") == Some("aws_acm_certificate_validation.app")
        })
        .unwrap();
    assert_eq!(
        cluster.attribute("data-previous-address"),
        Some("aws_acm_certificate_validation.old")
    );
    assert_eq!(cluster.attribute("data-import"), Some("true"));
    let badges: Vec<_> = cluster
        .children()
        .filter(|n| n.attribute("data-import-badge") == Some("true"))
        .collect();
    assert_eq!(badges.len(), 1);
    let badge_rect = badges[0]
        .children()
        .find(|n| n.has_tag_name("rect"))
        .unwrap();
    let badge_bottom = badge_rect.attribute("y").unwrap().parse::<usize>().unwrap()
        + badge_rect
            .attribute("height")
            .unwrap()
            .parse::<usize>()
            .unwrap();
    let heading = cluster.children().find(|n| n.has_tag_name("text")).unwrap();
    assert!(badge_bottom + 10 < heading.attribute("y").unwrap().parse::<usize>().unwrap());
    assert!(
        cluster
            .children()
            .any(|n| n.has_tag_name("text") && n.text() == Some("update"))
    );
    for card in doc
        .descendants()
        .filter(|n| n.attribute("data-placement") == Some("logical"))
    {
        assert_eq!(
            card.children()
                .any(|n| n.attribute("data-import-badge").is_some()),
            card.attribute("data-import") == Some("true")
        );
    }
    assert_eq!(cluster.attribute("data-role"), Some("controller"));
    assert!(cluster.children().any(|n| {
        n.has_tag_name("title")
            && n.text()
                .unwrap()
                .starts_with("module.root.aws_acm_certificate_validation.app")
    }));
    assert!(
        cluster
            .children()
            .any(|n| n.has_tag_name("text") && n.text() == Some("controller"))
    );
    let terraform = svg::render_with_format(&graph, &layout, svg::AddressFormat::Terraform);
    let terraform_doc = roxmltree::Document::parse(&terraform).unwrap();
    let role = terraform_doc
        .descendants()
        .find(|n| n.attribute("data-terraform-address") == Some("aws_iam_role.app"))
        .unwrap();
    assert_eq!(role.attribute("data-role"), Some("identity"));
    assert!(
        role.children()
            .any(|n| n.has_tag_name("title") && n.text().unwrap().starts_with("aws_iam_role.app"))
    );
    assert!(!output.contains("PRIVATE_"));
    assert!(output.contains("3 spatial cards, 5 logical resources in 5 group cards"));
    assert_eq!(graph, original);
    assert_eq!(output, svg::render(&graph, &Layout::new(&graph)));
}

#[test]
fn logical_classification_is_explicit_and_provider_aware() {
    for (kind, expected) in [
        ("aws_iam_policy", Placement::Logical),
        ("aws_iam_role_policy", Placement::Logical),
        ("aws_s3_bucket", Placement::Spatial),
        ("aws_dynamodb_table", Placement::Spatial),
        ("aws_cloudfront_distribution", Placement::Spatial),
        ("aws_lb", Placement::Spatial),
        ("aws_ecs_service", Placement::Spatial),
        ("aws_ecs_cluster", Placement::Spatial),
        ("aws_ecs_task_definition", Placement::Logical),
        ("aws_future_resource", Placement::Unresolved),
        ("aws_cloudwatch_log_group", Placement::Spatial),
        ("aws_cloudwatch_metric_alarm", Placement::Logical),
        ("aws_cloudwatch_dashboard", Placement::Logical),
        ("aws_lb_listener_rule", Placement::Spatial),
    ] {
        assert_eq!(
            crate::provider::placement(&crate::model::ProviderIdentity::inferred(kind), kind),
            expected
        );
    }
    let mut value = input();
    value["resource_changes"][0]["provider_name"] = serde_json::json!("example.com/custom/aws");
    let graph = semantic::transform(&plan::parse(&value.to_string()).unwrap());
    assert_eq!(
        graph
            .nodes
            .iter()
            .find(|n| n.address == "aws_acm_certificate_validation.app")
            .unwrap()
            .placement,
        Placement::Unresolved
    );
}

#[test]
fn logical_groups_use_canonical_provider_type_and_mode_without_combining_actions() {
    use crate::model::ProviderIdentity;
    use serde_json::json;
    use std::collections::BTreeSet;

    let value = json!({"format_version":"1.2", "resource_changes":[
        {"address":"aws_iam_role.inferred", "type":"aws_iam_role", "change":{"actions":["create"]}},
        {"address":"aws_iam_role.explicit", "type":"aws_iam_role", "provider_name":" HASHICORP/AWS ", "change":{"actions":["update"]}},
        {"address":"module.other.aws_iam_role.aliased", "type":"aws_iam_role", "change":{"actions":["delete"]}},
        {"address":"data.aws_iam_role.shared", "type":"aws_iam_role", "mode":"data", "change":{"actions":["read"]}},
        {"address":"aws_iam_policy.other", "type":"aws_iam_policy"},
        {"address":"aws_iam_role.foreign", "type":"aws_iam_role", "provider_name":"example.com/acme/aws", "change":{"actions":["delete","create"]}},
        {"address":"aws_iam_role.unknown", "type":"aws_iam_role"}
    ], "configuration":{
        "provider_config":{
            "aws.primary":{"full_name":"hashicorp/aws", "alias":"primary"},
            "aws.secondary":{"full_name":"REGISTRY.TERRAFORM.IO/HASHICORP/AWS", "alias":"secondary"}
        },
        "root_module":{
            "resources":[{"address":"aws_iam_role.explicit", "provider_config_key":"aws.primary"}],
            "module_calls":{"other":{"module":{"resources":[{"address":"aws_iam_role.aliased", "provider_config_key":"aws.secondary"}]}}}
        }
    }});
    let mut graph = semantic::transform(&plan::parse(&value.to_string()).unwrap());
    for node in &mut graph.0.nodes {
        node.placement = Placement::Logical;
        if node.address == "aws_iam_role.unknown" {
            node.provider = ProviderIdentity::Unknown;
        }
    }
    assert!(
        graph
            .nodes
            .iter()
            .any(|n| n.provider == ProviderIdentity::InferredAws)
    );
    for (address, alias) in [
        ("aws_iam_role.explicit", "primary"),
        ("module.other.aws_iam_role.aliased", "secondary"),
    ] {
        let node = graph.nodes.iter().find(|n| n.address == address).unwrap();
        assert_eq!(
            node.provider,
            ProviderIdentity::from_source("hashicorp/aws")
        );
        assert_eq!(
            node.provider_configuration
                .as_ref()
                .unwrap()
                .alias
                .as_deref(),
            Some(alias)
        );
    }
    let output = svg::render(&graph, &Layout::new(&graph));
    let doc = roxmltree::Document::parse(&output).unwrap();
    let groups: Vec<_> = doc
        .descendants()
        .filter(|n| n.attribute("data-logical-group") == Some("true"))
        .collect();
    assert_eq!(groups.len(), 5);
    for (source, resource_type, mode, addresses) in [
        (
            "registry.terraform.io/hashicorp/aws",
            "aws_iam_role",
            "managed",
            vec![
                "aws_iam_role.inferred",
                "aws_iam_role.explicit",
                "module.other.aws_iam_role.aliased",
            ],
        ),
        (
            "registry.terraform.io/hashicorp/aws",
            "aws_iam_role",
            "data",
            vec!["data.aws_iam_role.shared"],
        ),
        (
            "registry.terraform.io/hashicorp/aws",
            "aws_iam_policy",
            "managed",
            vec!["aws_iam_policy.other"],
        ),
        (
            "example.com/acme/aws",
            "aws_iam_role",
            "managed",
            vec!["aws_iam_role.foreign"],
        ),
        (
            "unknown",
            "aws_iam_role",
            "managed",
            vec!["aws_iam_role.unknown"],
        ),
    ] {
        let group = groups
            .iter()
            .find(|n| {
                n.attribute("data-provider-source") == Some(source)
                    && n.attribute("data-resource-type") == Some(resource_type)
                    && n.attribute("data-mode") == Some(mode)
            })
            .unwrap();
        let heading: String = group
            .children()
            .filter(|n| n.has_tag_name("text"))
            .filter_map(|n| n.text())
            .collect();
        assert!(heading.contains(if source == "unknown" {
            "Provider unavailable"
        } else {
            source
        }));
        assert!(heading.contains(resource_type));
        assert!(heading.contains(mode));
        let rows: Vec<_> = group
            .descendants()
            .filter(|n| n.attribute("data-placement") == Some("logical"))
            .collect();
        assert_eq!(
            rows.iter()
                .map(|n| n.attribute("data-terraform-address").unwrap())
                .collect::<BTreeSet<_>>(),
            addresses.into_iter().collect::<BTreeSet<_>>()
        );
        let mut bottom = 0;
        for row in rows {
            let node = graph
                .nodes
                .iter()
                .find(|n| row.attribute("data-terraform-address") == Some(n.address.as_str()))
                .unwrap();
            assert_eq!(
                row.attribute("id"),
                Some(svg::identity::resource(node).as_str())
            );
            assert_eq!(row.attribute("data-mode"), Some(mode));
            assert_eq!(
                row.attribute("data-external"),
                (mode == "data").then_some("true")
            );
            assert_logical_row(row, node.action);
            assert_eq!(
                underline(row).attribute("stroke-dasharray"),
                (mode == "data").then_some("6 4")
            );
            assert!(row_y(row) > bottom);
            bottom = row_y(row) + number(underline(row), "y1");
            assert!(row.children().any(|n| n.has_tag_name("text")
                && n.text() == Some(svg::operations::action_label(node.action, &node.metadata))));
        }
    }
    assert_eq!(output.matches("data-placement=\"logical\"").count(), 7);
    assert!(output.contains("0 spatial cards, 7 logical resources in 5 group cards"));
    for (action, count) in [
        ("create", 1),
        ("update", 1),
        ("delete", 1),
        ("replace", 1),
        ("read", 1),
        ("unchanged", 0),
        ("other", 2),
    ] {
        assert!(output.contains(&format!(">{action} ({count})</text>")));
    }
    let grouped = |graph: &crate::model::Graph| {
        super::cards(graph, svg::AddressFormat::Qualified)
            .into_iter()
            .map(|card| (card.width, card.height, card.content))
            .collect::<Vec<_>>()
    };
    let expected = grouped(&graph);
    graph.0.nodes.reverse();
    assert_eq!(expected, grouped(&graph));
}

#[test]
fn logical_focus_and_changes_preserve_selected_resources_and_reference_evidence() {
    let graph = semantic::transform(&plan::parse(&input().to_string()).unwrap());
    for (focus, count) in [
        ("aws_instance.app", 0),
        ("aws_acm_certificate_validation.app", 1),
    ] {
        let selected = view::single(
            &graph,
            &view::Options {
                focus: Some(focus.into()),
                depth: Some(0),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(selected.nodes.len(), 1);
        let output = svg::render(&selected, &Layout::new(&selected));
        assert_eq!(output.matches("data-placement=\"logical\"").count(), count);
        assert_eq!(output.matches("data-logical-group=\"true\"").count(), count);
        if count == 0 {
            assert!(output.contains("aws_iam_instance_profile.app"));
        }
    }
    let changed = view::single(
        &graph,
        &view::Options {
            changed_only: true,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(
        changed
            .nodes
            .iter()
            .any(|n| n.address == "aws_acm_certificate_validation.app")
    );
    let output = svg::render(&changed, &Layout::new(&changed));
    let doc = roxmltree::Document::parse(&output).unwrap();
    let selected: std::collections::BTreeSet<_> = changed
        .nodes
        .iter()
        .filter(|n| n.placement == Placement::Logical)
        .map(|n| n.address.as_str())
        .collect();
    assert_eq!(
        doc.descendants()
            .filter(|n| n.attribute("data-placement") == Some("logical"))
            .map(|n| n.attribute("data-terraform-address").unwrap())
            .collect::<std::collections::BTreeSet<_>>(),
        selected
    );
    assert!(
        doc.descendants()
            .filter(|n| n.attribute("data-logical-group") == Some("true"))
            .all(|n| n
                .descendants()
                .any(|n| n.attribute("data-placement") == Some("logical")))
    );
}

#[test]
fn logical_group_rows_wrap_long_addresses_and_keep_deposed_identity_and_input_order() {
    let mut value = input();
    let address = format!("aws_iam_policy.{}", "long_name".repeat(30));
    let previous = format!(
        r#"module.{}.aws_iam_policy.old["  支払い & \"blue\" <edge>  "]"#,
        "長いモジュール".repeat(24)
    );
    value["resource_changes"].as_array_mut().unwrap().extend([
        serde_json::json!({"address":address, "type":"aws_iam_policy", "previous_address":previous}),
        serde_json::json!({"address":address, "type":"aws_iam_policy", "deposed":"old", "change":{"actions":["delete"]}}),
    ]);
    let render = |value: &serde_json::Value| {
        let graph = semantic::transform(&plan::parse(&value.to_string()).unwrap());
        svg::render(&graph, &Layout::new(&graph))
    };
    let output = render(&value);
    let doc = roxmltree::Document::parse(&output).unwrap();
    let cards: Vec<_> = doc
        .descendants()
        .filter(|n| n.attribute("data-terraform-address") == Some(address.as_str()))
        .collect();
    assert_eq!(cards.len(), 2);
    assert_ne!(cards[0].attribute("id"), cards[1].attribute("id"));
    assert_eq!(
        cards[0].parent().unwrap().parent(),
        cards[1].parent().unwrap().parent()
    );
    assert_ne!(cards[0].parent(), cards[1].parent());
    assert!(
        cards
            .iter()
            .any(|n| n.attribute("data-deposed-key") == Some("old"))
    );
    let mut bottom = 0;
    for card in cards {
        assert_logical_row(
            card,
            if card.attribute("data-deposed-key").is_some() {
                crate::model::Action::Delete
            } else {
                crate::model::Action::Other
            },
        );
        assert!(row_y(card) > bottom);
        bottom = row_y(card) + number(underline(card), "y1");
        let text: String = card
            .children()
            .filter(|n| n.has_tag_name("text"))
            .filter_map(|n| n.text())
            .collect();
        assert!(text.starts_with(card.attribute("data-qualified-address").unwrap()));
        if card.attribute("data-deposed-key").is_none() {
            assert!(text.contains(&format!("Moved from: {previous}")));
        }
        assert!(
            card.descendants()
                .filter(|n| n.has_tag_name("text"))
                .count()
                > 5
        );
    }
    value["resource_changes"].as_array_mut().unwrap().reverse();
    value["configuration"]["root_module"]["resources"]
        .as_array_mut()
        .unwrap()
        .reverse();
    assert_eq!(output, render(&value));
}

#[test]
fn named_states_keep_logical_ids_separate_and_cross_state_provenance_without_routes() {
    use crate::model::architecture::{
        Architecture, CrossStateEdge, CrossStateProvenance, Endpoint, StateId,
    };
    let first = StateId::new("first").unwrap();
    let second = StateId::new("second").unwrap();
    let graph = semantic::transform(&plan::parse(&input().to_string()).unwrap());
    let all = Architecture {
        states: std::collections::BTreeMap::from([
            (first.clone(), graph.clone()),
            (second.clone(), graph),
        ]),
        relationships: vec![CrossStateEdge {
            from: Endpoint {
                state: first,
                address: "aws_acm_certificate_validation.app".into(),
            },
            to: Endpoint {
                state: second,
                address: "aws_ecs_service.app".into(),
            },
            provenance: CrossStateProvenance::TerraformRemoteState {
                remote: "data.terraform_remote_state.app".into(),
                output: "cluster".into(),
                output_action: None,
            },
        }],
    };
    let view = view::apply(&all, &Default::default()).unwrap();
    let output = svg::render_states(&view, crate::svg::AddressFormat::default());
    let doc = roxmltree::Document::parse(&output).unwrap();
    for state in ["first", "second"] {
        let groups: Vec<_> = doc
            .descendants()
            .filter(|n| {
                n.attribute("data-logical-group") == Some("true")
                    && n.ancestors()
                        .any(|n| n.attribute("data-state-id") == Some(state))
            })
            .collect();
        assert_eq!(groups.len(), 5);
        assert!(groups.iter().all(|n| {
            n.descendants()
                .filter(|n| n.attribute("data-placement") == Some("logical"))
                .count()
                == 1
        }));
    }
    let ids: Vec<_> = doc
        .descendants()
        .filter(|n| {
            n.attribute("data-terraform-address") == Some("aws_acm_certificate_validation.app")
        })
        .map(|n| n.attribute("id").unwrap())
        .collect();
    assert_eq!(ids.len(), 2);
    assert_ne!(ids[0], ids[1]);
    for card in doc.descendants().filter(|n| {
        n.attribute("data-terraform-address") == Some("aws_acm_certificate_validation.app")
    }) {
        assert_eq!(
            card.attribute("data-previous-address"),
            Some("aws_acm_certificate_validation.old")
        );
        assert!(card.children().any(|n| n.has_tag_name("text")
            && n.text() == Some("Moved from: aws_acm_certificate_validation.old")));
        assert!(
            card.children()
                .find(|n| n.has_tag_name("title"))
                .unwrap()
                .text()
                .unwrap()
                .contains("moved from: aws_acm_certificate_validation.old")
        );
    }
    assert!(output.contains("cross-state-relationships"));
    assert!(!output.contains("independent states"));
    assert!(output.contains("1 logical"));
    let references: Vec<serde_json::Value> = doc
        .descendants()
        .filter(|n| {
            n.has_tag_name("metadata")
                && n.attribute("id")
                    .is_some_and(|id| id.ends_with("resource-references"))
        })
        .flat_map(|n| serde_json::from_str::<Vec<serde_json::Value>>(n.text().unwrap()).unwrap())
        .collect();
    let remote_metadata = references
        .iter()
        .find(|r| r["attribute"] == "data.terraform_remote_state.app.outputs.cluster")
        .unwrap();
    assert_eq!(remote_metadata["targets"][0]["state"], "first");
    assert_eq!(
        remote_metadata["targets"][0]["address"],
        "aws_acm_certificate_validation.app"
    );
    assert!(
        references
            .iter()
            .filter(|r| r["attribute"] == "cluster")
            .all(|r| r["targets"][0].get("state").is_none())
    );
    assert!(
        !doc.descendants()
            .any(|n| n.has_tag_name("path") && n.attribute("data-cross-state").is_some())
    );
    assert_eq!(all.relationships, view.relationships());
    for link in doc.descendants().filter_map(|n| n.attribute("href")) {
        let id = link.strip_prefix('#').unwrap();
        assert!(
            doc.descendants().any(|n| n.attribute("id") == Some(id)),
            "broken link {link}"
        );
    }
    let remote = doc
        .descendants()
        .find(|n| {
            n.attribute("data-reference-attribute")
                == Some("data.terraform_remote_state.app.outputs.cluster")
        })
        .unwrap();
    assert_eq!(
        remote.attribute("href"),
        Some(format!("#{}", ids[0]).as_str())
    );
}

#[test]
fn consumer_uses_only_direct_references_and_keeps_content_inside_cards() {
    let graph = semantic::transform(&plan::parse(&input().to_string()).unwrap());
    let layout = Layout::new(&graph);
    let output = svg::render(&graph, &layout);
    let doc = roxmltree::Document::parse(&output).unwrap();
    let card = |address| {
        doc.descendants()
            .find(|n| n.attribute("data-terraform-address") == Some(address))
            .unwrap()
    };
    let service = card("aws_ecs_service.app");
    for (attribute, text) in [
        ("cluster", "Cluster:"),
        ("task_definition", "Task definition:"),
    ] {
        let link = service
            .descendants()
            .find(|n| n.attribute("data-reference-attribute") == Some(attribute))
            .unwrap();
        assert!(
            link.descendants()
                .any(|n| n.has_tag_name("text") && n.text().unwrap().starts_with(text))
        );
    }
    let worker = card("aws_instance.app");
    assert!(
        worker
            .descendants()
            .any(|n| n.attribute("data-reference-attribute") == Some("iam_instance_profile"))
    );
    assert!(
        worker
            .descendants()
            .any(|n| n.attribute("data-reference-attribute") == Some("ami"))
    );
    assert!(
        !worker
            .descendants()
            .any(|n| n.attribute("data-reference-attribute") == Some("role"))
    );
    let dependency = worker
        .descendants()
        .find(|n| n.attribute("data-reference-attribute") == Some("depends_on"))
        .unwrap();
    assert!(
        dependency
            .descendants()
            .any(|n| n.has_tag_name("text") && n.text().unwrap().starts_with("Dependency:"))
    );
    assert!(
        card("aws_iam_instance_profile.app")
            .descendants()
            .any(|n| n.attribute("data-reference-attribute") == Some("role"))
    );
    assert!(
        card("aws_ecs_task_definition.app")
            .descendants()
            .any(|n| n.attribute("data-reference-attribute") == Some("configuration"))
    );
    for (index, node) in graph
        .nodes
        .iter()
        .enumerate()
        .filter(|(_, n)| n.placement != Placement::Logical)
    {
        let card = card(&node.address);
        for text in card.descendants().filter(|n| n.has_tag_name("text")) {
            let y: usize = text.attribute("y").unwrap().parse().unwrap();
            assert!(y + 4 < layout.bounds[index].origin.y + layout.header_heights[index]);
        }
    }
    let focused = view::single(
        &graph,
        &view::Options {
            focus: Some("aws_ecs_service.app".into()),
            depth: Some(0),
            ..Default::default()
        },
    )
    .unwrap();
    let output = svg::render(&focused, &Layout::new(&focused));
    assert!(output.contains("Unavailable"));
    let doc = roxmltree::Document::parse(&output).unwrap();
    assert!(
        !doc.descendants()
            .any(|n| n.attribute("data-reference-attribute").is_some())
    );
}

#[test]
fn shared_profiles_and_long_reference_lists_are_fully_visible() {
    let mut value = input();
    let mut worker = value["configuration"]["root_module"]["resources"][2].clone();
    worker["address"] = serde_json::json!("aws_instance.second");
    value["configuration"]["root_module"]["resources"]
        .as_array_mut()
        .unwrap()
        .push(worker);
    value["resource_changes"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::json!({"address":"aws_instance.second", "type":"aws_instance"}));
    let mut addresses = Vec::new();
    for index in 0..8 {
        let address = format!("aws_iam_policy.long_{}_{}", index, "name".repeat(30));
        value["resource_changes"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!({"address":address,"type":"aws_iam_policy"}));
        addresses.push(address);
    }
    value["configuration"]["root_module"]["resources"][0]["expressions"]["policies"] = serde_json::json!({"references":addresses.iter().map(|a| format!("{a}.arn")).collect::<Vec<_>>()});
    let graph = semantic::transform(&plan::parse(&value.to_string()).unwrap());
    let output = svg::render(&graph, &Layout::new(&graph));
    let doc = roxmltree::Document::parse(&output).unwrap();
    let profile = doc
        .descendants()
        .find(|n| n.attribute("data-terraform-address") == Some("aws_iam_instance_profile.app"))
        .unwrap()
        .attribute("id")
        .unwrap();
    assert_eq!(
        doc.descendants()
            .filter(|n| n.attribute("href") == Some(format!("#{profile}").as_str()))
            .count(),
        2
    );
    let service = doc
        .descendants()
        .find(|n| n.attribute("data-terraform-address") == Some("aws_ecs_service.app"))
        .unwrap();
    assert_eq!(
        service
            .descendants()
            .filter(|n| n.has_tag_name("a"))
            .count(),
        10
    );
    let summary = service
        .descendants()
        .find(|n| n.attribute("data-resource-references").is_some())
        .unwrap();
    let title = summary
        .children()
        .find(|n| n.has_tag_name("title"))
        .unwrap()
        .text()
        .unwrap();
    assert!(addresses.iter().all(|a| title.contains(a)));
    assert!(!output.contains("more (see full title)"));
    let group = doc
        .descendants()
        .find(|n| {
            n.attribute("data-logical-group") == Some("true")
                && n.attribute("data-resource-type") == Some("aws_iam_policy")
        })
        .unwrap();
    let rows: Vec<_> = group
        .descendants()
        .filter(|n| n.attribute("data-placement") == Some("logical"))
        .collect();
    assert_eq!(rows.len(), addresses.len());
    let mut bottom = 0;
    for row in rows {
        assert_logical_row(row, crate::model::Action::Other);
        assert!(row_y(row) > bottom);
        bottom = row_y(row) + number(underline(row), "y1");
    }
    for address in &addresses {
        assert!(
            summary
                .descendants()
                .filter(|n| n.has_tag_name("a"))
                .any(|link| {
                    let visible = link
                        .children()
                        .filter(|n| n.has_tag_name("text"))
                        .filter_map(|n| n.text())
                        .collect::<String>();
                    visible == format!("References: {address}")
                })
        );
    }
}

#[test]
fn incomplete_mixed_references_never_link_a_candidate_as_verified_usage() {
    let mut value = input();
    value["configuration"]["root_module"]["resources"][2]["expressions"]["iam_instance_profile"] = serde_json::json!({"references":["aws_iam_instance_profile.app.name", "aws_iam_instance_profile.missing.name"]});
    let graph = semantic::transform(&plan::parse(&value.to_string()).unwrap());
    let output = svg::render(&graph, &Layout::new(&graph));
    let doc = roxmltree::Document::parse(&output).unwrap();
    let worker = doc
        .descendants()
        .find(|n| n.attribute("data-terraform-address") == Some("aws_instance.app"))
        .unwrap();
    assert!(
        !worker
            .descendants()
            .any(|n| n.attribute("data-reference-attribute") == Some("iam_instance_profile"))
    );
    assert!(
        worker
            .descendants()
            .any(|n| n.has_tag_name("text") && n.text() == Some("Instance profile: unresolved"))
    );
    let node = graph
        .nodes
        .iter()
        .find(|n| n.address == "aws_instance.app")
        .unwrap();
    let reference = node
        .references
        .iter()
        .find(|r| r.attribute == "iam_instance_profile")
        .unwrap();
    assert!(!reference.complete);
    assert!(!reference.targets.is_empty());
}

#[test]
fn variable_backed_logical_usage_is_unresolved_but_constants_are_omitted() {
    let mut value = input();
    value["configuration"]["root_module"]["variables"] = serde_json::json!({"cluster":{}});
    for (expression, expected) in [
        (serde_json::json!({"references":["var.cluster"]}), true),
        (
            serde_json::json!({"constant_value":"external-cluster"}),
            false,
        ),
    ] {
        value["configuration"]["root_module"]["resources"][0]["expressions"]["cluster"] =
            expression;
        let graph = semantic::transform(&plan::parse(&value.to_string()).unwrap());
        let output = svg::render(&graph, &Layout::new(&graph));
        let doc = roxmltree::Document::parse(&output).unwrap();
        let service = doc
            .descendants()
            .find(|n| n.attribute("data-terraform-address") == Some("aws_ecs_service.app"))
            .unwrap();
        assert_eq!(
            service
                .descendants()
                .any(|n| n.has_tag_name("text") && n.text() == Some("Cluster: unresolved")),
            expected
        );
        assert!(
            !service
                .descendants()
                .any(|n| n.attribute("data-reference-attribute") == Some("cluster"))
        );
    }
}

#[test]
fn dynamic_logical_candidates_are_unresolved_while_exact_targets_link() {
    let mut value = input();
    value["resource_changes"].as_array_mut().unwrap().extend((0..2).map(|i| serde_json::json!({"address":format!("aws_iam_instance_profile.profile[{i}]"),"type":"aws_iam_instance_profile"})));
    for (traversal, links) in [
        ("aws_iam_instance_profile.profile[count.index].name", 0),
        ("aws_iam_instance_profile.profile[0].name", 1),
        ("aws_iam_instance_profile.profile[1].name", 1),
    ] {
        value["configuration"]["root_module"]["resources"][2]["expressions"]["iam_instance_profile"] =
            serde_json::json!({"references":[traversal]});
        let graph = semantic::transform(&plan::parse(&value.to_string()).unwrap());
        let output = svg::render(&graph, &Layout::new(&graph));
        let doc = roxmltree::Document::parse(&output).unwrap();
        let worker = doc
            .descendants()
            .find(|n| n.attribute("data-terraform-address") == Some("aws_instance.app"))
            .unwrap();
        assert_eq!(
            worker
                .descendants()
                .filter(|n| n.attribute("data-reference-attribute") == Some("iam_instance_profile"))
                .count(),
            links,
            "{traversal}"
        );
        if links == 0 {
            assert!(
                worker
                    .descendants()
                    .any(|n| n.text() == Some("Instance profile: unresolved"))
            );
            assert!(worker.descendants().any(|n| n.has_tag_name("title")
                && n.text().is_some_and(|t| {
                    t.contains("iam_instance_profile; unresolved attribute reference")
                })));
            let reference = graph
                .nodes
                .iter()
                .find(|n| n.address == "aws_instance.app")
                .unwrap()
                .references
                .iter()
                .find(|r| r.attribute == "iam_instance_profile")
                .unwrap();
            assert_eq!(reference.targets.len(), 2);
            assert!(
                reference
                    .issues
                    .contains(&crate::model::DiagnosticReason::DynamicInstanceSelection)
            );
        }
    }
}

#[test]
fn launch_templates_are_logical_and_asg_usage_requires_complete_nested_references() {
    let input = serde_json::json!({
        "format_version":"1.2",
        "resource_changes":[
            {"address":"aws_launch_template.direct","type":"aws_launch_template","provider_name":"registry.terraform.io/hashicorp/aws","change":{"actions":["create"]}},
            {"address":"aws_launch_template.mixed","type":"aws_launch_template","provider_name":"registry.terraform.io/hashicorp/aws","change":{"actions":["update"]}},
            {"address":"aws_launch_template.dynamic[0]","type":"aws_launch_template","provider_name":"registry.terraform.io/hashicorp/aws","change":{"actions":["no-op"]}},
            {"address":"aws_launch_template.dynamic[1]","type":"aws_launch_template","provider_name":"registry.terraform.io/hashicorp/aws","change":{"actions":["no-op"]}},
            {"address":"data.aws_launch_template.lookup","type":"aws_launch_template","mode":"data","provider_name":"registry.terraform.io/hashicorp/aws","change":{"actions":["read"]}},
            {"address":"aws_autoscaling_group.direct","type":"aws_autoscaling_group","provider_name":"registry.terraform.io/hashicorp/aws"},
            {"address":"aws_autoscaling_group.mixed","type":"aws_autoscaling_group","provider_name":"registry.terraform.io/hashicorp/aws"},
            {"address":"aws_autoscaling_group.dynamic","type":"aws_autoscaling_group","provider_name":"registry.terraform.io/hashicorp/aws"}
        ],
        "configuration":{
            "provider_config":{
                "aws":{"full_name":"registry.terraform.io/hashicorp/aws","expressions":{"region":{"constant_value":"us-east-1"}}}
            },
            "root_module":{"resources":[
                {"address":"aws_launch_template.direct","provider_config_key":"aws","expressions":{}},
                {"address":"aws_launch_template.mixed","provider_config_key":"aws","expressions":{}},
                {"address":"data.aws_launch_template.lookup","provider_config_key":"aws","expressions":{}},
                {"address":"aws_autoscaling_group.direct","provider_config_key":"aws","expressions":{
                    "launch_template":[{"id":{"references":["aws_launch_template.direct.id"]}}]
                }},
                {"address":"aws_autoscaling_group.mixed","provider_config_key":"aws","expressions":{
                    "mixed_instances_policy":[{"launch_template":[{"launch_template_specification":[
                        {"launch_template_name":{"references":["aws_launch_template.mixed.name"]}}
                    ]}]}]
                }},
                {"address":"aws_autoscaling_group.dynamic","provider_config_key":"aws","expressions":{
                    "launch_template":[{"id":{"references":["aws_launch_template.dynamic[count.index].id"]}}]
                }}
            ]}
        }
    });
    let graph = semantic::transform(&plan::parse(&input.to_string()).unwrap());

    for address in [
        "aws_launch_template.direct",
        "aws_launch_template.mixed",
        "aws_launch_template.dynamic[0]",
        "aws_launch_template.dynamic[1]",
        "data.aws_launch_template.lookup",
    ] {
        let node = graph
            .nodes
            .iter()
            .find(|node| node.address == address)
            .unwrap();
        assert_eq!(node.placement, Placement::Logical, "{address}");
        assert_eq!(node.role, crate::model::ResourceRole::Node, "{address}");
    }
    for address in [
        "aws_launch_template.direct",
        "aws_launch_template.mixed",
        "data.aws_launch_template.lookup",
    ] {
        let node = graph
            .nodes
            .iter()
            .find(|node| node.address == address)
            .unwrap();
        assert_eq!(
            node.entity
                .scope
                .as_ref()
                .unwrap()
                .region
                .as_ref()
                .unwrap()
                .0
                .as_str(),
            "us-east-1"
        );
    }

    let output = svg::render(&graph, &Layout::new(&graph));
    let doc = roxmltree::Document::parse(&output).unwrap();
    for (address, attribute) in [
        ("aws_autoscaling_group.direct", "launch_template.id"),
        (
            "aws_autoscaling_group.mixed",
            "mixed_instances_policy.launch_template.launch_template_specification.launch_template_name",
        ),
    ] {
        let card = doc
            .descendants()
            .find(|node| node.attribute("data-terraform-address") == Some(address))
            .unwrap();
        let link = card
            .descendants()
            .find(|node| node.attribute("data-reference-attribute") == Some(attribute))
            .unwrap();
        assert!(
            link.descendants().any(|node| {
                node.has_tag_name("text")
                    && node
                        .text()
                        .is_some_and(|text| text.starts_with("Launch template:"))
            }),
            "{address}"
        );
        let target = link.attribute("href").unwrap().strip_prefix('#').unwrap();
        assert!(doc.descendants().any(|node| {
            node.attribute("id") == Some(target)
                && node.attribute("data-placement") == Some("logical")
        }));
    }

    let dynamic = graph
        .nodes
        .iter()
        .find(|node| node.address == "aws_autoscaling_group.dynamic")
        .unwrap();
    let reference = dynamic
        .references
        .iter()
        .find(|reference| reference.attribute == "launch_template.id")
        .unwrap();
    assert_eq!(reference.label, "Launch template");
    assert!(
        reference
            .issues
            .contains(&crate::model::DiagnosticReason::DynamicInstanceSelection)
    );
    let card = doc
        .descendants()
        .find(|node| {
            node.attribute("data-terraform-address") == Some("aws_autoscaling_group.dynamic")
        })
        .unwrap();
    assert!(
        !card
            .descendants()
            .any(|node| node.attribute("data-reference-attribute") == Some("launch_template.id"))
    );
    assert!(card.descendants().any(|node| {
        node.has_tag_name("text") && node.text() == Some("Launch template: unresolved")
    }));
}
