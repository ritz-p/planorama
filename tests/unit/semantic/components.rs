use crate::{model::ResourceRole, plan, semantic};
use serde_json::{Value, json};

fn input() -> Value {
    serde_json::from_str(include_str!("../../fixtures/components-plan.json")).unwrap()
}

fn forwarding_input() -> Value {
    let mut input = input();
    input["configuration"]["root_module"]["resources"][1]["expressions"]["action"] = json!([
        {"forward":[{"target_group":[{"arn":{"references":["aws_lb_target_group.shared.arn"]}}]}]}
    ]);
    input["resource_changes"][0]["change"]["after"] = json!({"arn":"PRIVATE_LB"});
    input["resource_changes"][1]["change"]["after"] = json!({"arn":"PRIVATE_LISTENER", "load_balancer_arn":"PRIVATE_LB", "default_action":[{"target_group_arn":"PRIVATE_TARGET"}]});
    input["resource_changes"][2]["change"]["after"] = json!({"listener_arn":"PRIVATE_LISTENER", "action":[{"forward":[{"target_group":[{"arn":"PRIVATE_TARGET"}]}]}]});
    input["resource_changes"][3]["change"]["after"] = json!({"arn":"PRIVATE_TARGET"});
    input["resource_changes"][4]["change"]["after"] = json!({"arn":"PRIVATE_OTHER_LB"});
    input
}

fn contains_target(graph: &crate::model::Graph, address: &str) -> bool {
    graph
        .components
        .iter()
        .any(|c| c.members.iter().any(|m| m == address))
}

#[test]
fn reference_candidates_need_known_matching_arn_values_on_every_ownership_hop() {
    for case in 0..12 {
        let mut input = forwarding_input();
        match case {
            0 => input["resource_changes"][1]["change"]["after"]["default_action"][0]["target_group_arn"] = "PRIVATE_EXTERNAL".into(),
            1 => input["resource_changes"][2]["change"]["after"]["action"][0]["forward"][0]["target_group"][0]["arn"] = "PRIVATE_EXTERNAL".into(),
            2 => input["resource_changes"][1]["change"]["after"]["load_balancer_arn"] = "PRIVATE_EXTERNAL".into(),
            3 => input["resource_changes"][2]["change"]["after"]["listener_arn"] = "PRIVATE_EXTERNAL".into(),
            4 => input["resource_changes"][1]["change"]["after_unknown"] = json!({"default_action":[{"target_group_arn":true}]}),
            5 => input["resource_changes"][3]["change"]["after_unknown"] = json!({"arn":true}),
            6 => input["resource_changes"][1]["change"]["after_unknown"] = json!({"default_action":true}),
            7 => { input["resource_changes"][1]["change"].as_object_mut().unwrap().remove("after"); },
            8 => input["resource_changes"][3]["change"]["after"]["arn"] = "".into(),
            9 => input["resource_changes"][2]["change"]["after"]["action"][0]["forward"][0]["target_group"].as_array_mut().unwrap().push(json!({"arn":"PRIVATE_EXTERNAL"})),
            10 => input["resource_changes"][0]["change"]["after_unknown"] = json!(true),
            _ => input["resource_changes"][1]["change"]["after"] = Value::Null,
        }
        let raw = plan::parse(&input.to_string()).unwrap();
        let graph = semantic::transform(&raw);
        assert!(
            !contains_target(&graph, "aws_lb_target_group.shared"),
            "case {case}"
        );
        assert_eq!(graph.components[0].members.len(), 3);
        assert_eq!(graph.edges, crate::semantic::base_graph(&raw).edges);
        assert!(!format!("{raw:?}{graph:?}").contains("PRIVATE_"));
    }
}

