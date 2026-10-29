use crate::{model::EdgeKind, plan, semantic};
use serde_json::{Value, json};

fn attachment(target_type: &str, kind: &str, field: &str) -> Value {
    json!({"format_version":"1.2", "resource_changes":[
        {"address":"aws_lb_target_group.app", "type":"aws_lb_target_group", "change":{"actions":["no-op"], "after":{"arn":"PRIVATE_GROUP", "id":"PRIVATE_GROUP", "target_type":target_type}}},
        {"address":format!("{kind}.app"), "type":kind, "change":{"actions":["no-op"], "after":{(field):"PRIVATE_TARGET", "load_balancer_type":"application"}}},
        {"address":"aws_lb_target_group_attachment.app", "type":"aws_lb_target_group_attachment", "change":{"actions":["create"], "after":{"target_group_arn":"PRIVATE_GROUP", "target_id":"PRIVATE_TARGET"}}}
    ], "configuration":{"provider_config":{"aws":{"name":"aws", "full_name":"registry.terraform.io/hashicorp/aws", "expressions":{"region":{"constant_value":"us-east-1"}}}}, "root_module":{"resources":[
        {"address":"aws_lb_target_group.app", "provider_config_key":"aws"},
        {"address":format!("{kind}.app"), "provider_config_key":"aws"},
        {"address":"aws_lb_target_group_attachment.app", "provider_config_key":"aws", "expressions":{
            "target_group_arn":{"references":["aws_lb_target_group.app.arn"]},
            "target_id":{"references":[format!("{kind}.app.{field}")]}
        }}
    ]}}})
}

#[test]
fn typed_attachments_require_matching_current_explicit_endpoints() {
    for (target_type, kind, field) in [
        ("instance", "aws_instance", "id"),
        ("lambda", "aws_lambda_function", "arn"),
        ("alb", "aws_lb", "arn"),
    ] {
        for case in 0..22 {
            let mut input = attachment(target_type, kind, field);
            match case {
                1 => input["resource_changes"][0]["change"]["after"]["target_type"] = json!("ip"),
                2 => input["resource_changes"][2]["change"]["after"]["target_id"] = json!("OTHER"),
                3 => {
                    input["resource_changes"][2]["change"]["after"]["target_group_arn"] =
                        json!("OTHER")
                }
                4 => {
                    input["resource_changes"][0]["change"]["after_unknown"] =
                        json!({"target_type":true})
                }
                5 => {
                    input["resource_changes"][1]["change"]["after_sensitive"] =
                        json!({(field):true})
                }
                6 => {
                    input["resource_changes"][2]["change"]["after_unknown"] =
                        json!({"target_id":true})
                }
                7 => input["resource_changes"][0]["mode"] = json!("data"),
                8 => {
                    input["resource_changes"][1]["provider_name"] = json!("example.com/foreign/aws")
                }
                9 => input["resource_changes"][1]["deposed"] = json!("old"),
                10 => input["resource_changes"][2]["change"]["actions"] = json!(["delete"]),
                11 => input["resource_changes"][1]["change"]["actions"] = json!(["delete"]),
                12 => {
                    input["configuration"]["root_module"]["resources"][2]["expressions"]["target_id"] =
                        json!({"constant_value":"PRIVATE_TARGET"})
                }
                13 => {
                    input["configuration"]["root_module"]["resources"][2]["expressions"]["target_id"] =
                        json!({"references":[format!("{kind}.app.{field}"), "var.unknown"]})
                }
                14 => {
                    input["configuration"]["root_module"]["resources"][1]["provider_config_key"] =
                        json!("aws.other")
                }
                15 => {
                    input["configuration"]["provider_config"]["aws"]["expressions"]["region"] =
                        json!({"references":["var.region"]})
                }
                16 => input["resource_changes"][0]["change"]["after"]["target_type"] = Value::Null,
                17 => {
                    input["configuration"]["root_module"]["resources"][2]["expressions"]["target_group_arn"] =
                        json!({"references":["aws_lb_target_group.app.name"]})
                }
                18 => {
                    input["resource_changes"][0]["change"]["after_sensitive"] =
                        json!({"target_type":true})
                }
                19 => {
                    input["resource_changes"][2]["provider_name"] = json!("example.com/foreign/aws")
                }
                20 => {
                    input["planned_values"]["root_module"]["resources"] =
                        json!([{"address":format!("{kind}.app"),"sensitive_values":{(field):true}}])
                }
                21 => input["resource_changes"][2]["change"]["after_sensitive"] = json!(true),
                _ => {}
            }
            let raw = plan::parse(&input.to_string()).unwrap();
            let graph = semantic::transform(&raw);
            assert_eq!(
                graph.edges.iter().any(|e| e.kind == EdgeKind::Association),
                case == 0,
                "{target_type}/{case}"
            );
            assert_eq!(
                graph.nodes.len(),
                if case == 0 { 2 } else { 3 },
                "{target_type}/{case}"
            );
            assert!(!format!("{raw:?}{graph:?}").contains("PRIVATE_"));
        }
    }
}

