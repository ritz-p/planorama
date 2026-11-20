use super::label;
use crate::configured_service_links::{check, check_provider_alias, verified};
use serde_json::{Value, json};

const PROVIDER: &str = "registry.terraform.io/hashicorp/aws";

fn plan(resources: &[(&str, Value, Value)]) -> Value {
    let changes: Vec<_> = resources
        .iter()
        .map(|(address, values, _)| {
            let (kind, name) = address.split_once('.').unwrap();
            json!({
                "address": address, "mode": "managed", "type": kind, "name": name,
                "provider_name": PROVIDER, "change": {"actions": ["create"], "after": values}
            })
        })
        .collect();
    let config: Vec<_> = resources
        .iter()
        .map(|(address, _, expressions)| {
            let (kind, name) = address.split_once('.').unwrap();
            json!({
                "address": address, "mode": "managed", "type": kind, "name": name,
                "provider_config_key": "aws", "expressions": expressions
            })
        })
        .collect();
    json!({
        "format_version": "1.2", "terraform_version": "1.9.8", "resource_changes": changes,
        "configuration": {
            "provider_config": {"aws": {"name": "aws", "full_name": PROVIDER, "expressions": {
                "region": {"constant_value": "us-east-1"},
                "assume_role": [{"role_arn": {"constant_value": "arn:aws:iam::123456789012:role/Provider"}}]
            }}},
            "root_module": {"resources": config}
        }
    })
}

fn graph(plan: &Value) -> crate::model::ArchitectureGraph {
    crate::semantic::transform(&crate::plan::parse(&plan.to_string()).unwrap())
}

fn amplify() -> Value {
    let service = "arn:aws:iam::123456789012:role/Build";
    let compute = "arn:aws:iam::123456789012:role/Compute";
    let backend = "arn:aws:amplify:us-east-1:123456789012:apps/d123/backendenvironments/dev";
    plan(&[
        ("aws_iam_role.build", json!({"arn": service}), json!({})),
        ("aws_iam_role.compute", json!({"arn": compute}), json!({})),
        (
            "aws_amplify_app.site",
            json!({"id":"d123", "iam_service_role_arn":service, "compute_role_arn":compute}),
            json!({
                "iam_service_role_arn":{"references":["aws_iam_role.build.arn"]},
                "compute_role_arn":{"references":["aws_iam_role.compute.arn"]}
            }),
        ),
        (
            "aws_amplify_backend_environment.dev",
            json!({"app_id":"d123", "environment_name":"dev", "arn":backend}),
            json!({"app_id":{"references":["aws_amplify_app.site.id"]}}),
        ),
        (
            "aws_amplify_branch.trunk",
            json!({"app_id":"d123", "branch_name":"trunk", "backend_environment_arn":backend}),
            json!({
                "app_id":{"references":["aws_amplify_app.site.id"]},
                "backend_environment_arn":{"references":["aws_amplify_backend_environment.dev.arn"]}
            }),
        ),
    ])
}

#[test]
fn amplify_uses_separate_roles_and_same_app_backend() {
    let fixture = amplify();
    let input = fixture.to_string();
    for (address, path, source, field) in [
        (
            "aws_amplify_app.site",
            "iam_service_role_arn",
            "aws_iam_role.build",
            "arn",
        ),
        (
            "aws_amplify_app.site",
            "compute_role_arn",
            "aws_iam_role.compute",
            "arn",
        ),
        (
            "aws_amplify_backend_environment.dev",
            "app_id",
            "aws_amplify_app.site",
            "id",
        ),
        (
            "aws_amplify_branch.trunk",
            "backend_environment_arn",
            "aws_amplify_backend_environment.dev",
            "arn",
        ),
    ] {
        check(
            &input,
            crate::plan::parse,
            label,
            &[(address, path)],
            (source, field),
        );
        check_provider_alias(
            &input,
            address,
            path,
            source,
            label(address.split('.').next().unwrap(), path).unwrap(),
            !source.starts_with("aws_iam_role."),
        );
    }
    let graph = graph(&fixture);
    for node in graph.nodes.iter().filter(|n| {
        matches!(
            n.resource_type.as_str(),
            super::AMPLIFY_BRANCH | super::AMPLIFY_BACKEND
        )
    }) {
        assert_eq!(node.member_owner.as_ref().unwrap().attribute, "app_id");
    }
}

#[test]
fn amplify_rejects_cross_app_and_computed_endpoints() {
    let mut fixture = amplify();
    fixture["resource_changes"][3]["change"]["after"]["app_id"] = json!("different");
    let graph = graph(&fixture);
    assert!(!verified(
        &graph,
        "aws_amplify_branch.trunk",
        "backend_environment_arn",
        Some("Amplify backend environment")
    ));
    assert!(
        graph
            .nodes
            .iter()
            .find(|n| n.address == "aws_amplify_backend_environment.dev")
            .unwrap()
            .member_owner
            .is_none()
    );
    for (kind, path) in [
        (super::AMPLIFY_BRANCH, "associated_resources"),
        (super::AMPLIFY_BRANCH, "source_branch"),
        (super::AMPLIFY_BRANCH, "destination_branch"),
        (super::AMPLIFY_BACKEND, "stack_name"),
        (super::AMPLIFY_BACKEND, "deployment_artifacts"),
    ] {
        assert!(label(kind, path).is_none());
    }
}