#[test]
fn complete_forwarding_paths_add_one_target_member_without_changing_graph_semantics() {
    let mut input = forwarding_input();
    input["resource_changes"]
        .as_array_mut()
        .unwrap()
        .push(json!({"address":"aws_lb_listener.second", "type":"aws_lb_listener", "change":{"after":{"load_balancer_arn":"PRIVATE_LB", "default_action":[{"forward":[{"target_group":[{"arn":"PRIVATE_TARGET"}]}]}]}}}));
    input["configuration"]["root_module"]["resources"].as_array_mut().unwrap().push(json!({
        "address":"aws_lb_listener.second", "expressions":{
            "load_balancer_arn":{"references":["aws_lb.app.arn"]},
            "default_action":[{"forward":[{"target_group":[{"arn":{"references":["aws_lb_target_group.shared.arn"]}}]}]}]
        }
    }));
    let raw = plan::parse(&input.to_string()).unwrap();
    let graph = semantic::transform(&raw);
    let base = crate::semantic::base_graph(&raw);
    assert_eq!(graph.nodes, base.nodes);
    assert_eq!(graph.edges, base.edges);
    assert_eq!(graph.relationships, base.relationships);
    assert!(!format!("{raw:?}{graph:?}").contains("PRIVATE_"));
    let component = &graph.components[0];
    assert_eq!(
        component
            .members
            .iter()
            .filter(|m| *m == "aws_lb_target_group.shared")
            .count(),
        1
    );
    assert_eq!(component.entity.provenance.len(), component.members.len());
    let old_id = semantic::transform(&plan::parse(&super::tests::input().to_string()).unwrap())
        .components[0]
        .entity
        .id
        .clone();
    assert_eq!(component.entity.id, old_id);
    input["resource_changes"].as_array_mut().unwrap().reverse();
    input["configuration"]["root_module"]["resources"]
        .as_array_mut()
        .unwrap()
        .reverse();
    let mut reordered = plan::parse(&input.to_string()).unwrap();
    reordered.edges.reverse();
    reordered.attributes.reverse();
    assert_eq!(graph.components, semantic::transform(&reordered).components);
}

#[test]
fn shared_or_unresolved_owners_never_report_a_unique_target_owner() {
    for parent in [
        json!({"references":["aws_lb.other.arn"]}),
        json!({"references":["aws_lb.missing.arn"]}),
        json!({"references":["aws_lb.app.arn", "aws_lb.other.arn"]}),
        json!({"constant_value":"unknown"}),
    ] {
        let mut input = forwarding_input();
        input["resource_changes"]
            .as_array_mut()
            .unwrap()
            .push(json!({"address":"aws_lb_listener.second", "type":"aws_lb_listener", "change":{"after":{"load_balancer_arn":"PRIVATE_OTHER_LB", "default_action":[{"target_group_arn":"PRIVATE_TARGET"}]}}}));
        input["configuration"]["root_module"]["resources"].as_array_mut().unwrap().push(json!({"address":"aws_lb_listener.second", "expressions":{
            "load_balancer_arn": parent,
            "default_action":[{"target_group_arn":{"references":["aws_lb_target_group.shared.arn"]}}]
        }}));
        let graph = semantic::transform(&plan::parse(&input.to_string()).unwrap());
        assert!(!contains_target(&graph, "aws_lb_target_group.shared"));
        assert!(
            graph
                .nodes
                .iter()
                .any(|n| n.address == "aws_lb_target_group.shared")
        );
        assert!(
            graph.components[0]
                .members
                .iter()
                .any(|m| m == "aws_lb_listener.https")
        );
    }
}

#[test]
fn incomplete_actions_and_unrelated_uses_leave_target_groups_ungrouped() {
    for case in 0..14 {
        let mut input = forwarding_input();
        match case {
            0 => input["configuration"]["root_module"]["resources"][0]["expressions"]["default_action"] = json!({"references":["aws_lb_target_group.shared.arn"]}),
            1 => input["configuration"]["root_module"]["resources"][0]["expressions"]["default_action"][0]["target_group_arn"] = json!({"references":["aws_lb_target_group.shared.arn", "var.target"]}),
            2 => input["configuration"]["root_module"]["resources"][0]["expressions"]["default_action"].as_array_mut().unwrap().push(json!({"target_group_arn":{"constant_value":"literal"}})),
            3 => input["configuration"]["root_module"]["resources"][0]["expressions"]["default_action"].as_array_mut().unwrap().push(json!({"type":{"constant_value":"forward"}})),
            4 => { input["configuration"]["root_module"]["resources"].as_array_mut().unwrap().remove(1); },
            5 => input["configuration"]["root_module"]["resources"][1]["expressions"]["listener_arn"] = json!({"references":["aws_lb_listener.missing.arn"]}),
            6 => input["resource_changes"][3]["provider_name"] = "example.com/custom/aws".into(),
            7 => input["resource_changes"][3]["mode"] = "data".into(),
            8 => { let mut old = input["resource_changes"][3].clone(); old["deposed"] = "old".into(); input["resource_changes"].as_array_mut().unwrap().push(old); },
            9 => { input["resource_changes"].as_array_mut().unwrap().remove(3); },
            10 => input["configuration"]["root_module"]["resources"][0]["expressions"]["tags"] = json!({"references":["aws_lb_target_group.shared.arn"]}),
            11 => { input["resource_changes"].as_array_mut().unwrap().push(json!({"address":"aws_lb_listener.unknown", "type":"aws_lb_listener"})); },
            12 => input["configuration"]["root_module"]["resources"][0]["expressions"]["default_action"][0]["forward"] = json!({"constant_value":[{"target_group":[{"arn":"literal"}]}]}),
            _ => input["configuration"]["root_module"]["resources"][0]["expressions"]["default_action"][0]["forward"] = json!({"references":["aws_lb_target_group.shared.arn"]}),
        }
        let raw = plan::parse(&input.to_string()).unwrap();
        let graph = semantic::transform(&raw);
        assert!(
            !contains_target(&graph, "aws_lb_target_group.shared"),
            "case {case}"
        );
        assert_eq!(graph.nodes.len(), raw.nodes.len());
    }
}

