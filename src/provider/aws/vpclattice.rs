use crate::model::{AttributeReference, Edge, EdgeKind, Graph, TerraformEntity};
use crate::semantic::{Input, relationships};
use serde_json::Value;
use std::collections::BTreeSet;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/vpclattice.rs"]
mod tests;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/vpclattice_resources.rs"]
mod resource_tests;

const FORWARD: &str = "default_action.forward.target_groups.target_group_identifier";
const RULE_FORWARD: &str = "action.forward.target_groups.target_group_identifier";

pub(crate) fn resource(kind: &str) -> bool {
    matches!(
        kind,
        "aws_vpclattice_service"
            | "aws_vpclattice_service_network"
            | "aws_vpclattice_listener"
            | "aws_vpclattice_listener_rule"
            | "aws_vpclattice_target_group"
            | "aws_vpclattice_target_group_attachment"
            | "aws_vpclattice_service_network_service_association"
            | "aws_vpclattice_service_network_vpc_association"
            | "aws_vpclattice_auth_policy"
            | "aws_vpclattice_resource_policy"
            | "aws_vpclattice_access_log_subscription"
            | "aws_vpclattice_resource_gateway"
            | "aws_vpclattice_resource_configuration"
            | "aws_vpclattice_service_network_resource_association"
            | "aws_vpclattice_domain_verification"
    )
}

pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    match (kind, path) {
        ("aws_vpclattice_resource_gateway", "vpc_id") => Some("Resource gateway VPC"),
        ("aws_vpclattice_resource_gateway", "subnet_ids") => Some("Resource gateway subnets"),
        ("aws_vpclattice_resource_gateway", "security_group_ids") => {
            Some("Resource gateway security groups")
        }
        ("aws_vpclattice_resource_configuration", "resource_gateway_identifier") => {
            Some("Configuration gateway")
        }
        ("aws_vpclattice_resource_configuration", "resource_configuration_group_id") => {
            Some("Parent resource group")
        }
        ("aws_vpclattice_resource_configuration", "domain_verification_id") => {
            Some("Configured domain verification")
        }
        (
            "aws_vpclattice_service_network_resource_association",
            "resource_configuration_identifier",
        ) => Some("Associated resource configuration"),
        ("aws_vpclattice_service_network_resource_association", "service_network_identifier") => {
            Some("Lattice service network")
        }
        ("aws_vpclattice_service", "certificate_arn") => Some("Service certificate"),
        ("aws_vpclattice_listener", "service_identifier" | "service_arn")
        | (
            "aws_vpclattice_listener_rule" | "aws_vpclattice_service_network_service_association",
            "service_identifier",
        ) => Some("Lattice service"),
        ("aws_vpclattice_listener_rule", "listener_identifier") => Some("Rule listener"),
        ("aws_vpclattice_listener", FORWARD) | ("aws_vpclattice_listener_rule", RULE_FORWARD) => {
            Some("Configured target groups")
        }
        ("aws_vpclattice_target_group", "config.vpc_identifier") => Some("Target group VPC"),
        ("aws_vpclattice_target_group_attachment", "target_group_identifier") => {
            Some("Attachment target group")
        }
        ("aws_vpclattice_target_group_attachment", "target.id") => {
            Some("Configured Lattice target")
        }
        (
            "aws_vpclattice_service_network_service_association"
            | "aws_vpclattice_service_network_vpc_association",
            "service_network_identifier",
        ) => Some("Lattice service network"),
        ("aws_vpclattice_service_network_vpc_association", "vpc_identifier") => {
            Some("Associated VPC")
        }
        ("aws_vpclattice_service_network_vpc_association", "security_group_ids") => {
            Some("Association security groups")
        }
        ("aws_vpclattice_auth_policy", "resource_identifier")
        | ("aws_vpclattice_resource_policy", "resource_arn") => Some("Lattice policy resource"),
        ("aws_vpclattice_access_log_subscription", "resource_identifier") => {
            Some("Lattice logging resource")
        }
        ("aws_vpclattice_access_log_subscription", "destination_arn") => {
            Some("Configured log destination")
        }
        _ => None,
    }
}

