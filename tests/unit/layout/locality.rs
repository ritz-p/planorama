use super::*;
use crate::{plan, semantic};

#[path = "locality_leaves.rs"]
mod leaf_tests;

fn fixture() -> Graph {
    semantic::transform(
        &plan::parse(include_str!("../../../examples/terraform-large/plan.json")).unwrap(),
    )
    .0
}

#[test]
fn listener_owner_resolution_is_conservative() {
    let graph = fixture();
    let pairs = crate::provider::aws::owner_affinities(&graph);
    assert_eq!(pairs.len(), 1);
    let (listener, owner) = pairs[0];
    assert_eq!(graph.nodes[owner].resource_type, "aws_lb");
    for case in 0..6 {
        let mut graph = graph.clone();
        let reference = graph.nodes[listener]
            .references
            .iter_mut()
            .find(|r| r.attribute == "load_balancer_arn")
            .unwrap();
        match case {
            0 => reference.complete = false,
            1 => {
                reference
                    .issues
                    .insert(crate::model::DiagnosticReason::DynamicInstanceSelection);
            }
            2 => reference.targets.clear(),
            3 => reference.targets.push(reference.targets[0].clone()),
            4 => {
                let reference = reference.clone();
                graph.nodes[listener].references.push(reference);
            }
            _ => graph.nodes[owner].resource_type = "aws_lb_target_group".into(),
        }
        assert!(
            crate::provider::aws::owner_affinities(&graph).is_empty(),
            "case {case}"
        );
    }
}

#[test]
fn multiple_owners_and_shared_targets_keep_stable_separate_cards() {
    use serde_json::json;
    let mut input = json!({"format_version": "1.2", "resource_changes": [], "configuration": {"root_module": {"resources": []}}});
    for (address, kind) in [
        ("aws_lb.a", "aws_lb"),
        ("aws_lb.z", "aws_lb"),
        ("aws_lb_target_group.shared", "aws_lb_target_group"),
    ] {
        input["resource_changes"]
            .as_array_mut()
            .unwrap()
            .push(json!({"address": address, "type": kind, "change": {"actions": ["create"]}}));
    }
    for (listener, owner) in [("a1", "a"), ("a2", "a"), ("z1", "z"), ("z2", "z")] {
        let address = format!("aws_lb_listener.{listener}");
        input["resource_changes"].as_array_mut().unwrap().push(json!({"address": address, "type": "aws_lb_listener", "change": {"actions": ["create"]}}));
        input["configuration"]["root_module"]["resources"].as_array_mut().unwrap().push(json!({"address": address, "expressions": {
            "load_balancer_arn": {"references": [format!("aws_lb.{owner}.arn")]},
            "default_action": [{"target_group_arn": {"references": ["aws_lb_target_group.shared.arn"]}}]
        }}));
    }
    let graph = semantic::transform(&plan::parse(&input.to_string()).unwrap()).0;
    let pairs = crate::provider::aws::owner_affinities(&graph);
    assert_eq!(pairs.len(), 4);
    let layout = Layout::new(&graph);
    for &(listener, owner) in &pairs {
        assert!(
            graph.nodes[listener]
                .address
                .ends_with(if graph.nodes[owner].address.ends_with('a') {
                    "a1"
                } else {
                    "z1"
                })
                || graph.nodes[listener].address.ends_with(
                    if graph.nodes[owner].address.ends_with('a') {
                        "a2"
                    } else {
                        "z2"
                    }
                )
        );
        assert_eq!(layout.containment.parents[listener], None);
        for &(_, other_owner) in &pairs {
            assert!(
                distance(layout.bounds[listener], layout.bounds[owner])
                    <= distance(layout.bounds[listener], layout.bounds[other_owner]),
                "{} is closer to another owner",
                graph.nodes[listener].address
            );
        }
        for (other, &bounds) in layout.bounds.iter().enumerate() {
            if listener != other {
                assert!(separated(layout.bounds[listener], bounds, 0));
            }
        }
    }
    let snapshot = |graph: &Graph| {
        let layout = Layout::new(graph);
        graph
            .nodes
            .iter()
            .zip(layout.bounds)
            .map(|(n, b)| (n.address.clone(), b))
            .collect::<std::collections::BTreeMap<_, _>>()
    };
    input["resource_changes"].as_array_mut().unwrap().reverse();
    input["configuration"]["root_module"]["resources"]
        .as_array_mut()
        .unwrap()
        .reverse();
    let reordered = semantic::transform(&plan::parse(&input.to_string()).unwrap()).0;
    assert_eq!(snapshot(&graph), snapshot(&reordered));
}

#[test]
#[ignore = "deferred regression: three large-fixture locality layouts"]
fn deferred_regression_large_listener_locality_preserves_geometry_and_routes() {
    let graph = fixture();
    let (listener, owner) = crate::provider::aws::owner_affinities(&graph)[0];
    let mut baseline = graph.clone();
    baseline.nodes[listener]
        .references
        .retain(|r| r.attribute != "load_balancer_arn");
    let start = std::time::Instant::now();
    let old = Layout::new(&baseline);
    let old_ms = start.elapsed().as_millis();
    let start = std::time::Instant::now();
    let new = Layout::new(&graph);
    eprintln!(
        "owner locality: distance {} -> {}; area {} -> {}; routes {:?} -> {:?}; runtime {} -> {} ms",
        distance(old.bounds[listener], old.bounds[owner]),
        distance(new.bounds[listener], new.bounds[owner]),
        old.width * old.height,
        new.width * new.height,
        super::super::metrics::measure(&old.paths),
        super::super::metrics::measure(&new.paths),
        old_ms,
        start.elapsed().as_millis()
    );
    assert!(
        distance(new.bounds[listener], new.bounds[owner]) * 4
            < distance(old.bounds[listener], old.bounds[owner]) * 3
    );
    assert_eq!(old.containment.parents, new.containment.parents);
    assert!(new.width * new.height <= old.width * old.height * 11 / 10);
    for (other, &bounds) in new.bounds.iter().enumerate() {
        if other != listener && bounds.width > 0 && !new.containment.is_ancestor(other, listener) {
            assert!(
                separated(
                    new.bounds[listener],
                    bounds,
                    super::super::relationship_markers::padding(
                        &crate::view::spatial::project(&graph).graph
                    )
                ),
                "overlap with {}: {:?} {:?}",
                graph.nodes[other].address,
                new.bounds[listener],
                bounds
            );
        }
    }
    assert_eq!(graph.edges.len(), new.paths.len());
    assert_eq!(new.bounds, Layout::new(&graph).bounds);
}
