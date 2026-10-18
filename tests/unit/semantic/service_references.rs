use crate::{layout::Layout, plan, semantic, svg};
use serde_json::{Value, json};

#[test]
fn route53_rejects_ambiguous_and_dynamic_instances_and_accepts_nlb() {
    for dynamic in [false, true] {
        let mut value: Value =
            serde_json::from_str(include_str!("../../fixtures/route53-plan.json")).unwrap();
        let mut second = value["resource_changes"][2].clone();
        value["resource_changes"][2]["address"] = json!("aws_lb.web[0]");
        second["address"] = json!("aws_lb.web[1]");
        value["resource_changes"]
            .as_array_mut()
            .unwrap()
            .push(second);
        if dynamic {
            value["configuration"]["root_module"]["resources"][0]["expressions"]["alias"][0]["name"] =
                json!({"references":["aws_lb.web","var.index"]});
        }
        let parsed = plan::parse(&value.to_string()).unwrap();
        let graph = semantic::transform(&parsed);
        assert!(
            !graph
                .nodes
                .iter()
                .flat_map(|n| &n.references)
                .any(|r| r.label == "Alias target" && r.complete)
        );
        assert!(!semantic::diagnostics::collect(&parsed).is_empty());
    }
    let input = include_str!("../../fixtures/route53-plan.json").replace("application", "network");
    let graph = semantic::transform(&plan::parse(&input).unwrap());
    assert!(
        graph
            .nodes
            .iter()
            .flat_map(|n| &n.references)
            .any(|r| r.label == "Alias target" && r.complete)
    );
}

#[test]
fn service_links_keep_state_lifecycle_and_coverage_identity() {
    let fixture = include_str!("../../fixtures/route53-plan.json");
    let plans = plan::parse_inputs(
        ["first", "second"]
            .into_iter()
            .map(|state| crate::model::PlanInput {
                state_id: crate::model::StateId::new(state).unwrap(),
                plan_json: fixture.into(),
            })
            .collect(),
    )
    .unwrap();
    for plan in plans.states.values() {
        let graph = semantic::transform(plan);
        assert_eq!(graph.coverage.input.resources, 5);
        assert_eq!(graph.coverage.selected.resources, 5);
        assert!(
            graph
                .nodes
                .iter()
                .flat_map(|n| &n.references)
                .flat_map(|r| &r.targets)
                .all(|t| t.state.is_none())
        );
    }
    let mut value: Value = serde_json::from_str(fixture).unwrap();
    value["resource_changes"][1]["previous_address"] = json!("aws_route53_record.old");
    value["resource_changes"][1]["change"]["importing"] = json!({"id":"PRIVATE_IMPORT"});
    let mut old = value["resource_changes"][1].clone();
    old["deposed"] = json!("previous");
    old["change"]["actions"] = json!(["delete"]);
    value["resource_changes"].as_array_mut().unwrap().push(old);
    let graph = semantic::transform(&plan::parse(&value.to_string()).unwrap());
    assert_eq!(graph.coverage.selected.resources, 6);
    let current = graph
        .nodes
        .iter()
        .find(|n| n.address == "aws_route53_record.alias" && n.deposed_key.is_none())
        .unwrap();
    assert_eq!(
        current.previous_address.as_deref(),
        Some("aws_route53_record.old")
    );
    assert!(
        graph
            .nodes
            .iter()
            .filter(|n| n.deposed_key.is_some())
            .all(|n| !n
                .references
                .iter()
                .any(|r| r.label == "Alias target" && r.complete))
    );
    assert!(!svg::render(&graph, &Layout::new(&graph)).contains("PRIVATE_IMPORT"));
}