fn fields(kind: &str, path: &str, source: &str) -> &'static [&'static str] {
    match (kind, path, source) {
        (_, "vpc_id", "aws_vpc") | (_, "subnet_ids", "aws_subnet") => &["id"],
        (_, "resource_gateway_identifier", "aws_vpclattice_resource_gateway")
        | (_, "resource_configuration_identifier", "aws_vpclattice_resource_configuration") => {
            &["id", "arn"]
        }
        (_, "resource_configuration_group_id", "aws_vpclattice_resource_configuration")
        | (_, "domain_verification_id", "aws_vpclattice_domain_verification") => &["id"],
        ("aws_vpclattice_service", "certificate_arn", "aws_acm_certificate") => &["arn"],
        ("aws_vpclattice_listener", "service_arn", "aws_vpclattice_service") => &["arn"],
        ("aws_vpclattice_listener", "service_identifier", "aws_vpclattice_service") => &["id"],
        (_, "service_identifier", "aws_vpclattice_service")
        | (_, "service_network_identifier", "aws_vpclattice_service_network")
        | (_, FORWARD | RULE_FORWARD | "target_group_identifier", "aws_vpclattice_target_group") => {
            &["id", "arn"]
        }
        (_, "listener_identifier", "aws_vpclattice_listener") => &["listener_id", "arn"],
        (_, "config.vpc_identifier" | "vpc_identifier", "aws_vpc")
        | (_, "security_group_ids", "aws_security_group")
        | (_, "target.id", "aws_instance") => &["id"],
        (_, "target.id", "aws_lambda_function" | "aws_lb") => &["arn"],
        (
            "aws_vpclattice_auth_policy",
            "resource_identifier",
            "aws_vpclattice_service" | "aws_vpclattice_service_network",
        )
        | (
            "aws_vpclattice_resource_policy",
            "resource_arn",
            "aws_vpclattice_service"
            | "aws_vpclattice_service_network"
            | "aws_vpclattice_resource_configuration",
        ) => &["arn"],
        (
            "aws_vpclattice_access_log_subscription",
            "resource_identifier",
            "aws_vpclattice_service"
            | "aws_vpclattice_service_network"
            | "aws_vpclattice_resource_configuration",
        ) => &["id", "arn"],
        (
            "aws_vpclattice_access_log_subscription",
            "destination_arn",
            "aws_s3_bucket" | "aws_cloudwatch_log_group" | "aws_kinesis_firehose_delivery_stream",
        ) => &["arn"],
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

fn basic(
    nodes: &[TerraformEntity],
    refs: &[AttributeReference],
    evidence: &[(&Value, &Value)],
    r: &AttributeReference,
) -> bool {
    super::nested_references::verify(nodes, refs, evidence, r, fields, |_, path| {
        let (v, m) = evidence[r.target];
        super::nested_references::value(v, m, path).is_some_and(|v| {
            if matches!(r.attribute.as_str(), "security_group_ids" | "subnet_ids") {
                v.is_array()
            } else {
                v.is_string()
            }
        })
    })
    .is_some()
}

fn endpoint(
    nodes: &[TerraformEntity],
    refs: &[AttributeReference],
    evidence: &[(&Value, &Value)],
    target: usize,
    path: &str,
) -> Option<usize> {
    let r = super::lambda_core::unique(refs, target, path)?;
    let [source] = r.sources.as_slice() else {
        return None;
    };
    basic(nodes, refs, evidence, r).then_some(*source)
}

fn listener_service(
    nodes: &[TerraformEntity],
    refs: &[AttributeReference],
    evidence: &[(&Value, &Value)],
    listener: usize,
) -> Option<usize> {
    let mut services = BTreeSet::new();
    for path in ["service_identifier", "service_arn"] {
        if refs
            .iter()
            .any(|r| r.target == listener && r.attribute == path)
        {
            services.insert(endpoint(nodes, refs, evidence, listener, path)?);
        }
    }
    if services.len() != 1 {
        return None;
    }
    let service = *services.first()?;
    for (field, output) in [("service_identifier", "id"), ("service_arn", "arn")] {
        if let Some(value) = super::service_evidence::known(evidence, listener, field) {
            if super::service_evidence::known(evidence, service, output) != Some(value) {
                return None;
            }
        }
    }
    Some(service)
}

fn absent(v: &Value, m: &Value, path: &str) -> bool {
    super::nested_references::value(v, m, path)
        .is_some_and(|v| v.is_null() || v.as_array().is_some_and(Vec::is_empty))
}

fn network_context(
    nodes: &[TerraformEntity],
    refs: &[AttributeReference],
    evidence: &[(&Value, &Value)],
    target: usize,
    vpc_path: &str,
    subnets: bool,
) -> Option<Vec<usize>> {
    let vpc = endpoint(nodes, refs, evidence, target, vpc_path)?;
    let (v, m) = evidence[target];
    let mut context = vec![vpc];
    for path in if subnets {
        &["subnet_ids", "security_group_ids"][..]
    } else {
        &["security_group_ids"][..]
    } {
        if *path == "security_group_ids" && absent(v, m, path) {
            if refs.iter().any(|r| {
                r.target == target
                    && r.attribute == *path
                    && (!r.sources.is_empty() || !r.issues.is_empty())
            }) {
                return None;
            }
            continue;
        }
        let members = super::lambda_core::unique(refs, target, path)?;
        if !basic(nodes, refs, evidence, members) {
            return None;
        }
        for &member in &members.sources {
            if super::lambda_core::parent(nodes, refs, evidence, member)? != vpc {
                return None;
            }
            context.push(member);
        }
    }
    Some(context)
}

fn context(
    nodes: &[TerraformEntity],
    refs: &[AttributeReference],
    evidence: &[(&Value, &Value)],
    r: &AttributeReference,
) -> Option<Vec<usize>> {
    let kind = nodes[r.target].resource_type.as_str();
    let (v, m) = evidence[r.target];
    let known = |index, path| super::service_evidence::known(evidence, index, path);
    let mut context = Vec::new();
    if kind == "aws_vpclattice_resource_gateway" {
        return network_context(nodes, refs, evidence, r.target, "vpc_id", true);
    }
    if kind == "aws_vpclattice_resource_configuration" {
        let mode = known(r.target, "type")?;
        if !matches!(mode, "SINGLE" | "GROUP" | "CHILD" | "ARN") {
            return None;
        }
        match r.attribute.as_str() {
            "resource_gateway_identifier" => {
                if mode == "CHILD" || !absent(v, m, "resource_configuration_group_id") {
                    return None;
                }
            }
            "resource_configuration_group_id" => {
                let [parent] = r.sources.as_slice() else {
                    return None;
                };
                if mode != "CHILD" || known(*parent, "type") != Some("GROUP") {
                    return None;
                }
            }
            _ => {}
        }
    }
    if kind == "aws_vpclattice_service_network_resource_association"
        && r.attribute == "resource_configuration_identifier"
    {
        let [configuration] = r.sources.as_slice() else {
            return None;
        };
        if !matches!(
            known(*configuration, "type"),
            Some("SINGLE" | "GROUP" | "ARN")
        ) {
            return None;
        }
    }
    if kind == "aws_vpclattice_listener" {
        context.push(listener_service(nodes, refs, evidence, r.target)?);
    }
    if kind == "aws_vpclattice_listener_rule" {
        let service = endpoint(nodes, refs, evidence, r.target, "service_identifier")?;
        let listener = endpoint(nodes, refs, evidence, r.target, "listener_identifier")?;
        if listener_service(nodes, refs, evidence, listener)? != service {
            return None;
        }
        context.extend([service, listener]);
    }
    if matches!(r.attribute.as_str(), FORWARD | RULE_FORWARD) {
        let action = if r.attribute == FORWARD {
            "default_action"
        } else {
            "action"
        };
        if !absent(v, m, &format!("{action}.0.fixed_response")) {
            return None;
        }
    }
    if kind == "aws_vpclattice_target_group"
        && r.attribute == "config.vpc_identifier"
        && !matches!(known(r.target, "type"), Some("INSTANCE" | "IP" | "ALB"))
    {
        return None;
    }
    if kind == "aws_vpclattice_target_group_attachment" && r.attribute == "target.id" {
        let group = endpoint(nodes, refs, evidence, r.target, "target_group_identifier")?;
        let [source] = r.sources.as_slice() else {
            return None;
        };
        if !match (
            known(group, "type")?,
            super::aliases::entity_type(&nodes[*source]),
        ) {
            ("INSTANCE", "aws_instance") | ("LAMBDA", "aws_lambda_function") => true,
            ("ALB", "aws_lb") => known(*source, "load_balancer_type") == Some("application"),
            _ => false,
        } {
            return None;
        }
        context.push(group);
    }
    if kind == "aws_vpclattice_service_network_vpc_association"
        && matches!(
            r.attribute.as_str(),
            "vpc_identifier" | "security_group_ids"
        )
    {
        return network_context(nodes, refs, evidence, r.target, "vpc_identifier", false);
    }
    if kind == "aws_vpclattice_access_log_subscription" {
        let owner = endpoint(nodes, refs, evidence, r.target, "resource_identifier")?;
        if !matches!(
            (
                nodes[owner].resource_type.as_str(),
                known(r.target, "service_network_log_type")?,
            ),
            ("aws_vpclattice_service", "SERVICE")
                | ("aws_vpclattice_resource_configuration", "RESOURCE")
                | ("aws_vpclattice_service_network", "SERVICE" | "RESOURCE")
        ) {
            return None;
        }
        context.push(owner);
    }
    Some(context)
}

pub(crate) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    evidence: &[(&Value, &Value)],
) {
    let results: Vec<_> = refs
        .iter()
        .map(|r| {
            label(&nodes[r.target].resource_type, &r.attribute)?;
            Some(
                basic(nodes, refs, evidence, r)
                    .then(|| context(nodes, refs, evidence, r))
                    .flatten(),
            )
        })
        .collect();
    for (r, result) in refs.iter_mut().zip(results) {
        if let Some(context) = result {
            r.service_reference_verified = context.is_some();
            r.endpoint_network_context = context.unwrap_or_default();
        }
    }
}

fn compatible(raw: &Input, target: usize, source: usize, cross_account: bool) -> bool {
    let a = &raw.nodes[target];
    let b = &raw.nodes[source];
    let ar = a.entity.scope.as_ref().and_then(|s| s.region.as_ref());
    let br = b.entity.scope.as_ref().and_then(|s| s.region.as_ref());
    if ar.is_none() || ar != br {
        return false;
    }
    if super::iam_policies::same_account(
        a.provider_configuration.as_ref(),
        b.provider_configuration.as_ref(),
    ) {
        return true;
    }
    cross_account
        && a.provider_configuration
            .as_ref()
            .zip(b.provider_configuration.as_ref())
            .is_some_and(|(a, b)| {
                !a.account_conflict
                    && !b.account_conflict
                    && a.account.is_some()
                    && b.account.is_some()
            })
}

pub(super) fn bindings(raw: &Input, r: &AttributeReference) -> bool {
    if label(&raw.nodes[r.target].resource_type, &r.attribute).is_none() {
        return true;
    }
    let cross = |r: &AttributeReference, source| {
        matches!(
            r.attribute.as_str(),
            "service_network_identifier"
                | "resource_identifier"
                | "resource_configuration_identifier"
        ) && matches!(
            raw.nodes[r.target].resource_type.as_str(),
            "aws_vpclattice_service_network_service_association"
                | "aws_vpclattice_service_network_vpc_association"
                | "aws_vpclattice_service_network_resource_association"
                | "aws_vpclattice_access_log_subscription"
        ) && r
            .source_attributes_by_instance
            .get(&source)
            .is_some_and(|attrs| attrs.len() == 1 && attrs.contains("arn"))
    };
    r.sources
        .iter()
        .all(|&s| compatible(raw, r.target, s, cross(r, s)))
        && r.endpoint_network_context.iter().all(|&s| {
            let shared_log_owner = raw.nodes[r.target].resource_type
                == "aws_vpclattice_access_log_subscription"
                && super::lambda_core::unique(&raw.attributes, r.target, "resource_identifier")
                    .is_some_and(|owner| cross(owner, s));
            compatible(raw, r.target, s, shared_log_owner)
        })
}

pub(super) fn present(raw: &Input, graph: &mut Graph) {
    for (target, node) in raw.nodes.iter().enumerate().filter(|(_, n)| {
        n.provider.is_aws() && n.resource_type == "aws_vpclattice_resource_gateway"
    }) {
        let Some(r) = super::lambda_core::unique(&raw.attributes, target, "vpc_id")
            .filter(|r| super::service_references::verified(raw, r))
        else {
            continue;
        };
        let [vpc] = r.sources.as_slice() else {
            continue;
        };
        if !node.presentation_scope_matches(&raw.nodes[*vpc]) {
            continue;
        }
        let evidence = r
            .endpoint_network_context
            .iter()
            .map(|&member| relationships::reference(graph, member, target))
            .collect();
        relationships::record(graph, *vpc, target, EdgeKind::Containment, evidence);
        graph.edges.push(Edge {
            kind: EdgeKind::Containment,
            ..Edge::from((*vpc, target))
        });
    }
}
