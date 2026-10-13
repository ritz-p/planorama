use crate::{
    layout::Layout,
    model::{Placement, ResourceRole},
    plan, semantic, svg, view,
};
use serde_json::{Value, json};

fn input() -> Value {
    json!({
        "format_version":"1.2",
        "resource_changes":[
            {"address":"aws_autoscaling_group.app","type":"aws_autoscaling_group","change":{"actions":["create"]}},
            {"address":"aws_autoscaling_group.other","type":"aws_autoscaling_group","change":{"actions":["create"]}},
            {"address":"aws_autoscaling_policy.cpu","type":"aws_autoscaling_policy","change":{"actions":["create"]}},
            {"address":"aws_autoscaling_policy.memory","type":"aws_autoscaling_policy","change":{"actions":["update"]}}
        ],
        "configuration":{"root_module":{"resources":[
            {"address":"aws_autoscaling_policy.cpu","expressions":{"autoscaling_group_name":{"references":["aws_autoscaling_group.app.name"]}}},
            {"address":"aws_autoscaling_policy.memory","expressions":{"autoscaling_group_name":{"references":["aws_autoscaling_group.app.name"]}}}
        ]}}
    })
}

fn render(value: &Value) -> (crate::model::ArchitectureGraph, String) {
    let graph = semantic::transform(&plan::parse(&value.to_string()).unwrap());
    let output = svg::render(&graph, &Layout::new(&graph));
    (graph, output)
}

#[test]
fn policies_are_logical_and_add_verified_asg_context_without_spatial_paths() {
    let (graph, output) = render(&input());
    let policies: Vec<_> = graph
        .nodes
        .iter()
        .filter(|node| node.resource_type == "aws_autoscaling_policy")
        .collect();
    assert_eq!(policies.len(), 2);
    assert!(policies.iter().all(|node| node.placement == Placement::Logical));
    assert!(policies.iter().all(|node| node.role == ResourceRole::Policy));
    assert!(policies.iter().all(|node| node.references.iter().any(|reference| {
        reference.label == "Auto Scaling group"
            && reference.complete
            && reference.targets.len() == 1
            && reference.targets[0].id.address == "aws_autoscaling_group.app"
    })));

    let group = graph
        .nodes
        .iter()
        .find(|node| node.address == "aws_autoscaling_group.app")
        .unwrap();
    let inverse: Vec<_> = group
        .references
        .iter()
        .filter(|reference| reference.label == "Scaling policy")
        .collect();
    assert_eq!(inverse.len(), 2);
    assert_eq!(graph.edges.len(), 2);
    assert!(Layout::new(&graph).paths.iter().all(Vec::is_empty));
    assert!(output.contains("Auto Scaling group: aws_autoscaling_group.app"));
    assert!(output.contains("Scaling policy: aws_autoscaling_policy.cpu"));
    assert!(output.contains("Scaling policy: aws_autoscaling_policy.memory"));
}

#[test]
fn ambiguous_wrong_type_missing_and_cross_provider_targets_stay_conservative() {
    for case in 0..4 {
        let mut value = input();
        match case {
            0 => {
                value["configuration"]["root_module"]["resources"][0]["expressions"]
                    ["autoscaling_group_name"]["references"] =
                    json!(["aws_autoscaling_group.app.name", "aws_autoscaling_group.other.name"]);
            }
            1 => value["resource_changes"][0]["type"] = json!("aws_instance"),
            2 => {
                value["configuration"]["root_module"]["resources"][0]["expressions"]
                    ["autoscaling_group_name"]["references"] =
                    json!(["aws_autoscaling_group.missing.name"]);
            }
            3 => {
                value["configuration"]["provider_config"] = json!({
                    "aws": {
                        "full_name":"registry.terraform.io/hashicorp/aws",
                        "expressions":{"region":{"constant_value":"us-east-1"}}
                    },
                    "aws.other": {
                        "full_name":"registry.terraform.io/hashicorp/aws",
                        "alias":"other",
                        "expressions":{"region":{"constant_value":"us-west-2"}}
                    }
                });
                value["configuration"]["root_module"]["resources"]
                    .as_array_mut()
                    .unwrap()
                    .push(json!({
                        "address":"aws_autoscaling_group.app",
                        "provider_config_key":"aws.other"
                    }));
                value["configuration"]["root_module"]["resources"][0]["provider_config_key"] =
                    json!("aws");
            }
            _ => unreachable!(),
        }
        let (graph, output) = render(&value);
        assert!(!graph.nodes.iter().any(|node| {
            node.references.iter().any(|reference| {
                reference.label == "Scaling policy"
                    && reference
                        .targets
                        .iter()
                        .any(|target| target.id.address == "aws_autoscaling_policy.cpu")
            })
        }));
        assert!(!output.contains("Scaling policy: aws_autoscaling_policy.cpu"));
    }
}

#[test]
fn filtering_keeps_unavailable_context_without_dangling_links() {
    let parsed = plan::parse(&input().to_string()).unwrap();
    let graph = semantic::transform(&parsed);
    for focus in ["aws_autoscaling_group.app", "aws_autoscaling_policy.cpu"] {
        let focused = view::single(
            &graph,
            &view::Options {
                focus: Some(focus.into()),
                depth: Some(0),
                ..Default::default()
            },
        )
        .unwrap();
        let output = svg::render(&focused, &Layout::new(&focused));
        assert!(output.contains("Unavailable"));
        let doc = roxmltree::Document::parse(&output).unwrap();
        for href in doc.descendants().filter_map(|node| node.attribute("href")) {
            assert!(doc
                .descendants()
                .any(|node| node.attribute("id") == href.strip_prefix('#')));
        }
    }
}

#[test]
fn multiple_policies_remain_distinct_and_deterministic_when_input_reorders() {
    let value = input();
    let (_, first) = render(&value);
    let mut reversed = value;
    reversed["resource_changes"].as_array_mut().unwrap().reverse();
    reversed["configuration"]["root_module"]["resources"]
        .as_array_mut()
        .unwrap()
        .reverse();
    let (_, second) = render(&reversed);
    assert_eq!(first, second);
}


#[test]
fn dynamic_instance_selection_does_not_create_verified_inverse_context() {
    let parsed = plan::parse(&input().to_string()).unwrap();
    let mut raw = crate::semantic::Input::new(&parsed);
    let policy = raw
        .nodes
        .iter()
        .position(|node| node.address == "aws_autoscaling_policy.cpu")
        .unwrap();
    let reference = raw
        .attributes
        .iter_mut()
        .find(|reference| {
            reference.target == policy && reference.attribute == "autoscaling_group_name"
        })
        .unwrap();
    reference.complete = true;
    reference
        .issues
        .insert(crate::model::DiagnosticReason::DynamicInstanceSelection);

    let mut graph = crate::semantic::base_graph(&parsed);
    super::present(&raw, &mut graph);

    let group = graph
        .nodes
        .iter()
        .find(|node| node.address == "aws_autoscaling_group.app")
        .unwrap();
    assert!(!group.references.iter().any(|reference| {
        reference.label == "Scaling policy"
            && reference
                .targets
                .iter()
                .any(|target| target.id.address == "aws_autoscaling_policy.cpu")
    }));
    let policy = graph
        .nodes
        .iter()
        .find(|node| node.address == "aws_autoscaling_policy.cpu")
        .unwrap();
    assert!(policy.references.iter().any(|reference| {
        reference.attribute == "autoscaling_group_name" && !reference.complete
    }));
}