#[test]
fn exact_instance_targets_work_but_dynamic_selectors_never_choose_an_instance() {
    for (suffix, reference, expected) in [
        ("[0]", "aws_lb_target_group.shared[0].arn", true),
        (
            "[\"blue\"]",
            "aws_lb_target_group.shared[\"blue\"].arn",
            true,
        ),
        ("[0]", "aws_lb_target_group.shared[count.index].arn", false),
        (
            "[\"blue\"]",
            "aws_lb_target_group.shared[each.key].arn",
            false,
        ),
        ("[0]", "aws_lb_target_group.shared.arn", false),
    ] {
        let mut input = forwarding_input();
        let address = format!("aws_lb_target_group.shared{suffix}");
        input["resource_changes"][3]["address"] = address.clone().into();
        input["configuration"]["root_module"]["resources"][0]["expressions"]["default_action"][0]
            ["target_group_arn"] = json!({"references":[reference]});
        input["configuration"]["root_module"]["resources"][1]["expressions"]["action"][0]["forward"]
            [0]["target_group"][0]["arn"] = json!({"references":[reference]});
        let graph = semantic::transform(&plan::parse(&input.to_string()).unwrap());
        assert_eq!(contains_target(&graph, &address), expected, "{reference}");
    }
}

#[test]
fn aliases_must_preserve_exact_target_selection() {
    for (reference, expected) in [
        ("local.target", true),
        ("local.target.arn", false),
        ("local.target[0]", false),
    ] {
        let mut input = forwarding_input();
        input["configuration"]["root_module"]["locals"] =
            json!({"target":{"references":["aws_lb_target_group.shared.arn"]}});
        input["configuration"]["root_module"]["resources"][0]["expressions"]["default_action"][0]
            ["target_group_arn"] = json!({"references":[reference]});
        let graph = semantic::transform(&plan::parse(&input.to_string()).unwrap());
        assert_eq!(
            contains_target(&graph, "aws_lb_target_group.shared"),
            expected,
            "{reference}"
        );
    }
}

