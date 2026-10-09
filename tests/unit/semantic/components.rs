use crate::{model::ResourceRole, plan, semantic};
use serde_json::{Value, json};

fn input() -> Value {
    serde_json::from_str(include_str!("../../fixtures/components-plan.json")).unwrap()
}

#[test]
fn logical_components_preserve_source_resources_actions_and_edges() {
    let raw = plan::parse(&input().to_string()).unwrap();
    let graph = semantic::transform(&raw);
    assert!(crate::semantic::base_graph(&raw).components.is_empty());
    assert_eq!(graph.nodes, crate::semantic::base_graph(&raw).nodes);
    assert_eq!(graph.edges, crate::semantic::base_graph(&raw).edges);
    assert_eq!(graph.checks, raw.checks);
    assert_eq!(graph.components.len(), 1);
    let component = &graph.components[0];
    assert_eq!(component.kind, "load_balancer");
    assert_eq!(component.id, "logical:load_balancer:aws_lb.app");
    assert_eq!(
        component.members,
        [
            "aws_lb.app",
            "aws_lb_listener.https",
            "aws_lb_listener_rule.api"
        ]
    );
    assert!(
        graph
            .nodes
            .iter()
            .all(|n| n.role != ResourceRole::Container)
    );
    let rule = graph
        .nodes
        .iter()
        .find(|n| n.address == "aws_lb_listener_rule.api")
        .unwrap();
    assert_eq!(
        rule.previous_address.as_deref(),
        Some("aws_lb_listener_rule.old")
    );
    assert!(rule.metadata.replace_paths.is_some());
    assert!(!format!("{graph:?}").contains("TOP_SECRET"));
}

#[test]
fn unsupported_and_ambiguous_listener_parents_are_not_grouped() {
    for case in 0..7 {
        let mut input = input();
        let reference = &mut input["configuration"]["root_module"]["resources"][0]["expressions"]["load_balancer_arn"];
        match case {
            0 => *reference = json!({"constant_value":"TOP_SECRET"}),
            1 => *reference = json!({"references":["aws_lb.app.arn", "aws_lb.other.arn"]}),
            2 => *reference = json!({"references":["aws_lb.missing.arn"]}),
            3 => *reference = json!({"references":["aws_lb_target_group.shared.arn"]}),
            4 => input["resource_changes"][1]["mode"] = "data".into(),
            5 => input["resource_changes"][0]["provider_name"] = "example.com/custom/aws".into(),
            _ => {
                let mut deposed = input["resource_changes"][0].clone();
                deposed["deposed"] = "old".into();
                input["resource_changes"]
                    .as_array_mut()
                    .unwrap()
                    .push(deposed);
            }
        }
        let raw = plan::parse(&input.to_string()).unwrap();
        let graph = semantic::transform(&raw);
        assert!(graph.components.is_empty(), "case {case}");
        assert_eq!(graph.nodes.len(), raw.nodes.len());
    }
}

#[test]
fn ambiguous_rules_stay_ungrouped_and_member_order_is_deterministic() {
    let mut input = input();
    input["configuration"]["root_module"]["resources"][1]["expressions"]["listener_arn"] =
        json!({"references":["aws_lb_listener.missing.arn"]});
    let expected = semantic::transform(&plan::parse(&input.to_string()).unwrap());
    assert_eq!(expected.components[0].members.len(), 2);
    input["resource_changes"].as_array_mut().unwrap().reverse();
    input["configuration"]["root_module"]["resources"]
        .as_array_mut()
        .unwrap()
        .reverse();
    let graph = semantic::transform(&plan::parse(&input.to_string()).unwrap());
    assert_eq!(graph.components, expected.components);
    assert_eq!(graph.nodes, expected.nodes);
}
