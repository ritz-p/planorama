use crate::{model::*, plan, semantic};
use serde_json::{Value, json};

fn fixture() -> Value {
    serde_json::from_str(include_str!("../../fixtures/cloudfront-plan.json")).unwrap()
}

fn graph(value: &Value) -> ArchitectureGraph {
    semantic::transform(&plan::parse(&value.to_string()).unwrap())
}

fn origin(graph: &Graph) -> &ResourceReference {
    graph
        .nodes
        .iter()
        .find(|n| n.address == "aws_cloudfront_distribution.assets")
        .unwrap()
        .references
        .iter()
        .find(|r| r.label == "Origin reference")
        .unwrap()
}

#[test]
fn cloudfront_preserves_resources_and_distinguishes_verified_references() {
    let graph = graph(&fixture());
    assert_eq!(graph.coverage.selected.resources, 11);
    assert_eq!(graph.nodes.len(), 11);
    assert!(origin(&graph).complete);
    assert_eq!(
        origin(&graph).targets[0].id.address,
        "aws_s3_bucket.assets[0]"
    );
    for node in &graph.nodes {
        if node.resource_type.starts_with("aws_cloudfront_") {
            assert!(node.entity.scope.as_ref().unwrap().global);
            if node.resource_type != "aws_cloudfront_distribution" {
                assert_eq!(node.placement, Placement::Logical);
                assert_eq!(node.role, ResourceRole::Policy);
            }
        } else {
            assert_eq!(
                node.entity
                    .scope
                    .as_ref()
                    .unwrap()
                    .region
                    .as_ref()
                    .unwrap()
                    .0,
                "us-west-2"
            );
        }
    }
    assert_eq!(
        graph
            .nodes
            .iter()
            .flat_map(|n| &n.references)
            .filter(|r| r.complete && r.label != "References")
            .count(),
        11
    );
    let custom = graph
        .nodes
        .iter()
        .find(|n| n.address.ends_with(".custom"))
        .unwrap();
    assert!(
        custom
            .references
            .iter()
            .any(|r| r.label == "Origin reference" && !r.complete)
    );
}

#[test]
fn cloudfront_requires_complete_current_typed_and_evaluated_endpoints() {
    for case in 0..14 {
        let mut value = fixture();
        match case {
            0 => {
                value["resource_changes"][0]["provider_name"] =
                    json!("registry.terraform.io/other/aws")
            }
            1 => value["resource_changes"][0]["type"] = json!("aws_sqs_queue"),
            2 => {
                value["resource_changes"][0]["change"]["after_unknown"] =
                    json!({"bucket_regional_domain_name":true})
            }
            3 => {
                value["resource_changes"][0]["change"]["after"]["bucket_regional_domain_name"] =
                    json!("different")
            }
            4 => value["resource_changes"][0]["mode"] = json!("data"),
            5 => value["resource_changes"][0]["change"]["actions"] = json!(["delete"]),
            6 => value["resource_changes"][0]["deposed"] = json!("old"),
            7 => value["resource_changes"][7]["change"]["actions"] = json!(["delete"]),
            8 => value["resource_changes"][7]["change"]["after_unknown"] = json!({"origin":true}),
            9 => {
                value["configuration"]["root_module"]["resources"][7]["expressions"]["origin"][0]
                    ["domain_name"] = json!({"references":["aws_s3_bucket.assets[0].bucket_regional_domain_name","var.missing"]})
            }
            10 => {
                value["configuration"]["root_module"]["resources"][7]["expressions"]["origin"][0]
                    ["domain_name"] = json!({"references":["aws_s3_bucket.assets[count.index].bucket_regional_domain_name"]})
            }
            11 => {
                let block = value["resource_changes"][7]["change"]["after"]["origin"][0].clone();
                value["resource_changes"][7]["change"]["after"]["origin"]
                    .as_array_mut()
                    .unwrap()
                    .push(block);
            }
            12 => {
                value["resource_changes"].as_array_mut().unwrap().remove(0);
            }
            _ => {
                value["resource_changes"][7]["provider_name"] =
                    json!("registry.terraform.io/other/aws")
            }
        }
        let parsed = plan::parse(&value.to_string()).unwrap();
        let result = semantic::transform(&parsed);
        if case == 13 {
            assert!(
                !result
                    .nodes
                    .iter()
                    .find(|n| n.address.ends_with(".assets")
                        && n.resource_type == "aws_cloudfront_distribution")
                    .unwrap()
                    .references
                    .iter()
                    .any(|r| r.label == "Origin reference")
            );
        } else {
            assert!(!origin(&result).complete, "case {case}");
            assert!(
                semantic::diagnostics::collect(&parsed)
                    .iter()
                    .any(|d| d.attribute == "origin.domain_name"
                        && d.reason == DiagnosticReason::UnprovenEndpointValue)
            );
        }
    }
    let mut value = fixture();
    value["resource_changes"][1]["change"]["after"]["load_balancer_type"] = json!("network");
    assert!(
        graph(&value)
            .nodes
            .iter()
            .find(|n| n.address == "aws_cloudfront_distribution.app")
            .unwrap()
            .references
            .iter()
            .any(|r| r.label == "Origin reference" && !r.complete)
    );
}

