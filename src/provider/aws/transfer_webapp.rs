use crate::model::{AttributeReference, Graph, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;
#[cfg(test)]
#[path = "../../../tests/unit/semantic/transfer_webapp.rs"]
mod tests;
const WEB: &str = "aws_transfer_web_app";
const INSTANCE: &str = "identity_provider_details.identity_center_config.instance_arn";
const ROLE: &str = "identity_provider_details.identity_center_config.role";
const VPC: &str = "endpoint_details.vpc.vpc_id";
const SUBNETS: &str = "endpoint_details.vpc.subnet_ids";
const GROUPS: &str = "endpoint_details.vpc.security_group_ids";
pub(crate) fn resource(kind: &str) -> bool {
    matches!(kind, WEB | "aws_transfer_web_app_customization")
}
pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    match (kind, path) {
        (WEB, INSTANCE) => Some("Identity Center instance"),
        (WEB, ROLE) => Some("Web app Identity Center role"),
        (WEB, VPC) => Some("Web app VPC"),
        (WEB, SUBNETS) => Some("Web app subnets"),
        (WEB, GROUPS) => Some("Web app security groups"),
        ("aws_transfer_web_app_customization", "web_app_id") => Some("Web app owner"),
        _ => None,
    }
}
pub(crate) fn indexed(kind: &str, path: &str) -> bool {
    kind == WEB
        && matches!(
            path,
            "identity_provider_details.0.identity_center_config.0.instance_arn"
                | "identity_provider_details.0.identity_center_config.0.role"
                | "endpoint_details.0.vpc.0.vpc_id"
                | "endpoint_details.0.vpc.0.subnet_ids"
                | "endpoint_details.0.vpc.0.security_group_ids"
        )
}
fn fields(kind: &str, path: &str, source: &str) -> &'static [&'static str] {
    match (kind, path, source) {
        (WEB, VPC, "aws_vpc")
        | (WEB, SUBNETS, "aws_subnet")
        | (WEB, GROUPS, "aws_security_group") => &["id"],
        (WEB, ROLE, "aws_iam_role") => &["arn"],
        ("aws_transfer_web_app_customization", "web_app_id", WEB) => &["web_app_id"],
        _ => &[],
    }
}
fn shape(mut value: &Value, path: &str) -> bool {
    let mut parts = path.split('.').peekable();
    while let Some(part) = parts.next() {
        value = &value[part];
        if parts.peek().is_some() {
            let Some(rows) = value.as_array() else {
                return false;
            };
            let [row] = rows.as_slice() else {
                return false;
            };
            value = row;
        }
    }
    if matches!(path, SUBNETS | GROUPS) {
        value.is_array()
    } else {
        value.is_string()
    }
}
fn basic(
    nodes: &[TerraformEntity],
    refs: &[AttributeReference],
    e: &[(&Value, &Value)],
    r: &AttributeReference,
) -> bool {
    shape(e[r.target].0, &r.attribute)
        && super::nested_references::verify(nodes, refs, e, r, fields, |_, _| true).is_some()
}
fn proof(
    nodes: &[TerraformEntity],
    refs: &[AttributeReference],
    e: &[(&Value, &Value)],
    r: &AttributeReference,
) -> Option<Vec<usize>> {
    if super::lambda_core::unique(refs, r.target, &r.attribute).is_none()
        || !shape(e[r.target].0, &r.attribute)
    {
        return None;
    }
    if r.attribute == INSTANCE {
        if !r.instances_explicit || !r.issues.is_empty() {
            return None;
        }
        let path = "identity_provider_details.0.identity_center_config.0.instance_arn";
        let leaf = super::lambda_core::unique(refs, r.target, path)?;
        let actual =
            super::nested_references::value(e[r.target].0, e[r.target].1, path)?.as_str()?;
        return (leaf.indexed_block_lengths == [1, 1]
            && leaf.sources == r.sources
            && super::identity_center_instance::verify(nodes, leaf, e, actual))
        .then(Vec::new);
    }
    if !basic(nodes, refs, e, r) {
        return None;
    }
    if !matches!(r.attribute.as_str(), SUBNETS | GROUPS) {
        return Some(Vec::new());
    }
    let vpc = super::lambda_core::unique(refs, r.target, VPC)?;
    let [parent] = vpc.sources.as_slice() else {
        return None;
    };
    (basic(nodes, refs, e, vpc)
        && r.sources
            .iter()
            .all(|&s| super::lambda_core::parent(nodes, refs, e, s) == Some(*parent)))
    .then_some(vec![*parent])
}
pub(super) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    e: &[(&Value, &Value)],
) {
    let results: Vec<_> = refs
        .iter()
        .map(|r| {
            label(&nodes[r.target].resource_type, &r.attribute).map(|_| proof(nodes, refs, e, r))
        })
        .collect();
    for (r, result) in refs.iter_mut().zip(results) {
        if let Some(context) = result {
            r.service_reference_verified = context.is_some();
            if context.is_some() {
                r.complete = true;
            }
            r.endpoint_network_context = context.unwrap_or_default();
        }
    }
}
pub(super) fn bindings(raw: &Input, r: &AttributeReference) -> bool {
    if label(&raw.nodes[r.target].resource_type, &r.attribute).is_none() {
        return true;
    }
    r.sources
        .iter()
        .chain(&r.endpoint_network_context)
        .all(|&s| {
            let l = &raw.nodes[r.target];
            let right = &raw.nodes[s];
            super::eventbridge::bindings(
                l.provider_configuration.as_ref(),
                right.provider_configuration.as_ref(),
                l.entity.scope.as_ref().and_then(|s| s.region.as_ref()),
                right.entity.scope.as_ref().and_then(|s| s.region.as_ref()),
                right.resource_type == "aws_iam_role",
            )
        })
}
pub(super) fn present(raw: &Input, graph: &mut Graph) {
    super::lambda_core::present_network(raw, graph, WEB, (SUBNETS, GROUPS), bindings);
}