#[test]
fn target_attachments_are_compatible_but_do_not_establish_ownership() {
    let mut input = forwarding_input();
    input["configuration"]["provider_config"] = json!({"aws":{"name":"aws","full_name":"registry.terraform.io/hashicorp/aws","expressions":{"region":{"constant_value":"us-east-1"}}}});
    input["resource_changes"][3]["change"]["after"]["target_type"] = json!("instance");
    input["resource_changes"].as_array_mut().unwrap().extend([
        json!({"address":"aws_lb_target_group_attachment.app", "type":"aws_lb_target_group_attachment", "change":{"after":{"target_group_arn":"PRIVATE_TARGET","target_id":"worker"}}}),
        json!({"address":"aws_instance.worker", "type":"aws_instance", "change":{"after":{"id":"worker"}}}),
    ]);
    input["configuration"]["root_module"]["resources"].as_array_mut().unwrap().push(json!({"address":"aws_lb_target_group_attachment.app", "expressions":{
        "target_group_arn":{"references":["aws_lb_target_group.shared.arn"]}, "target_id":{"references":["aws_instance.worker.id"]}
    }}));
    input["configuration"]["root_module"]["resources"]
        .as_array_mut()
        .unwrap()
        .extend([
            json!({"address":"aws_lb_target_group.shared"}),
            json!({"address":"aws_instance.worker"}),
        ]);
    for resource in input["configuration"]["root_module"]["resources"]
        .as_array_mut()
        .unwrap()
    {
        resource["provider_config_key"] = json!("aws");
    }
    let graph = semantic::transform(&plan::parse(&input.to_string()).unwrap());
    assert!(contains_target(&graph, "aws_lb_target_group.shared"));
    assert!(
        graph
            .edges
            .iter()
            .any(|e| e.kind == crate::model::EdgeKind::Association)
    );
    input["configuration"]["root_module"]["resources"]
        .as_array_mut()
        .unwrap()
        .drain(0..2);
    let graph = semantic::transform(&plan::parse(&input.to_string()).unwrap());
    assert!(!contains_target(&graph, "aws_lb_target_group.shared"));
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
fn target_ownership_requires_explicit_keys_along_the_entire_owner_chain() {
    for (resource, field, kind) in [
        (0, "load_balancer_arn", "aws_lb"),
        (1, "listener_arn", "aws_lb_listener"),
    ] {
        for explicit in [false, true] {
            let mut input = forwarding_input();
            let original = if resource == 0 {
                "aws_lb.app"
            } else {
                "aws_lb_listener.https"
            };
            let address = format!("{original}[0]");
            input["resource_changes"][resource]["address"] = address.clone().into();
            input["configuration"]["root_module"]["resources"][if resource == 0 { 0 } else { 1 }]
                ["expressions"][field] = json!({"references":[format!("{}.arn", if explicit { &address } else { original })]});
            let graph = semantic::transform(&plan::parse(&input.to_string()).unwrap());
            assert_eq!(
                contains_target(&graph, "aws_lb_target_group.shared"),
                explicit,
                "{kind}: explicit={explicit}"
            );
        }
    }
}

#[test]
fn rule_forwarding_can_add_distinct_targets_and_filtering_does_not_reinfer_ownership() {
    let mut input = forwarding_input();
    input["resource_changes"]
        .as_array_mut()
        .unwrap()
        .push(json!({"address":"aws_lb_target_group.rule", "type":"aws_lb_target_group", "change":{"after":{"arn":"PRIVATE_RULE_TARGET"}}}));
    input["configuration"]["root_module"]["resources"][1]["expressions"]["action"][0]["forward"][0]
        ["target_group"]
        .as_array_mut()
        .unwrap()
        .push(json!({"arn":{"references":["aws_lb_target_group.rule.arn"]}}));
    input["resource_changes"][2]["change"]["after"]["action"][0]["forward"][0]["target_group"]
        .as_array_mut()
        .unwrap()
        .push(json!({"arn":"PRIVATE_RULE_TARGET"}));
    let plan = plan::parse(&input.to_string()).unwrap();
    let graph = semantic::transform(&plan);
    assert!(contains_target(&graph, "aws_lb_target_group.rule"));
    assert!(contains_target(&graph, "aws_lb_target_group.shared"));
    let selected = crate::view::single(
        &graph,
        &crate::view::Options {
            focus: Some("aws_lb_target_group.shared".into()),
            depth: Some(0),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(selected.nodes.len(), 1);
    assert!(selected.components.is_empty());
    let mut filtered = graph.0.clone();
    filtered
        .nodes
        .retain(|n| n.address != "aws_lb_target_group.shared");
    let components = super::infer(&crate::semantic::Input::new(&plan), &filtered);
    assert!(
        components
            .iter()
            .all(|c| !c.members.iter().any(|m| m == "aws_lb_target_group.shared"))
    );
}

#[test]
fn ownership_and_member_links_remain_local_to_each_named_state() {
    use crate::model::architecture::{Architecture, StateId};
    let mut first = forwarding_input();
    let graph = semantic::transform(&plan::parse(&first.to_string()).unwrap());
    first["configuration"]["root_module"]["resources"][1]["expressions"]["listener_arn"] =
        json!({"references":["aws_lb_listener.missing.arn"]});
    let ungrouped = semantic::transform(&plan::parse(&first.to_string()).unwrap());
    assert!(!contains_target(&ungrouped, "aws_lb_target_group.shared"));
    let all = Architecture {
        states: std::collections::BTreeMap::from([
            (StateId::new("first").unwrap(), graph),
            (StateId::new("second").unwrap(), ungrouped),
        ]),
        relationships: vec![],
    };
    let view = crate::view::apply(&all, &Default::default()).unwrap();
    let svg = crate::svg::render_states(&view, crate::svg::AddressFormat::Qualified);
    let doc = roxmltree::Document::parse(&svg).unwrap();
    let members: Vec<_> = doc
        .descendants()
        .filter(|n| {
            n.has_tag_name("a")
                && n.attribute("data-source-address") == Some("aws_lb_target_group.shared")
        })
        .collect();
    assert_eq!(members.len(), 1);
    let member = members[0];
    let target_id = member.attribute("href").unwrap().strip_prefix('#').unwrap();
    let target = doc
        .descendants()
        .find(|n| n.attribute("id") == Some(target_id))
        .unwrap();
    for node in [member, target] {
        assert_eq!(
            node.ancestors().find_map(|n| n.attribute("data-state-id")),
            Some("first")
        );
    }
    assert_eq!(
        doc.descendants()
            .filter(|n| n.attribute("data-terraform-address") == Some("aws_lb_target_group.shared"))
            .count(),
        2
    );
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