#[test]
fn cloudfront_selection_states_lifecycle_and_output_privacy() {
    let mut value = fixture();
    value["resource_changes"][7]["previous_address"] = json!("aws_cloudfront_distribution.old");
    value["resource_changes"][7]["change"]["importing"] = json!({"id":"PRIVATE_IMPORT"});
    value["resource_changes"][7]["change"]["actions"] = json!(["delete", "create"]);
    let mut old = value["resource_changes"][7].clone();
    old["deposed"] = json!("old");
    old["change"]["before"] = old["change"]["after"].take();
    old["change"]["actions"] = json!(["delete"]);
    value["resource_changes"].as_array_mut().unwrap().push(old);
    let result = graph(&value);
    assert_eq!(result.coverage.selected.resources, 12);
    assert!(
        result
            .nodes
            .iter()
            .filter(|n| n.deposed_key.is_some())
            .all(|n| n.references.is_empty())
    );
    let current = result
        .nodes
        .iter()
        .find(|n| n.address == "aws_cloudfront_distribution.assets" && n.deposed_key.is_none())
        .unwrap();
    assert_eq!(current.action, Action::Replace);
    assert!(current.metadata.import.is_some());
    assert!(current.previous_address.is_some());
    for format in [crate::OutputFormat::Svg, crate::OutputFormat::Html] {
        let output = crate::generate(
            &value.to_string(),
            &crate::Options {
                format,
                ..Default::default()
            },
        )
        .unwrap()
        .content;
        assert!(!output.contains("PRIVATE_"));
        assert!(!output.contains("amazonaws.com"));
        assert!(output.contains("Origin reference"));
    }
    let selected = crate::view::single(
        &graph(&fixture()),
        &crate::view::Options {
            focus: Some("aws_cloudfront_distribution.assets".into()),
            depth: Some(0),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(selected.coverage.selected.resources, 1);
    assert!(
        crate::svg::render(&selected, &crate::layout::Layout::new(&selected))
            .contains("Unavailable")
    );
    let mut partial = fixture();
    partial["resource_changes"]
        .as_array_mut()
        .unwrap()
        .remove(0);
    let states = plan::parse_inputs(
        [("one", fixture()), ("two", partial)]
            .into_iter()
            .map(|(name, value)| PlanInput {
                state_id: StateId::new(name).unwrap(),
                plan_json: value.to_string(),
            })
            .collect(),
    )
    .unwrap();
    for (state, parsed) in states.states {
        assert_eq!(
            origin(&semantic::transform(&parsed)).complete,
            state.as_str() == "one"
        );
    }
}

#[test]
fn cloudfront_module_selection_and_real_plan_reference_prefixes() {
    let mut value = fixture();
    value["configuration"]["root_module"]["resources"][7]["expressions"]["origin"][0]["origin_access_control_id"] = json!({"references":[
        "aws_cloudfront_origin_access_control.assets.id",
        "aws_cloudfront_origin_access_control.assets"
    ]});
    let resources = value["configuration"]["root_module"]["resources"].take();
    value["configuration"]["root_module"]["module_calls"] =
        json!({"cdn":{"module":{"resources":resources}}});
    for node in value["resource_changes"].as_array_mut().unwrap() {
        node["address"] = json!(format!("module.cdn.{}", node["address"].as_str().unwrap()));
        node["module_address"] = json!("module.cdn");
    }
    let output = crate::generate(
        &value.to_string(),
        &crate::Options {
            modules: vec!["module.cdn".into()],
            ..Default::default()
        },
    )
    .unwrap()
    .content;
    assert!(output.contains("11 resources"));
    assert!(output.contains("Origin access control reference"));
    let result = graph(&value);
    assert!(
        result
            .nodes
            .iter()
            .find(|n| n.address == "module.cdn.aws_cloudfront_distribution.assets")
            .unwrap()
            .references
            .iter()
            .any(|r| r.label == "Origin access control reference" && r.complete)
    );
}

#[test]
fn cloudfront_access_and_policy_references_reject_unproven_outputs() {
    for (endpoint, consumer, attribute, field) in [
        (2, "assets", "origin.origin_access_control_id", "id"),
        (
            3,
            "legacy",
            "origin.s3_origin_config.origin_access_identity",
            "cloudfront_access_identity_path",
        ),
        (4, "assets", "default_cache_behavior.cache_policy_id", "id"),
        (
            5,
            "assets",
            "default_cache_behavior.origin_request_policy_id",
            "id",
        ),
        (
            6,
            "assets",
            "default_cache_behavior.response_headers_policy_id",
            "id",
        ),
        (4, "app", "ordered_cache_behavior.cache_policy_id", "id"),
        (
            5,
            "app",
            "ordered_cache_behavior.origin_request_policy_id",
            "id",
        ),
        (
            6,
            "app",
            "ordered_cache_behavior.response_headers_policy_id",
            "id",
        ),
    ] {
        for case in 0..4 {
            let mut value = fixture();
            match case {
                0 => {
                    value["resource_changes"][endpoint]["change"]["after"][field] =
                        json!("different")
                }
                1 => {
                    value["resource_changes"][endpoint]["change"]["after_unknown"] =
                        json!({field:true})
                }
                2 => {
                    value["resource_changes"][endpoint]["provider_name"] =
                        json!("registry.terraform.io/other/aws")
                }
                _ => value["resource_changes"][endpoint]["change"]["actions"] = json!(["delete"]),
            }
            let graph = graph(&value);
            let node = graph
                .nodes
                .iter()
                .find(|n| n.address == format!("aws_cloudfront_distribution.{consumer}"))
                .unwrap();
            assert!(
                !node
                    .references
                    .iter()
                    .any(|r| r.attribute == attribute && r.label != "References" && r.complete),
                "{attribute} case {case}"
            );
        }
    }
}
