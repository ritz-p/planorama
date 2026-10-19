use crate::{
    layout::Layout,
    model::{EdgeKind, Placement, ReferenceKind},
    plan, semantic, svg, view,
};
use serde_json::{Value, json};

const FIXTURE: &str = include_str!("../../fixtures/ecs-clusters-plan.json");

#[test]
fn ecs_membership_preserves_independent_cluster_and_multivpc_service_placement() {
    super::service_references::tests::check_service_fixture(
        FIXTURE,
        "Cluster membership",
        "cluster",
    );
    let graph = semantic::transform(&plan::parse(FIXTURE).unwrap());
    assert_eq!(graph.coverage.selected.resources, 11);
    let index = |address: &str| {
        graph
            .nodes
            .iter()
            .position(|n| n.address == address)
            .unwrap()
    };
    let cluster = index("aws_ecs_cluster.shared");
    assert!(graph.nodes.iter().all(|n| n.member_owner.is_none()));
    assert!(
        graph
            .edges
            .iter()
            .all(|e| e.kind != EdgeKind::Containment || (e.from != cluster && e.to != cluster))
    );
    for (service, parent) in [
        ("aws_ecs_service.a", "aws_vpc.a"),
        ("aws_ecs_service.b", "aws_subnet.b"),
    ] {
        assert!(graph.edges.iter().any(|e| e.kind == EdgeKind::Containment
            && e.from == index(parent)
            && e.to == index(service)));
        let membership = graph.nodes[index(service)]
            .references
            .iter()
            .find(|r| r.label == "Cluster membership")
            .unwrap();
        assert!(membership.complete);
        assert_eq!(membership.kind, ReferenceKind::Membership);
        assert_eq!(membership.targets.len(), 1);
        assert_eq!(membership.targets[0].id.address, "aws_ecs_cluster.shared");
        let selected = view::single(
            &graph,
            &view::Options {
                focus: Some(service.into()),
                depth: Some(0),
                ..Default::default()
            },
        )
        .unwrap();
        assert!(
            !selected
                .nodes
                .iter()
                .any(|n| n.resource_type == "aws_ecs_cluster")
        );
        assert!(svg::render(&selected, &Layout::new(&selected)).contains("Unavailable"));
    }
    for node in &graph.nodes {
        assert_eq!(
            node.placement == Placement::Logical,
            node.resource_type == "aws_ecs_task_definition"
        );
        if node.resource_type == "aws_ecs_cluster" {
            assert_eq!(
                node.entity
                    .scope
                    .as_ref()
                    .and_then(|s| s.region.as_ref())
                    .is_some(),
                node.address != "aws_ecs_cluster.unknown"
            );
        }
    }
    let selected = view::single(
        &graph,
        &view::Options {
            focus: Some("aws_ecs_cluster.shared".into()),
            depth: Some(0),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(selected.nodes.len(), 1);
    assert_eq!(selected.coverage.selected.resources, 1);
    let input = FIXTURE.replace(
        "aws_ecs_cluster.shared",
        "module.cluster.aws_ecs_cluster.shared",
    );
    let graph = semantic::transform(&plan::parse(&input).unwrap());
    let selected = view::single(
        &graph,
        &view::Options {
            modules: vec!["module.cluster".into()],
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(selected.nodes.len(), 1);
    assert_eq!(selected.nodes[0].placement, Placement::Spatial);
}

#[test]
fn ecs_membership_requires_current_values_and_compatible_regional_bindings() {
    for case in [
        "name",
        "unknown-region",
        "other-region",
        "unknown-value",
        "data-consumer",
        "deposed-consumer",
        "tags",
        "dependency",
    ] {
        let mut value: Value = serde_json::from_str(FIXTURE).unwrap();
        match case {
            "name" => {
                value["resource_changes"][1]["change"]["after"]["cluster"] =
                    json!("PRIVATE_CLUSTER_NAME");
                value["configuration"]["root_module"]["resources"][1]["expressions"]["cluster"] =
                    json!({"references":["aws_ecs_cluster.shared.name"]});
            }
            "unknown-region" => {
                value["configuration"]["provider_config"]["aws"]["expressions"]["region"] =
                    json!({"references":["var.region"]})
            }
            "other-region" => {
                value["configuration"]["provider_config"]["aws.other"] = json!({"full_name":"hashicorp/aws", "expressions":{"region":{"constant_value":"eu-west-1"}}});
                value["configuration"]["root_module"]["resources"][1]["provider_config_key"] =
                    json!("aws.other");
            }
            "unknown-value" => {
                value["resource_changes"][1]["change"]["after_unknown"] = json!({"cluster":true})
            }
            "data-consumer" => value["resource_changes"][1]["mode"] = json!("data"),
            "deposed-consumer" => value["resource_changes"][1]["deposed"] = json!("old"),
            "tags" => {
                let expression =
                    value["configuration"]["root_module"]["resources"][1]["expressions"]["cluster"]
                        .take();
                value["configuration"]["root_module"]["resources"][1]["expressions"]["tags"] =
                    expression;
            }
            _ => {
                value["configuration"]["root_module"]["resources"][1]["expressions"]["cluster"] =
                    Value::Null;
                value["configuration"]["root_module"]["resources"][1]["depends_on"] =
                    json!(["aws_ecs_cluster.shared"]);
            }
        }
        let graph = semantic::transform(&plan::parse(&value.to_string()).unwrap());
        let consumer = graph
            .nodes
            .iter()
            .find(|n| n.address == "aws_ecs_service.a")
            .unwrap();
        assert_eq!(
            consumer
                .references
                .iter()
                .any(|r| r.kind == ReferenceKind::Membership),
            case == "name",
            "{case}"
        );
        if matches!(case, "tags" | "dependency") {
            assert!(
                view::references::rows(consumer)
                    .iter()
                    .any(|r| r.target.is_some()
                        && matches!(r.reference.label, "References" | "Dependency"))
            );
        }
        assert_eq!(graph.coverage.selected.resources, 11);
    }
    let mut parsed = plan::parse(FIXTURE).unwrap();
    for node in &mut parsed.nodes {
        if node.address == "aws_ecs_cluster.shared" {
            node.provider_configuration
                .as_mut()
                .unwrap()
                .account_conflict = true;
        }
    }
    let graph = semantic::transform(&parsed);
    assert!(
        graph
            .nodes
            .iter()
            .flat_map(|n| &n.references)
            .all(|r| r.kind != ReferenceKind::Membership)
    );
}

#[test]
fn ecs_membership_rejects_unknown_resource_override_scopes() {
    for endpoint in [0, 1] {
        for expression in [
            json!({"constant_value":"eu-west-1"}),
            json!({"references":["var.region"]}),
        ] {
            let mut value: Value = serde_json::from_str(FIXTURE).unwrap();
            value["resource_changes"][1]["change"]["after"]["cluster"] =
                json!("PRIVATE_CLUSTER_NAME");
            value["configuration"]["root_module"]["resources"][1]["expressions"]["cluster"] =
                json!({"references":["aws_ecs_cluster.shared.name"]});
            value["configuration"]["root_module"]["resources"][endpoint]["expressions"]["region"] =
                expression;
            value["resource_changes"][endpoint]["change"]["after"]["region"] = json!("eu-west-1");
            let parsed = plan::parse(&value.to_string()).unwrap();
            assert!(
                parsed
                    .attributes
                    .iter()
                    .filter(|r| r.target == 1 && r.attribute == "cluster")
                    .all(|r| !r.service_reference_verified)
            );
            let graph = semantic::transform(&parsed);
            let consumer = graph
                .nodes
                .iter()
                .find(|n| n.address == "aws_ecs_service.a")
                .unwrap();
            assert!(
                consumer
                    .references
                    .iter()
                    .all(|r| r.kind != ReferenceKind::Membership)
            );
            assert!(
                view::references::rows(consumer)
                    .iter()
                    .any(|r| r.reference.label == "Cluster" && r.target.is_some())
            );
            let address = if endpoint == 0 {
                "aws_ecs_cluster.shared"
            } else {
                "aws_ecs_service.a"
            };
            assert!(
                graph
                    .nodes
                    .iter()
                    .find(|n| n.address == address)
                    .unwrap()
                    .entity
                    .scope
                    .as_ref()
                    .and_then(|s| s.region.as_ref())
                    .is_none()
            );
            assert_eq!(graph.coverage.selected.resources, 11);
        }
    }
}

#[test]
fn ecs_long_identity_lifecycle_and_named_state_exports_remain_distinct() {
    let address = format!("aws_ecs_cluster.shared[\"{}<&\"]", "long".repeat(60));
    let mut value: Value = serde_json::from_str(
        &FIXTURE.replace("aws_ecs_cluster.shared", &address.replace('"', "\\\"")),
    )
    .unwrap();
    value["configuration"]["root_module"]["resources"][0]["address"] =
        json!("aws_ecs_cluster.shared");
    value["resource_changes"][0]["previous_address"] = json!("aws_ecs_cluster.old");
    value["resource_changes"][0]["change"]["importing"] = json!({"id":"PRIVATE_IMPORT"});
    let mut old = value["resource_changes"][0].clone();
    old["deposed"] = json!("old");
    old["change"]["actions"] = json!(["delete"]);
    value["resource_changes"].as_array_mut().unwrap().push(old);
    let graph = semantic::transform(&plan::parse(&value.to_string()).unwrap());
    assert_eq!(graph.coverage.selected.resources, 12);
    assert_eq!(
        graph
            .nodes
            .iter()
            .flat_map(|n| &n.references)
            .filter(|r| r.kind == ReferenceKind::Membership)
            .count(),
        2
    );
    let architecture = crate::model::architecture::Architecture {
        states: ["one", "two"]
            .into_iter()
            .map(|s| (crate::model::StateId::new(s).unwrap(), graph.clone()))
            .collect(),
        relationships: Vec::new(),
    };
    let selected = view::apply(&architecture, &Default::default()).unwrap();
    let output = svg::render_states(&selected, svg::Options::default());
    let doc = roxmltree::Document::parse(&output).unwrap();
    assert_eq!(
        doc.descendants()
            .filter(|n| n.attribute("data-terraform-address") == Some(address.as_str()))
            .count(),
        4
    );
    for href in doc.descendants().filter_map(|n| n.attribute("href")) {
        assert!(
            doc.descendants()
                .any(|n| n.attribute("id") == href.strip_prefix('#'))
        );
    }
    let html = crate::html::render_states(&selected, svg::Options::default());
    assert!(!html.contains("PRIVATE_"));
    for format in [crate::OutputFormat::Svg, crate::OutputFormat::Html] {
        let output = crate::generate(
            &value.to_string(),
            &crate::Options {
                format,
                icon_mode: crate::IconMode::ServiceName,
                ..Default::default()
            },
        )
        .unwrap()
        .content;
        assert!(!output.contains("PRIVATE_"));
        assert!(output.contains("aws_ecs_cluster.old"));
    }
}
