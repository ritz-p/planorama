use crate::{layout::Layout, model::Placement, plan, semantic, svg, view};

fn input() -> serde_json::Value {
    serde_json::json!({"format_version":"1.2", "resource_changes":[
        {"address":"aws_ecs_cluster.app", "type":"aws_ecs_cluster", "previous_address":"aws_ecs_cluster.old", "change":{"actions":["update"],"importing":{"id":"PRIVATE_IMPORT"},"after":{"secret":"PRIVATE_VALUE"}}},
        {"address":"aws_ecs_task_definition.app", "type":"aws_ecs_task_definition"},
        {"address":"aws_ecs_service.app", "type":"aws_ecs_service"},
        {"address":"aws_iam_role.app", "type":"aws_iam_role"},
        {"address":"aws_iam_instance_profile.app", "type":"aws_iam_instance_profile"},
        {"address":"aws_instance.app", "type":"aws_instance"},
        {"address":"data.aws_ami.app", "type":"aws_ami", "mode":"data"},
        {"address":"custom_resource.app", "type":"custom_resource"}
    ],"configuration":{"root_module":{"resources":[
        {"address":"aws_ecs_service.app", "expressions":{"cluster":{"references":["aws_ecs_cluster.app.id"]},"task_definition":{"references":["aws_ecs_task_definition.app.arn"]}}},
        {"address":"aws_iam_instance_profile.app", "expressions":{"role":{"references":["aws_iam_role.app.name"]}}},
        {"address":"aws_instance.app", "depends_on":["aws_ecs_cluster.app"], "expressions":{"iam_instance_profile":{"references":["aws_iam_instance_profile.app.name"]},"ami":{"references":["data.aws_ami.app.id"]},"unknown":{"references":["aws_iam_role.missing.arn"]}}},
        {"address":"aws_ecs_task_definition.app", "expressions":{"configuration":{"references":["aws_instance.app.id"]}}}
    ]}}})
}

#[test]
fn logical_projection_retains_graph_identity_references_and_operations() {
    let raw = plan::parse(&input().to_string()).unwrap();
    let graph = semantic::transform(&raw);
    let original = graph.clone();
    let layout = Layout::new(&graph);
    assert_eq!(graph.nodes.len(), 8);
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
    assert_eq!(
        doc.descendants()
            .filter(|n| n.attribute("data-placement") == Some("logical"))
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
        .find(|n| n.attribute("data-terraform-address") == Some("aws_ecs_cluster.app"))
        .unwrap();
    assert_eq!(
        cluster.attribute("data-previous-address"),
        Some("aws_ecs_cluster.old")
    );
    assert_eq!(cluster.attribute("data-import"), Some("true"));
    assert_eq!(cluster.attribute("data-role"), Some("controller"));
    assert!(cluster.children().any(|n| {
        n.has_tag_name("title")
            && n.text()
                .unwrap()
                .starts_with("module.root.aws_ecs_cluster.app")
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
    assert_eq!(role.attribute("data-role"), Some("policy"));
    assert!(
        role.children()
            .any(|n| n.has_tag_name("title") && n.text().unwrap().starts_with("aws_iam_role.app"))
    );
    assert!(!output.contains("PRIVATE_"));
    assert!(output.contains("3 spatial cards, 5 logical cards"));
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
        ("aws_future_resource", Placement::Unresolved),
        ("aws_cloudwatch_log_group", Placement::Spatial),
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
            .find(|n| n.address == "aws_ecs_cluster.app")
            .unwrap()
            .placement,
        Placement::Unresolved
    );
}

#[test]
fn logical_focus_and_changes_preserve_selected_resources_and_reference_evidence() {
    let graph = semantic::transform(&plan::parse(&input().to_string()).unwrap());
    for (focus, count) in [("aws_instance.app", 0), ("aws_ecs_cluster.app", 1)] {
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
            .any(|n| n.address == "aws_ecs_cluster.app")
    );
}

#[test]
fn logical_cards_wrap_long_addresses_and_keep_deposed_identity_and_input_order() {
    let mut value = input();
    let address = format!("aws_iam_policy.{}", "long_name".repeat(30));
    value["resource_changes"].as_array_mut().unwrap().extend([
        serde_json::json!({"address":address, "type":"aws_iam_policy"}),
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
    assert!(
        cards
            .iter()
            .any(|n| n.attribute("data-deposed-key") == Some("old"))
    );
    for card in cards {
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
                address: "aws_ecs_cluster.app".into(),
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
    let output = svg::render_states(&view, Default::default());
    let doc = roxmltree::Document::parse(&output).unwrap();
    let ids: Vec<_> = doc
        .descendants()
        .filter(|n| n.attribute("data-terraform-address") == Some("aws_ecs_cluster.app"))
        .map(|n| n.attribute("id").unwrap())
        .collect();
    assert_eq!(ids.len(), 2);
    assert_ne!(ids[0], ids[1]);
    assert!(output.contains("cross-state-relationships"));
    assert!(!output.contains("independent states"));
    assert!(output.contains("1 logical"));
    assert!(
        !doc.descendants()
            .any(|n| n.has_tag_name("path") && n.attribute("data-cross-state").is_some())
    );
    assert_eq!(all.relationships, view.relationships());
}
