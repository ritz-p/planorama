use super::configured_links::{self, Evidence, Rule};
use crate::model::{AttributeReference, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;

fn rule(kind: &str, path: &str) -> Option<Rule> {
    Some(match (kind, path) {
        (
            "aws_appmesh_gateway_route"
            | "aws_appmesh_route"
            | "aws_appmesh_virtual_gateway"
            | "aws_appmesh_virtual_node"
            | "aws_appmesh_virtual_router"
            | "aws_appmesh_virtual_service",
            "mesh_name",
        ) => Rule::new("App Mesh", &[("aws_appmesh_mesh", &["name"])], false, &[]),
        ("aws_appmesh_gateway_route", "virtual_gateway_name") => Rule::new(
            "Virtual gateway",
            &[("aws_appmesh_virtual_gateway", &["name"])],
            false,
            &[],
        ),
        ("aws_appmesh_route", "virtual_router_name") => Rule::new(
            "Virtual router",
            &[("aws_appmesh_virtual_router", &["name"])],
            false,
            &[],
        ),
        ("aws_appmesh_route", "spec.grpc_route.action.weighted_target.virtual_node") => Rule::new(
            "Weighted virtual node",
            &[("aws_appmesh_virtual_node", &["name"])],
            false,
            &["spec.grpc_route.action.weighted_target"],
        ),
        ("aws_appmesh_route", "spec.http_route.action.weighted_target.virtual_node") => Rule::new(
            "Weighted virtual node",
            &[("aws_appmesh_virtual_node", &["name"])],
            false,
            &["spec.http_route.action.weighted_target"],
        ),
        ("aws_appmesh_route", "spec.http2_route.action.weighted_target.virtual_node") => Rule::new(
            "Weighted virtual node",
            &[("aws_appmesh_virtual_node", &["name"])],
            false,
            &["spec.http2_route.action.weighted_target"],
        ),
        ("aws_appmesh_route", "spec.tcp_route.action.weighted_target.virtual_node") => Rule::new(
            "Weighted virtual node",
            &[("aws_appmesh_virtual_node", &["name"])],
            false,
            &["spec.tcp_route.action.weighted_target"],
        ),
        (
            "aws_appmesh_gateway_route",
            "spec.grpc_route.action.target.virtual_service.virtual_service_name",
        ) => Rule::new(
            "Gateway target service",
            &[("aws_appmesh_virtual_service", &["name"])],
            false,
            &[],
        ),
        (
            "aws_appmesh_gateway_route",
            "spec.http_route.action.target.virtual_service.virtual_service_name",
        ) => Rule::new(
            "Gateway target service",
            &[("aws_appmesh_virtual_service", &["name"])],
            false,
            &[],
        ),
        (
            "aws_appmesh_gateway_route",
            "spec.http2_route.action.target.virtual_service.virtual_service_name",
        ) => Rule::new(
            "Gateway target service",
            &[("aws_appmesh_virtual_service", &["name"])],
            false,
            &[],
        ),
        ("aws_appmesh_virtual_service", "spec.provider.virtual_node.virtual_node_name") => {
            Rule::new(
                "Provider virtual node",
                &[("aws_appmesh_virtual_node", &["name"])],
                false,
                &[],
            )
        }
        ("aws_appmesh_virtual_service", "spec.provider.virtual_router.virtual_router_name") => {
            Rule::new(
                "Provider virtual router",
                &[("aws_appmesh_virtual_router", &["name"])],
                false,
                &[],
            )
        }
        ("aws_appmesh_virtual_node", "spec.backend.virtual_service.virtual_service_name") => {
            Rule::new(
                "Backend service",
                &[("aws_appmesh_virtual_service", &["name"])],
                false,
                &["spec.backend"],
            )
        }
        _ => return None,
    })
}
pub(crate) fn resource(kind: &str) -> bool {
    matches!(
        kind,
        "aws_appmesh_gateway_route"
            | "aws_appmesh_route"
            | "aws_appmesh_virtual_gateway"
            | "aws_appmesh_virtual_node"
            | "aws_appmesh_virtual_router"
            | "aws_appmesh_virtual_service"
    )
}
pub(crate) fn indexed(kind: &str, path: &str) -> bool {
    resource(kind) && configured_links::indexed(kind, path, rule)
}
pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    rule(kind, path).map(|r| r.label)
}
pub(super) fn bindings(raw: &Input, r: &AttributeReference) -> bool {
    configured_links::bindings(raw, r, rule)
}
pub(super) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    values: &[(&Value, &Value)],
) {
    configured_links::annotate(nodes, refs, values, rule, qualifies)
}
fn qualifies(e: &Evidence<'_>, r: &AttributeReference, source: usize, _path: &str) -> bool {
    if r.attribute == "mesh_name" {
        return mesh_owner_matches(e, r.target, source);
    }
    let mesh = |child| {
        e.configured_endpoint(child, "mesh_name", "aws_appmesh_mesh", &["name"])
            .filter(|&mesh| mesh_owner_matches(e, child, mesh))
    };
    if !mesh(r.target)
        .zip(mesh(source))
        .is_some_and(|(left, right)| left == right)
    {
        return false;
    }
    let absent = |path: &str| {
        e.value(r.target, path)
            .is_some_and(|value| value.is_null() || value.as_array().is_some_and(Vec::is_empty))
    };
    match r.attribute.as_str() {
        "spec.provider.virtual_node.virtual_node_name" => {
            absent("spec.0.provider.0.virtual_router")
        }
        "spec.provider.virtual_router.virtual_router_name" => {
            absent("spec.0.provider.0.virtual_node")
        }
        path if path.starts_with("spec.") && path.contains("_route.") => {
            ["grpc_route", "http_route", "http2_route", "tcp_route"]
                .into_iter()
                .all(|mode| {
                    path.split('.').nth(1) == Some(mode) || absent(&format!("spec.0.{mode}"))
                })
        }
        _ => true,
    }
}
fn mesh_owner_matches(e: &Evidence<'_>, child: usize, mesh: usize) -> bool {
    let configured = e.nodes[mesh]
        .provider_configuration
        .as_ref()
        .and_then(|provider| provider.account.as_ref())
        .map(crate::model::AwsAccountScope::as_str);
    let reported = e.text(mesh, "mesh_owner");
    if configured
        .zip(reported)
        .is_some_and(|(left, right)| left != right)
    {
        return false;
    }
    e.value(child, "mesh_owner").is_some_and(|value| {
        value.is_null()
            || value.as_str().is_some_and(|owner| {
                crate::model::AwsAccountScope::new(owner).is_some()
                    && Some(owner) == configured.or(reported)
            })
    })
}
#[cfg(test)]
#[path = "../../../tests/unit/semantic/appmesh_configuration.rs"]
mod tests;
