use crate::model::{AttributeReference, Graph, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;
#[cfg(test)]
#[path = "../../../tests/unit/semantic/transfer_server.rs"]
mod tests;
const SERVER: &str = "aws_transfer_server";
const VPC: &str = "endpoint_details.vpc_id";
const SUBNETS: &str = "endpoint_details.subnet_ids";
const GROUPS: &str = "endpoint_details.security_group_ids";
const EIPS: &str = "endpoint_details.address_allocation_ids";
const ENDPOINT: &str = "endpoint_details.vpc_endpoint_id";
const LOGS: &str = "structured_log_destinations";
pub(crate) fn resource(kind: &str) -> bool {
    kind == SERVER
}
pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    if kind != SERVER {
        return None;
    }
    match path {
        VPC => Some("Transfer endpoint VPC"),
        SUBNETS => Some("Transfer endpoint subnets"),
        GROUPS => Some("Transfer endpoint security groups"),
        EIPS => Some("Transfer endpoint Elastic IPs"),
        ENDPOINT => Some("Transfer VPC endpoint"),
        "certificate" => Some("Transfer FTPS certificate"),
        "directory_id" => Some("Transfer identity directory"),
        "function" => Some("Transfer identity Lambda"),
        "invocation_role" => Some("Transfer identity invocation role"),
        "logging_role" => Some("Transfer logging role"),
        LOGS => Some("Transfer structured log groups"),
        "workflow_details.on_upload.workflow_id" => Some("Upload workflow"),
        "workflow_details.on_partial_upload.workflow_id" => Some("Partial upload workflow"),
        "workflow_details.on_upload.execution_role" => Some("Upload workflow role"),
        "workflow_details.on_partial_upload.execution_role" => Some("Partial upload workflow role"),
        _ => None,
    }
}
fn fields(_: &str, path: &str, source: &str) -> &'static [&'static str] {
    match (path, source) {
        (VPC, "aws_vpc")
        | (SUBNETS, "aws_subnet")
        | (GROUPS, "aws_security_group")
        | (ENDPOINT, "aws_vpc_endpoint")
        | ("directory_id", "aws_directory_service_directory")
        | (
            "workflow_details.on_upload.workflow_id"
            | "workflow_details.on_partial_upload.workflow_id",
            "aws_transfer_workflow",
        ) => &["id"],
        (EIPS, "aws_eip") => &["allocation_id", "id"],
        ("certificate", "aws_acm_certificate")
        | ("function", "aws_lambda_function")
        | (
            "invocation_role"
            | "logging_role"
            | "workflow_details.on_upload.execution_role"
            | "workflow_details.on_partial_upload.execution_role",
            "aws_iam_role",
        )
        | (LOGS, "aws_cloudwatch_log_group") => &["arn"],
        _ => &[],
    }
}
pub(crate) fn indexed(kind: &str, path: &str) -> bool {
    let normal = path
        .split('.')
        .filter(|s| s.parse::<usize>().is_err())
        .collect::<Vec<_>>()
        .join(".");
    normal != path && label(kind, &normal).is_some()
}
fn value<'a>(e: &[(&'a Value, &Value)], t: usize, path: &str) -> Option<&'a Value> {
    super::nested_references::value(e[t].0, e[t].1, path)
}
fn known<'a>(e: &[(&'a Value, &Value)], t: usize, path: &str) -> Option<&'a str> {
    value(e, t, path)?.as_str().filter(|s| !s.is_empty())
}
fn singleton(v: &Value) -> bool {
    v.as_array().is_some_and(|a| a.len() == 1)
}
fn basic(
    nodes: &[TerraformEntity],
    refs: &[AttributeReference],
    e: &[(&Value, &Value)],
    r: &AttributeReference,
) -> bool {
    super::nested_references::verify(nodes, refs, e, r, fields, |_, path| {
        value(e, r.target, path).is_some_and(|v| {
            if matches!(r.attribute.as_str(), SUBNETS | GROUPS | EIPS | LOGS) {
                v.is_array()
            } else {
                v.is_string()
            }
        })
    })
    .is_some()
}
fn vpc(
    nodes: &[TerraformEntity],
    refs: &[AttributeReference],
    e: &[(&Value, &Value)],
    t: usize,
) -> Option<usize> {
    let r = super::lambda_core::unique(refs, t, VPC)?;
    let [s] = r.sources.as_slice() else {
        return None;
    };
    basic(nodes, refs, e, r).then_some(*s)
}
fn network(
    nodes: &[TerraformEntity],
    refs: &[AttributeReference],
    e: &[(&Value, &Value)],
    r: &AttributeReference,
) -> Option<usize> {
    let parent = vpc(nodes, refs, e, r.target)?;
    (basic(nodes, refs, e, r)
        && r.sources
            .iter()
            .all(|&s| super::lambda_core::parent(nodes, refs, e, s) == Some(parent)))
    .then_some(parent)
}
fn logs(
    nodes: &[TerraformEntity],
    refs: &[AttributeReference],
    e: &[(&Value, &Value)],
    r: &AttributeReference,
) -> Option<()> {
    let actual = value(e, r.target, LOGS)?.as_array()?;
    let normalized: Option<Vec<Value>> = actual
        .iter()
        .map(|v| {
            let s = v.as_str()?;
            Some(Value::String(s.strip_suffix(":*").unwrap_or(s).into()))
        })
        .collect();
    let mut v = e[r.target].0.clone();
    v[LOGS] = Value::Array(normalized?);
    let mut evidence = e.to_vec();
    evidence[r.target].0 = &v;
    basic(nodes, refs, &evidence, r).then_some(())
}
fn proof(
    nodes: &[TerraformEntity],
    refs: &[AttributeReference],
    e: &[(&Value, &Value)],
    r: &AttributeReference,
) -> Option<Vec<usize>> {
    let t = r.target;
    let path = r.attribute.as_str();
    let mut context = Vec::new();
    if path == LOGS {
        logs(nodes, refs, e, r)?;
    } else if !basic(nodes, refs, e, r) {
        return None;
    }
    if path.starts_with("endpoint_details.") {
        if !singleton(&e[t].0["endpoint_details"]) {
            return None;
        }
        if path == ENDPOINT {
            if known(e, t, "endpoint_type") != Some("VPC_ENDPOINT")
                || !r
                    .sources
                    .iter()
                    .all(|&s| known(e, s, "vpc_endpoint_type") == Some("Interface"))
            {
                return None;
            }
        } else {
            if known(e, t, "endpoint_type") != Some("VPC") {
                return None;
            }
            match path {
                SUBNETS | GROUPS => context.push(network(nodes, refs, e, r)?),
                EIPS => {
                    if known(e, t, "ip_address_type") != Some("IPV4") {
                        return None;
                    }
                    let subnets = super::lambda_core::unique(refs, t, SUBNETS)?;
                    let parent = network(nodes, refs, e, subnets)?;
                    if r.sources.len() != subnets.sources.len() {
                        return None;
                    }
                    context.push(parent);
                    context.extend(&subnets.sources);
                }
                _ => {}
            }
        }
    }
    match path {
        "certificate" => {
            let protocols = value(e, t, "protocols")?.as_array()?;
            if !protocols.iter().any(|p| p.as_str() == Some("FTPS"))
                || !protocols
                    .iter()
                    .all(|p| matches!(p.as_str(), Some("FTP" | "FTPS" | "SFTP" | "AS2")))
            {
                return None;
            }
        }
        "directory_id"
            if known(e, t, "identity_provider_type") != Some("AWS_DIRECTORY_SERVICE") =>
        {
            return None;
        }
        "function" if known(e, t, "identity_provider_type") != Some("AWS_LAMBDA") => return None,
        "invocation_role" if known(e, t, "identity_provider_type") != Some("API_GATEWAY") => {
            return None;
        }
        p if p.starts_with("workflow_details.") => {
            let trigger = p.split('.').nth(1)?;
            if !singleton(&e[t].0["workflow_details"])
                || !singleton(&e[t].0["workflow_details"][0][trigger])
            {
                return None;
            }
        }
        _ => {}
    }
    Some(context)
}
pub(crate) fn annotate(
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
        if let Some(result) = result {
            r.service_reference_verified = result.is_some();
            if let Some(context) = result {
                r.endpoint_network_context = context;
                r.complete = true;
            }
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
            let a = &raw.nodes[r.target];
            let b = &raw.nodes[s];
            let ar = a.entity.scope.as_ref().and_then(|s| s.region.as_ref());
            let br = b.entity.scope.as_ref().and_then(|s| s.region.as_ref());
            super::iam_policies::same_account(
                a.provider_configuration.as_ref(),
                b.provider_configuration.as_ref(),
            ) && ar.is_some()
                && (b.resource_type == "aws_iam_role" || ar == br)
        })
}
pub(super) fn present(raw: &Input, graph: &mut Graph) {
    super::lambda_core::present_network(raw, graph, SERVER, (SUBNETS, GROUPS), bindings);
}
