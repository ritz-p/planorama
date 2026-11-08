use crate::{plan, semantic};
use serde_json::{Value, json};
const FIXTURE: &str = include_str!("../../fixtures/global-accelerator-plan.json");
fn complete(v: &Value, label: &str) -> bool {
    semantic::transform(&plan::parse(&v.to_string()).unwrap())
        .nodes
        .iter()
        .flat_map(|n| &n.references)
        .any(|r| r.label == label && r.complete)
}
#[test]
fn standard_hierarchy_keeps_exact_arn_owners_and_independent_endpoint_scopes() {
    let v: Value = serde_json::from_str(FIXTURE).unwrap();
    for (source, target, path, label) in [
        (0, 1, "accelerator_arn", "Listener accelerator"),
        (1, 2, "listener_arn", "Group listener"),
    ] {
        let mut pair = v.clone();
        pair["resource_changes"] =
            json!([v["resource_changes"][source], v["resource_changes"][target]]);
        pair["configuration"]["root_module"]["resources"] = json!([
            v["configuration"]["root_module"]["resources"][source],
            v["configuration"]["root_module"]["resources"][target]
        ]);
        super::super::service_references::tests::check_service_fixture(
            &pair.to_string(),
            label,
            path,
        );
    }
    for label in [
        "Flow log bucket",
        "Configured endpoints",
        "Endpoint cross-account attachment",
    ] {
        assert!(complete(&v, label), "{label}");
    }
    let g = semantic::transform(&plan::parse(FIXTURE).unwrap());
    assert_eq!(g.coverage.selected.resources, 7);
    assert!(
        !g.edges
            .iter()
            .any(|e| e.kind == crate::model::EdgeKind::Containment)
    );
    assert!(
        g.nodes
            .iter()
            .filter(|n| matches!(
                n.resource_type.as_str(),
                "aws_lb" | "aws_eip" | "aws_s3_bucket"
            ))
            .all(|n| !n.entity.scope.as_ref().unwrap().global && n.member_owner.is_none())
    );
}
#[test]
fn endpoint_sets_require_correlated_complete_values_exact_types_and_target_region() {
    for case in [
        "valid",
        "region",
        "lb-type",
        "incomplete",
        "unknown",
        "sensitive",
        "swapped",
        "literal",
        "deleted",
        "custom",
    ] {
        let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
        match case {
            "region" => v["resource_changes"][2]["change"]["after"]["endpoint_group_region"]=json!("us-west-2"),
            "lb-type" => v["resource_changes"][3]["change"]["after"]["load_balancer_type"]=json!("gateway"),
            "incomplete" => v["resource_changes"][2]["change"]["after"]["endpoint_configuration"].as_array_mut().unwrap().push(json!({"endpoint_id":"literal"})),
            "unknown" => v["resource_changes"][2]["change"]["after_unknown"]=json!({"endpoint_configuration":true}),
            "sensitive" => v["resource_changes"][3]["change"]["after_sensitive"]=json!({"arn":true}),
            "swapped" => v["configuration"]["root_module"]["resources"][2]["expressions"]["endpoint_configuration"].as_array_mut().unwrap().swap(0,1),
            "literal" => v["configuration"]["root_module"]["resources"][2]["expressions"]["endpoint_configuration"]=json!([]),
            "deleted" => v["resource_changes"][4]["change"]["actions"]=json!(["delete"]),
            "custom" => v["resource_changes"][3]["type"]=json!("aws_globalaccelerator_custom_routing_endpoint_group"),
            _=>{}
        }
        assert_eq!(
            complete(&v, "Configured endpoints"),
            case == "valid",
            "{case}"
        );
    }
}
#[test]
fn disabled_logs_and_custom_listeners_do_not_acquire_standard_relationships() {
    let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
    v["resource_changes"][0]["change"]["after"]["attributes"][0]["flow_logs_enabled"] =
        json!(false);
    assert!(!complete(&v, "Flow log bucket"));
    v["resource_changes"][1]["type"] = json!("aws_globalaccelerator_custom_routing_listener");
    assert!(!complete(&v, "Group listener"));
}

#[test]
fn accounts_and_named_states_preserve_independent_endpoints() {
    let mut p = plan::parse(FIXTURE).unwrap();
    for n in &mut p.nodes {
        n.provider_configuration.as_mut().unwrap().account =
            crate::model::AwsAccountScope::new("123456789012");
    }
    p.nodes
        .iter_mut()
        .find(|n| n.resource_type == "aws_lb")
        .unwrap()
        .provider_configuration
        .as_mut()
        .unwrap()
        .account = crate::model::AwsAccountScope::new("210987654321");
    let g = semantic::transform(&p);
    assert!(
        !g.nodes
            .iter()
            .flat_map(|n| &n.references)
            .any(|r| r.label == "Configured endpoints" && r.complete)
    );
    let plans = plan::parse_inputs(
        ["one", "two"]
            .into_iter()
            .map(|s| crate::model::PlanInput {
                state_id: crate::model::StateId::new(s).unwrap(),
                plan_json: FIXTURE.into(),
            })
            .collect(),
    )
    .unwrap();
    assert!(semantic::cross_state::resolve(&plans, &[]).edges.is_empty());
    for format in [crate::OutputFormat::Svg, crate::OutputFormat::Html] {
        assert!(
            crate::generate(
                FIXTURE,
                &crate::Options {
                    format,
                    ..Default::default()
                }
            )
            .unwrap()
            .content
            .contains("Configured endpoints")
        );
    }
}
