use crate::{
    model::{EdgeKind, Placement, ResourceRole},
    plan, semantic,
};
use serde_json::{Value, json};

const FIXTURE: &str = include_str!("../../fixtures/elasticache-serverless-plan.json");

#[test]
fn serverless_has_its_own_card_and_never_invents_provisioned_members() {
    let graph = semantic::transform(&plan::parse(FIXTURE).unwrap());
    assert_eq!(graph.nodes.len(), 6);
    assert_eq!(graph.coverage.selected.resources, 6);
    let cache = graph
        .nodes
        .iter()
        .position(|n| n.resource_type == "aws_elasticache_serverless_cache")
        .unwrap();
    let node = &graph.nodes[cache];
    assert_eq!(node.role, ResourceRole::Node);
    assert_eq!(node.placement, Placement::Spatial);
    assert!(node.member_owner.is_none());
    assert!(
        node.entity
            .scope
            .as_ref()
            .is_some_and(|s| !s.global && s.region.is_some())
    );
    assert!(graph.edges.iter().any(|e| e.to == cache
        && e.kind == EdgeKind::Containment
        && graph.nodes[e.from].resource_type == "aws_vpc"));
    assert!(graph.edges.iter().any(|e| e.to == cache
        && e.kind == EdgeKind::Connection
        && graph.nodes[e.from].resource_type == "aws_security_group"));
    assert!(
        !graph
            .edges
            .iter()
            .any(|e| e.from == cache && e.kind == EdgeKind::Containment)
    );
    for format in [crate::OutputFormat::Svg, crate::OutputFormat::Html] {
        for icon_mode in [crate::IconMode::Icon, crate::IconMode::ServiceName]
            .into_iter()
            .filter(|mode| cfg!(feature = "icon-artwork") || *mode == crate::IconMode::ServiceName)
        {
            let output = crate::generate(
                FIXTURE,
                &crate::Options {
                    format,
                    icon_mode,
                    ..Default::default()
                },
            )
            .unwrap()
            .content;
            assert!(!output.contains("PRIVATE_"));
            assert!(output.contains("aws_elasticache_serverless_cache"));
            assert!(output.contains("aws_elasticache_cluster"));
            if icon_mode == crate::IconMode::ServiceName {
                assert!(output.contains("Amazon ElastiCache"));
            }
        }
    }
}

#[test]
fn serverless_requires_explicit_complete_current_network_evidence() {
    for (attribute, endpoint, kind) in [
        ("subnet_ids", 2, EdgeKind::Containment),
        ("security_group_ids", 4, EdgeKind::Connection),
    ] {
        for case in 0..13 {
            let mut input: Value = serde_json::from_str(FIXTURE).unwrap();
            match case {
                1 => {
                    input["resource_changes"][endpoint]["provider_name"] =
                        json!("example.com/foreign/aws")
                }
                2 => {
                    input["resource_changes"][endpoint]["change"]["after_unknown"] =
                        json!({"id":true})
                }
                3 => {
                    input["resource_changes"][0]["change"]["after"][attribute] = json!(["literal"])
                }
                4 => input["resource_changes"][0]["change"]["actions"] = json!(["delete"]),
                5 => input["resource_changes"][endpoint]["deposed"] = json!("old"),
                6 => input["resource_changes"][endpoint]["mode"] = json!("data"),
                7 => {
                    input["configuration"]["root_module"]["resources"][0]["expressions"]
                        [attribute] = json!({"references":["var.selector"]})
                }
                8 => {
                    input["configuration"]["root_module"]["resources"][endpoint]["expressions"]["region"] =
                        json!({"constant_value":"us-west-2"})
                }
                9 => input["resource_changes"][endpoint]["type"] = json!("aws_instance"),
                10 => {
                    input["resource_changes"][0]["change"]["after_unknown"][attribute] = json!(true)
                }
                11 => input["resource_changes"][0]["mode"] = json!("data"),
                12 => {
                    input["resource_changes"][0]["provider_name"] = json!("example.com/foreign/aws")
                }
                _ => {}
            }
            let graph = semantic::transform(&plan::parse(&input.to_string()).unwrap());
            assert_eq!(
                graph.edges.iter().any(|e| e.kind == kind
                    && graph.nodes[e.to].resource_type == "aws_elasticache_serverless_cache"),
                case == 0,
                "{attribute}/{case}"
            );
            assert_eq!(graph.coverage.selected.resources, 6);
        }
    }
}

#[test]
fn serverless_keeps_indexed_filtered_and_state_qualified_identity() {
    let mut indexed: Value =
        serde_json::from_str(&FIXTURE.replace("aws_subnet.a", "aws_subnet.a[0]")).unwrap();
    indexed["configuration"]["root_module"]["resources"][2]["address"] = json!("aws_subnet.a");
    let graph = semantic::transform(&plan::parse(&indexed.to_string()).unwrap());
    assert!(graph.edges.iter().any(|e| e.kind == EdgeKind::Containment
        && graph.nodes[e.to].resource_type == "aws_elasticache_serverless_cache"));
    let filtered = crate::view::single(
        &graph,
        &crate::view::Options {
            focus: Some("aws_elasticache_serverless_cache.app".into()),
            depth: Some(0),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(filtered.nodes.len(), 2);
    assert!(filtered.nodes.iter().all(|n| matches!(
        n.resource_type.as_str(),
        "aws_vpc" | "aws_elasticache_serverless_cache"
    )));
    assert!(
        filtered
            .edges
            .iter()
            .all(|e| e.kind == EdgeKind::Containment)
    );
    let mut ambiguous: Value = serde_json::from_str(FIXTURE).unwrap();
    let mut second = ambiguous["resource_changes"][2].clone();
    ambiguous["resource_changes"][2]["address"] = json!("aws_subnet.a[0]");
    second["address"] = json!("aws_subnet.a[1]");
    ambiguous["resource_changes"]
        .as_array_mut()
        .unwrap()
        .push(second);
    let graph = semantic::transform(&plan::parse(&ambiguous.to_string()).unwrap());
    assert!(!graph.edges.iter().any(|e| e.kind == EdgeKind::Containment
        && graph.nodes[e.to].resource_type == "aws_elasticache_serverless_cache"));
    let plans = plan::parse_inputs(
        ["one", "two"]
            .into_iter()
            .map(|state| crate::model::PlanInput {
                state_id: crate::model::StateId::new(state).unwrap(),
                plan_json: FIXTURE.into(),
            })
            .collect(),
    )
    .unwrap();
    assert!(semantic::cross_state::resolve(&plans, &[]).edges.is_empty());
    for plan in plans.states.values() {
        assert_eq!(semantic::transform(plan).coverage.selected.resources, 6);
    }
}
