use crate::{model::*, plan, semantic};
use serde_json::{Value, json};

fn fixture() -> Value {
    serde_json::from_str(include_str!("../../fixtures/waf-plan.json")).unwrap()
}
fn graph(value: &Value) -> ArchitectureGraph {
    semantic::transform(&plan::parse(&value.to_string()).unwrap())
}
fn associations(graph: &Graph) -> usize {
    graph
        .edges
        .iter()
        .filter(|e| e.kind == EdgeKind::Association)
        .count()
}

#[test]
fn waf_scopes_and_verified_associations_preserve_provenance() {
    let value = fixture();
    let graph = graph(&value);
    assert_eq!(graph.coverage.selected.resources, 10);
    assert_eq!(graph.nodes.len(), 8);
    assert_eq!(associations(&graph), 2);
    for edge in graph
        .edges
        .iter()
        .filter(|e| e.kind == EdgeKind::Association)
    {
        let change = edge.change.as_ref().unwrap();
        assert!(change.address.starts_with("aws_wafv2_web_acl_association."));
        assert_eq!(change.action, Action::Create);
    }
    let acl = graph
        .nodes
        .iter()
        .find(|n| n.address == "aws_wafv2_web_acl.cdn")
        .unwrap();
    assert!(acl.entity.scope.as_ref().unwrap().global);
    assert!(acl.entity.scope.as_ref().unwrap().region.is_none());
    let regional = graph
        .nodes
        .iter()
        .find(|n| n.address == "aws_wafv2_web_acl.app")
        .unwrap();
    assert_eq!(
        regional
            .entity
            .scope
            .as_ref()
            .unwrap()
            .region
            .as_ref()
            .unwrap()
            .0,
        "us-west-2"
    );
    assert!(
        regional
            .references
            .iter()
            .any(|r| r.label == "Rule group reference" && r.complete)
    );
    assert!(
        graph
            .nodes
            .iter()
            .find(|n| n.address == "aws_cloudfront_distribution.cdn")
            .unwrap()
            .references
            .iter()
            .any(|r| r.label == "Web ACL reference" && r.complete)
    );
    assert!(
        graph
            .nodes
            .iter()
            .find(|n| n.address == "aws_wafv2_web_acl.unknown")
            .unwrap()
            .entity
            .scope
            .is_none()
    );
    assert!(
        semantic::diagnostics::collect(&plan::parse(&value.to_string()).unwrap())
            .iter()
            .any(|d| d.address == "aws_wafv2_web_acl.unknown" && d.attribute == "scope")
    );
}

#[test]
fn waf_rejects_unproven_endpoints_scope_and_lifecycle() {
    for case in 0..14 {
        let mut value = fixture();
        match case {
            0 => value["resource_changes"][1]["change"]["after"]["scope"] = json!("CLOUDFRONT"),
            1 => value["resource_changes"][1]["change"]["after_unknown"] = json!({"scope":true}),
            2 => {
                value["resource_changes"][3]["change"]["after"]["load_balancer_type"] =
                    json!("network")
            }
            3 => {
                value["resource_changes"][3]["provider_name"] =
                    json!("registry.terraform.io/other/aws")
            }
            4 => value["resource_changes"][3]["type"] = json!("aws_cloudfront_distribution"),
            5 => value["resource_changes"][3]["change"]["after_unknown"] = json!({"arn":true}),
            6 => value["resource_changes"][3]["change"]["after"]["arn"] = json!("different"),
            7 => value["resource_changes"][3]["change"]["actions"] = json!(["delete"]),
            8 => value["resource_changes"][3]["deposed"] = json!("old"),
            9 => value["resource_changes"][3]["mode"] = json!("data"),
            10 => value["resource_changes"][5]["change"]["actions"] = json!(["delete"]),
            11 => {
                value["configuration"]["root_module"]["resources"][5]["expressions"]["resource_arn"] =
                    json!({"references":["aws_lb.app.arn","var.missing"]})
            }
            12 => {
                value["configuration"]["root_module"]["resources"][3]["provider_config_key"] =
                    json!("aws.global")
            }
            _ => {
                value["resource_changes"].as_array_mut().unwrap().remove(3);
            }
        }
        let result = graph(&value);
        assert!(
            result
                .nodes
                .iter()
                .any(|n| n.address == "aws_wafv2_web_acl_association.alb"),
            "case {case}"
        );
        assert!(
            !result.edges.iter().any(|e| e.kind == EdgeKind::Association
                && e.change.as_ref().unwrap().address == "aws_wafv2_web_acl_association.alb"),
            "case {case}"
        );
    }
    for scope in [json!("REGIONAL"), json!("FUTURE"), Value::Null] {
        let mut value = fixture();
        value["resource_changes"][2]["change"]["after"]["scope"] = scope;
        let result = graph(&value);
        assert!(
            !result
                .nodes
                .iter()
                .find(|n| n.address == "aws_cloudfront_distribution.cdn")
                .unwrap()
                .references
                .iter()
                .any(|r| r.label == "Web ACL reference" && r.complete)
        );
    }
}