#[test]
fn alb_target_requires_application_type_and_verified_aliases() {
    let input = attachment("alb", "aws_lb", "arn")
        .to_string()
        .replace("aws_lb", "aws_alb");
    let mut input: Value = serde_json::from_str(&input).unwrap();
    assert!(
        semantic::transform(&plan::parse(&input.to_string()).unwrap())
            .edges
            .iter()
            .any(|e| e.kind == EdgeKind::Association)
    );
    for value in [json!("network"), Value::Null] {
        input["resource_changes"][1]["change"]["after"]["load_balancer_type"] = value;
        assert!(
            !semantic::transform(&plan::parse(&input.to_string()).unwrap())
                .edges
                .iter()
                .any(|e| e.kind == EdgeKind::Association)
        );
    }
}

#[test]
fn indexed_targets_are_unique_and_extra_dependencies_keep_helpers_visible() {
    let mut input = attachment("lambda", "aws_lambda_function", "arn");
    input["resource_changes"][1]["address"] = json!("aws_lambda_function.app[0]");
    input["resource_changes"][1]["index"] = json!(0);
    let mut other = input["resource_changes"][1].clone();
    other["address"] = json!("aws_lambda_function.app[1]");
    other["index"] = json!(1);
    input["resource_changes"]
        .as_array_mut()
        .unwrap()
        .push(other);
    for (reference, valid) in [
        ("aws_lambda_function.app[0].arn", true),
        ("aws_lambda_function.app.arn", false),
    ] {
        input["configuration"]["root_module"]["resources"][2]["expressions"]["target_id"] =
            json!({"references":[reference]});
        let graph = semantic::transform(&plan::parse(&input.to_string()).unwrap());
        assert_eq!(
            graph.edges.iter().any(|e| e.kind == EdgeKind::Association),
            valid
        );
    }
    input["configuration"]["root_module"]["resources"][2]["expressions"]["target_id"] =
        json!({"references":["aws_lambda_function.app[0].arn"]});
    input["configuration"]["root_module"]["resources"][2]["depends_on"] =
        json!(["aws_lambda_function.app[1]"]);
    let graph = semantic::transform(&plan::parse(&input.to_string()).unwrap());
    assert_eq!(graph.nodes.len(), 4);
    assert!(
        graph
            .nodes
            .iter()
            .flat_map(|n| &n.references)
            .any(|r| r.label == "Registered target reference" && r.complete)
    );
}

#[test]
fn attachment_proof_stays_within_named_states_and_selection() {
    let fixture = attachment("instance", "aws_instance", "id").to_string();
    let plans = plan::parse_inputs(
        ["first", "second"]
            .into_iter()
            .map(|state| crate::model::PlanInput {
                state_id: crate::model::StateId::new(state).unwrap(),
                plan_json: fixture.clone(),
            })
            .collect(),
    )
    .unwrap();
    assert!(semantic::cross_state::resolve(&plans, &[]).edges.is_empty());
    for plan in plans.states.values() {
        let graph = semantic::transform(plan);
        assert_eq!(graph.coverage.selected.resources, 3);
        let edge = graph
            .edges
            .iter()
            .find(|e| e.kind == EdgeKind::Association)
            .unwrap();
        assert_eq!(
            edge.change.as_ref().unwrap().address,
            "aws_lb_target_group_attachment.app"
        );
        let selected = crate::view::single(
            &graph,
            &crate::view::Options {
                focus: Some("aws_lb_target_group.app".into()),
                depth: Some(0),
                ..Default::default()
            },
        )
        .unwrap();
        assert!(
            !selected
                .edges
                .iter()
                .any(|e| e.kind == EdgeKind::Association)
        );
    }
}