#[test]
fn route53_indexed_identity_filtering_and_cloudfront() {
    let input =
        include_str!("../../fixtures/route53-plan.json").replace("aws_lb.web", "aws_lb.web[0]");
    let parsed = plan::parse(&input).unwrap();
    let graph = semantic::transform(&parsed);
    assert!(
        graph
            .nodes
            .iter()
            .flat_map(|n| &n.references)
            .any(|r| r.label == "Alias target" && r.complete)
    );
    let filtered = crate::view::single(
        &graph,
        &crate::view::Options {
            focus: Some("aws_route53_record.alias".into()),
            depth: Some(0),
            ..Default::default()
        },
    )
    .unwrap();
    let output = svg::render(&filtered, &Layout::new(&filtered));
    assert!(output.contains("Unavailable"));
    assert!(!output.contains("href=\"#aws_lb.web"));
    let mut value: Value =
        serde_json::from_str(include_str!("../../fixtures/route53-plan.json")).unwrap();
    value["resource_changes"][2]["type"] = json!("aws_cloudfront_distribution");
    let text = value
        .to_string()
        .replace("aws_lb.web", "aws_cloudfront_distribution.web")
        .replace("dns_name", "domain_name");
    let mut value: Value = serde_json::from_str(&text).unwrap();
    value["resource_changes"][2]["change"]["after"]["hosted_zone_id"] = json!("Z-LB");
    value["configuration"]["root_module"]["resources"][0]["expressions"]["alias"][0]["zone_id"] =
        json!({"references":["aws_cloudfront_distribution.web.hosted_zone_id"]});
    let graph = semantic::transform(&plan::parse(&value.to_string()).unwrap());
    assert!(
        graph
            .nodes
            .iter()
            .flat_map(|n| &n.references)
            .any(|r| r.label == "Alias target" && r.complete)
    );
}

#[test]
fn route53_requires_complete_current_typed_attribute_and_value_evidence() {
    for case in 0..13 {
        let mut value: Value =
            serde_json::from_str(include_str!("../../fixtures/route53-plan.json")).unwrap();
        match case {
            1 => value["resource_changes"][2]["provider_name"] = json!("example.com/foreign/aws"),
            2 => value["resource_changes"][2]["change"]["after_unknown"] = json!(true),
            3 => {
                value["resource_changes"][2]["change"]["after"]["load_balancer_type"] =
                    json!("gateway")
            }
            4 => {
                value["resource_changes"][1]["change"]["after"]["alias"][0]["zone_id"] =
                    json!("different")
            }
            5 => {
                value["configuration"]["root_module"]["resources"][0]["expressions"]["alias"][0]["name"] =
                    json!({"references":["aws_lb.web.tags"]})
            }
            6 => value["resource_changes"][1]["change"]["actions"] = json!(["delete"]),
            7 => value["resource_changes"][2]["deposed"] = json!("old"),
            8 => {
                value["configuration"]["root_module"]["resources"][0]["expressions"]["alias"][0]["name"] =
                    json!({"references":["aws_lb.web.dns_name","aws_lb.missing.dns_name"]})
            }
            9 => value["resource_changes"][2]["mode"] = json!("data"),
            10 => {
                value["configuration"]["root_module"]["resources"][0]["expressions"]["alias"][0]["name"] =
                    json!({"constant_value":"synthetic.invalid"})
            }
            11 => value["resource_changes"][1]["change"]["after_unknown"] = json!({"alias":true}),
            12 => value["resource_changes"][2]["change"]["actions"] = json!(["delete"]),
            _ => {}
        }
        let parsed = plan::parse(&value.to_string()).unwrap();
        let graph = semantic::transform(&parsed);
        assert_eq!(graph.nodes.len(), parsed.nodes.len());
        let alias = graph
            .nodes
            .iter()
            .find(|n| n.address == "aws_route53_record.alias")
            .unwrap();
        assert_eq!(
            alias
                .references
                .iter()
                .any(|r| r.label == "Alias target" && r.complete),
            case == 0,
            "case {case}"
        );
        assert_eq!(graph.edges.len(), parsed.edges.len());
        let output = svg::render(&graph, &Layout::new(&graph));
        assert!(!output.contains("synthetic.invalid"));
        assert!(!output.contains("Z-SYNTHETIC"));
    }
}
