use crate::{
    model::{EdgeKind, Placement, ResourceRole},
    plan, semantic,
};
use serde_json::{Value, json};

const FIXTURE: &str = include_str!("../../fixtures/identity-providers-plan.json");

#[test]
fn identity_provider_module_and_explicit_issuer_references_remain_generic() {
    let mut value: Value = serde_json::from_str(FIXTURE).unwrap();
    value["configuration"]["root_module"]["resources"][0]["expressions"] = json!({
        "url":{"references":["aws_eks_cluster.literal.identity.0.oidc.0.issuer", "aws_eks_cluster.literal"]}
    });
    for resource in value["resource_changes"].as_array_mut().unwrap() {
        resource["address"] = json!(format!(
            "module.identity.{}",
            resource["address"].as_str().unwrap()
        ));
    }
    let resources = value["configuration"]["root_module"]["resources"].take();
    value["configuration"]["root_module"] =
        json!({"module_calls":{"identity":{"module":{"resources":resources}}}});
    let graph = semantic::transform(&plan::parse(&value.to_string()).unwrap());
    let selected = crate::view::single(
        &graph,
        &crate::view::Options {
            modules: vec!["module.identity".into()],
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(selected.nodes.len(), 6);
    assert_eq!(selected.edges.len(), 3);
    assert!(
        selected
            .edges
            .iter()
            .all(|edge| edge.kind == EdgeKind::Dependency)
    );
    let provider = selected
        .nodes
        .iter()
        .find(|n| n.resource_type == "aws_iam_openid_connect_provider")
        .unwrap();
    assert!(
        provider
            .references
            .iter()
            .any(|r| r.label == "References" && r.attribute == "url" && r.complete)
    );
}

#[test]
fn identity_providers_are_global_logical_with_only_configuration_references() {
    let parsed = plan::parse(FIXTURE).unwrap();
    let graph = semantic::transform(&parsed);
    assert_eq!(graph.nodes.len(), 6);
    assert_eq!(graph.coverage.selected.resources, 6);
    assert_eq!(graph.edges.len(), 2);
    assert!(
        graph
            .edges
            .iter()
            .all(|edge| edge.kind == EdgeKind::Dependency)
    );
    assert!(
        parsed
            .attributes
            .iter()
            .all(|r| !r.service_reference_verified)
    );
    for node in graph
        .nodes
        .iter()
        .filter(|n| n.resource_type.ends_with("_provider"))
    {
        assert_eq!(node.role, ResourceRole::Policy);
        assert_eq!(node.placement, Placement::Logical);
        assert!(
            node.entity
                .scope
                .as_ref()
                .is_some_and(|s| s.global && s.region.is_none())
        );
    }
    for node in graph
        .nodes
        .iter()
        .filter(|n| n.resource_type == "aws_iam_role" && !n.address.ends_with("literal"))
    {
        assert!(
            node.references
                .iter()
                .any(|r| r.label == "References" && r.complete && r.targets.len() == 1)
        );
    }
    for format in [crate::OutputFormat::Svg, crate::OutputFormat::Html] {
        let output = crate::generate(
            FIXTURE,
            &crate::Options {
                format,
                ..Default::default()
            },
        )
        .unwrap();
        assert!(!output.content.contains("PRIVATE_"));
        assert!(
            output
                .content
                .contains("aws_iam_openid_connect_provider.workload")
        );
        assert!(output.content.contains("aws_iam_saml_provider.workforce"));
    }
}

#[test]
fn identity_provider_reference_failures_do_not_infer_federation() {
    for case in 0..8 {
        let mut value: Value = serde_json::from_str(FIXTURE).unwrap();
        match case {
            0 => value["resource_changes"][0]["provider_name"] = json!("example.com/foreign/aws"),
            1 => {
                value["resource_changes"][0]["type"] =
                    json!("aws_iam_openid_connect_provider_future")
            }
            2 => value["resource_changes"][0]["change"]["after_unknown"] = json!(true),
            3 => {
                value["configuration"]["root_module"]["resources"][1]["expressions"]["assume_role_policy"] =
                    json!({"references":["var.dynamic"]})
            }
            4 => {
                value["configuration"]["root_module"]["resources"][1]["expressions"]["assume_role_policy"] =
                    json!({"references":["aws_iam_openid_connect_provider.missing.arn"]})
            }
            5 => {
                let mut second = value["resource_changes"][0].clone();
                value["resource_changes"][0]["address"] =
                    json!("aws_iam_openid_connect_provider.workload[0]");
                second["address"] = json!("aws_iam_openid_connect_provider.workload[1]");
                value["resource_changes"]
                    .as_array_mut()
                    .unwrap()
                    .push(second);
            }
            6 => value["resource_changes"][0]["deposed"] = json!("old"),
            _ => value["resource_changes"][0]["change"]["actions"] = json!(["delete"]),
        }
        let parsed = plan::parse(&value.to_string()).unwrap();
        let graph = semantic::transform(&parsed);
        assert!(
            graph
                .edges
                .iter()
                .all(|edge| edge.kind == EdgeKind::Dependency)
        );
        assert_eq!(
            graph.coverage.selected.resources,
            value["resource_changes"].as_array().unwrap().len()
        );
        if case < 2 {
            assert_eq!(
                graph
                    .nodes
                    .iter()
                    .find(|n| n.address == "aws_iam_openid_connect_provider.workload")
                    .unwrap()
                    .role,
                ResourceRole::Unknown
            );
        }
        if (3..=5).contains(&case) {
            let role = graph
                .nodes
                .iter()
                .find(|n| n.address == "aws_iam_role.workload")
                .unwrap();
            assert!(
                role.references
                    .iter()
                    .all(|r| !r.complete || !r.issues.is_empty())
            );
        }
        assert!(!format!("{parsed:?}").contains("PRIVATE_"));
    }
}

#[test]
fn identity_providers_keep_indexed_data_deposed_state_and_selection_identity() {
    let indexed = FIXTURE.replace(
        "aws_iam_openid_connect_provider.workload",
        "aws_iam_openid_connect_provider.workload[0]",
    );
    let graph = semantic::transform(&plan::parse(&indexed).unwrap());
    assert_eq!(graph.edges.len(), 2);
    let filtered = crate::view::single(
        &graph,
        &crate::view::Options {
            focus: Some("aws_iam_role.workload".into()),
            depth: Some(0),
            ..Default::default()
        },
    )
    .unwrap();
    assert!(
        crate::svg::render(&filtered, &crate::layout::Layout::new(&filtered))
            .contains("Unavailable")
    );
    let mut value: Value = serde_json::from_str(FIXTURE).unwrap();
    let mut old = value["resource_changes"][0].clone();
    old["deposed"] = json!("old");
    old["change"]["actions"] = json!(["delete"]);
    value["resource_changes"][0]["previous_address"] = json!("aws_iam_openid_connect_provider.old");
    value["resource_changes"][0]["change"]["importing"] = json!({"id":"PRIVATE_IMPORT"});
    let mut data = value["resource_changes"][2].clone();
    data["address"] = json!("data.aws_iam_saml_provider.workforce");
    data["mode"] = json!("data");
    data["change"]["actions"] = json!(["read"]);
    value["resource_changes"]
        .as_array_mut()
        .unwrap()
        .extend([old, data]);
    let graph = semantic::transform(&plan::parse(&value.to_string()).unwrap());
    assert_eq!(graph.coverage.selected.resources, 8);
    assert!(
        graph
            .nodes
            .iter()
            .any(|n| n.mode == crate::model::EntityMode::Data && n.placement == Placement::Logical)
    );
    assert!(
        graph
            .nodes
            .iter()
            .any(|n| n.deposed_key.is_some() && n.action == crate::model::Action::Delete)
    );
    let plans = plan::parse_inputs(
        ["one", "two"]
            .into_iter()
            .map(|state| crate::model::PlanInput {
                state_id: crate::model::StateId::new(state).unwrap(),
                plan_json: value.to_string(),
            })
            .collect(),
    )
    .unwrap();
    assert!(semantic::cross_state::resolve(&plans, &[]).edges.is_empty());
    for parsed in plans.states.values() {
        assert_eq!(semantic::transform(parsed).coverage.selected.resources, 8);
    }
}