#[test]
fn waf_rule_groups_require_explicit_singletons_and_matching_scope() {
    for case in 0..5 {
        let mut value = fixture();
        match case {
            0 => value["resource_changes"][0]["change"]["after"]["scope"] = json!("CLOUDFRONT"),
            1 => {
                value["resource_changes"][0]["provider_name"] =
                    json!("registry.terraform.io/other/aws")
            }
            2 => value["resource_changes"][0]["change"]["after_unknown"] = json!({"arn":true}),
            3 => {
                value["configuration"]["root_module"]["resources"][1]["expressions"]["rule"][0]["statement"]
                    [0]["rule_group_reference_statement"][0]["arn"] =
                    json!({"references":["aws_wafv2_rule_group.app[count.index].arn"]})
            }
            _ => {
                let rule = value["resource_changes"][1]["change"]["after"]["rule"][0].clone();
                value["resource_changes"][1]["change"]["after"]["rule"]
                    .as_array_mut()
                    .unwrap()
                    .push(rule);
            }
        }
        let result = graph(&value);
        assert!(
            !result
                .nodes
                .iter()
                .find(|n| n.address == "aws_wafv2_web_acl.app")
                .unwrap()
                .references
                .iter()
                .any(|r| r.label == "Rule group reference" && r.complete),
            "case {case}"
        );
    }
}

#[test]
fn waf_selection_states_and_outputs_keep_identity_without_rule_values() {
    let value = fixture();
    for format in [crate::OutputFormat::Svg, crate::OutputFormat::Html] {
        for icon_mode in [
            crate::icons::IconMode::Icon,
            crate::icons::IconMode::ServiceName,
        ] {
            let text = crate::generate(
                &value.to_string(),
                &crate::Options {
                    format,
                    icon_mode,
                    ..Default::default()
                },
            )
            .unwrap()
            .content;
            assert!(!text.contains("PRIVATE_"));
            assert!(!text.contains("arn:aws:"));
            assert!(
                text.contains("aws_wafv2_web_acl_association.alb")
                    || text.contains("aws_wafv2_web_acl_association")
            );
            assert!(text.contains("AWS WAF") || text.contains("planorama-icon-aws-waf"));
        }
    }
    let graph = graph(&value);
    let selected = crate::view::single(
        &graph,
        &crate::view::Options {
            focus: Some("aws_wafv2_web_acl.app".into()),
            depth: Some(0),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(selected.coverage.selected.resources, 1);
    assert!(selected.edges.is_empty());
    let plans = plan::parse_inputs(
        ["one", "two"]
            .into_iter()
            .map(|name| PlanInput {
                state_id: StateId::new(name).unwrap(),
                plan_json: value.to_string(),
            })
            .collect(),
    )
    .unwrap();
    for (_, parsed) in plans.states {
        let result = semantic::transform(&parsed);
        assert_eq!(result.coverage.selected.resources, 10);
        assert_eq!(associations(&result), 2);
    }
}

#[test]
fn waf_lifecycle_and_modules_retain_association_identity() {
    let mut value = fixture();
    value["resource_changes"][5]["previous_address"] = json!("aws_wafv2_web_acl_association.old");
    value["resource_changes"][5]["change"]["importing"] = json!({"id":"PRIVATE_IMPORT"});
    value["resource_changes"][5]["change"]["actions"] = json!(["delete", "create"]);
    let mut old = value["resource_changes"][2].clone();
    old["deposed"] = json!("old");
    old["change"]["before"] = old["change"]["after"].take();
    old["change"]["actions"] = json!(["delete"]);
    value["resource_changes"].as_array_mut().unwrap().push(old);
    let parsed = plan::parse(&value.to_string()).unwrap();
    assert!(
        !semantic::diagnostics::collect(&parsed)
            .iter()
            .any(|d| d.address == "aws_wafv2_web_acl_association.api"
                && d.reason == DiagnosticReason::EndpointTypeMismatch)
    );
    let result = semantic::transform(&parsed);
    assert_eq!(result.coverage.selected.resources, 11);
    let edge = result
        .edges
        .iter()
        .find(|e| {
            e.change
                .as_ref()
                .is_some_and(|c| c.address == "aws_wafv2_web_acl_association.alb")
        })
        .unwrap();
    let change = edge.change.as_ref().unwrap();
    assert_eq!(change.action, Action::Replace);
    assert!(change.metadata.import.is_some());
    assert!(change.previous_address.is_some());
    let old = result
        .nodes
        .iter()
        .find(|n| n.deposed_key.is_some())
        .unwrap();
    assert!(old.entity.scope.as_ref().unwrap().global);
    assert!(old.references.is_empty());
    let resources = value["configuration"]["root_module"]["resources"].take();
    value["configuration"]["root_module"]["module_calls"] =
        json!({"web":{"module":{"resources":resources}}});
    for node in value["resource_changes"].as_array_mut().unwrap() {
        node["address"] = json!(format!("module.web.{}", node["address"].as_str().unwrap()));
        node["module_address"] = json!("module.web");
    }
    let output = crate::generate(
        &value.to_string(),
        &crate::Options {
            modules: vec!["module.web".into()],
            ..Default::default()
        },
    )
    .unwrap()
    .content;
    assert!(output.contains("11 resources"));
    assert!(!output.contains("PRIVATE_"));
}
