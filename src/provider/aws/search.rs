use crate::model::{
    Action, AttributeReference, Edge, EdgeKind, EntityMode, Graph, TerraformEntity,
};
use crate::semantic::{Input, relationships};
use serde_json::Value;
use std::collections::BTreeSet;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/search.rs"]
mod tests;

const SUBNETS: &str = "vpc_options.subnet_ids";
const GROUPS: &str = "vpc_options.security_group_ids";
const INSTANCE: &str = "iam_identity_center_options.iam_identity_center_instance_arn";

pub(super) fn helper(kind: &str) -> bool {
    matches!(
        kind,
        "aws_cloudsearch_domain_service_access_policy"
            | "aws_opensearch_domain_policy"
            | "aws_opensearch_domain_saml_options"
            | "aws_elasticsearch_domain_policy"
            | "aws_elasticsearch_domain_saml_options"
            | "aws_opensearch_authorize_vpc_endpoint_access"
    )
}

fn network(kind: &str) -> bool {
    matches!(
        kind,
        "aws_elasticsearch_domain"
            | "aws_elasticsearch_vpc_endpoint"
            | "aws_opensearch_vpc_endpoint"
    )
}

fn rule(
    kind: &str,
    path: &str,
) -> Option<(
    &'static [&'static str],
    &'static [&'static str],
    &'static str,
)> {
    if helper(kind) && path == "domain_name" {
        return Some(match kind {
            "aws_cloudsearch_domain_service_access_policy" => {
                (&["aws_cloudsearch_domain"], &["name"], "Search domain")
            }
            "aws_elasticsearch_domain_policy" | "aws_elasticsearch_domain_saml_options" => (
                &["aws_elasticsearch_domain"],
                &["domain_name"],
                "Search domain",
            ),
            _ => (
                &["aws_opensearch_domain"],
                &["domain_name"],
                "Search domain",
            ),
        });
    }
    if network(kind) {
        match path {
            SUBNETS => return Some((&["aws_subnet"], &["id"], "Configured subnets")),
            GROUPS => {
                return Some((
                    &["aws_security_group"],
                    &["id"],
                    "Configured security groups",
                ));
            }
            _ => {}
        }
    }
    if matches!(kind, "aws_opensearch_domain" | "aws_elasticsearch_domain") {
        return match path {
            "cognito_options.identity_pool_id" => Some((
                &["aws_cognito_identity_pool"],
                &["id"],
                "Cognito identity pool",
            )),
            "cognito_options.user_pool_id" => {
                Some((&["aws_cognito_user_pool"], &["id"], "Cognito user pool"))
            }
            "cognito_options.role_arn" => Some((&["aws_iam_role"], &["arn"], "Cognito role")),
            "domain_endpoint_options.custom_endpoint_certificate_arn" => {
                Some((&["aws_acm_certificate"], &["arn"], "Endpoint certificate"))
            }
            "encrypt_at_rest.kms_key_id" => Some((
                &["aws_kms_key"],
                &["id", "key_id", "arn"],
                "Encryption key reference",
            )),
            "log_publishing_options.cloudwatch_log_group_arn" => Some((
                &["aws_cloudwatch_log_group"],
                &["arn"],
                "Search log destination",
            )),
            _ => None,
        };
    }
    match (kind, path) {
        ("aws_opensearch_vpc_endpoint", "domain_arn") => {
            Some((&["aws_opensearch_domain"], &["arn"], "Endpoint domain"))
        }
        ("aws_elasticsearch_vpc_endpoint", "domain_arn") => {
            Some((&["aws_elasticsearch_domain"], &["arn"], "Endpoint domain"))
        }
        ("aws_opensearch_application", "data_source.data_source_arn") => Some((
            &[
                "aws_opensearch_domain",
                "aws_opensearchserverless_collection",
            ],
            &["arn"],
            "Search data source",
        )),
        ("aws_opensearch_application", "kms_key_arn") => {
            Some((&["aws_kms_key"], &["arn"], "Encryption key reference"))
        }
        (
            "aws_opensearch_application",
            "iam_identity_center_options.iam_role_for_identity_center_application_arn",
        ) => Some((&["aws_iam_role"], &["arn"], "Identity Center role")),
        ("aws_opensearch_application", INSTANCE) => Some((
            &["aws_ssoadmin_instances"],
            &["arns"],
            "Identity Center instance lookup",
        )),
        ("aws_opensearch_package", "package_source.s3_bucket_name") => {
            Some((&["aws_s3_bucket"], &["bucket"], "Package source bucket"))
        }
        ("aws_opensearch_package_association", "package_id") => {
            Some((&["aws_opensearch_package"], &["id"], "Search package"))
        }
        ("aws_opensearch_package_association", "domain_name") => Some((
            &["aws_opensearch_domain"],
            &["domain_name"],
            "Search domain",
        )),
        _ => None,
    }
}

pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    rule(kind, path).map(|r| r.2)
}

fn leaves<'a>(
    value: &'a Value,
    mask: &Value,
    path: &[&str],
    out: &mut Vec<&'a Value>,
) -> Option<()> {
    if let Some(rows) = value.as_array() {
        if rows.is_empty()
            || !matches!(mask, Value::Null | Value::Bool(false) | Value::Array(_))
            || mask.as_array().is_some_and(|m| m.len() != rows.len())
        {
            return None;
        }
        for (i, row) in rows.iter().enumerate() {
            leaves(row, &mask[i], path, out)?;
        }
    } else if let Some((key, rest)) = path.split_first() {
        if !value.is_object()
            || !matches!(mask, Value::Null | Value::Bool(false) | Value::Object(_))
        {
            return None;
        }
        leaves(&value[*key], &mask[*key], rest, out)?;
    } else {
        if !matches!(mask, Value::Null | Value::Bool(false)) {
            return None;
        }
        out.push(value);
    }
    Some(())
}

fn values<'a>(value: &'a Value, mask: &Value, path: &str) -> Option<BTreeSet<&'a str>> {
    let mut out = Vec::new();
    leaves(value, mask, &path.split('.').collect::<Vec<_>>(), &mut out)?;
    out.into_iter()
        .map(|v| v.as_str().filter(|s| !s.is_empty()))
        .collect()
}

fn enabled(value: &Value, mask: &Value, path: &str) -> bool {
    let mut out = Vec::new();
    leaves(value, mask, &path.split('.').collect::<Vec<_>>(), &mut out).is_some()
        && !out.is_empty()
        && out.iter().all(|v| v.as_bool() == Some(true))
}

fn gate(path: &str) -> Option<&'static str> {
    if path.starts_with("cognito_options.") {
        Some("cognito_options.enabled")
    } else if path.starts_with("domain_endpoint_options.") {
        Some("domain_endpoint_options.custom_endpoint_enabled")
    } else if path.starts_with("encrypt_at_rest.") {
        Some("encrypt_at_rest.enabled")
    } else if path.starts_with("log_publishing_options.") {
        Some("log_publishing_options.enabled")
    } else if path.starts_with("iam_identity_center_options.") {
        Some("iam_identity_center_options.enabled")
    } else {
        None
    }
}

fn current(node: &TerraformEntity) -> bool {
    node.provider.is_aws() && node.action != Action::Delete && node.deposed_key.is_none()
}

fn lookup_binding(left: &TerraformEntity, right: &TerraformEntity) -> bool {
    let region = |n: &TerraformEntity| {
        if n.has_region_override {
            n.region_override.clone()
        } else {
            n.provider_configuration
                .as_ref()
                .and_then(|c| c.region.clone())
        }
    };
    super::eventbridge::bindings(
        left.provider_configuration.as_ref(),
        right.provider_configuration.as_ref(),
        region(left).as_ref(),
        region(right).as_ref(),
        false,
    )
}

pub(super) fn bindings(raw: &Input, r: &AttributeReference) -> bool {
    if r.attribute == INSTANCE {
        return r.sources.iter().all(|&s| {
            let a = &raw.nodes[r.target];
            let b = &raw.nodes[s];
            super::eventbridge::bindings(
                a.provider_configuration.as_ref(),
                b.provider_configuration.as_ref(),
                a.entity.scope.as_ref().and_then(|s| s.region.as_ref()),
                b.provider_configuration
                    .as_ref()
                    .and_then(|c| c.region.as_ref()),
                false,
            )
        });
    }
    r.sources.iter().all(|&s| {
        let a = &raw.nodes[r.target];
        let b = &raw.nodes[s];
        super::eventbridge::bindings(
            a.provider_configuration.as_ref(),
            b.provider_configuration.as_ref(),
            a.entity.scope.as_ref().and_then(|s| s.region.as_ref()),
            b.entity.scope.as_ref().and_then(|s| s.region.as_ref()),
            b.resource_type == "aws_iam_role",
        )
    })
}

pub(crate) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    evidence: &[(&Value, &Value)],
) {
    let mut counts = std::collections::BTreeMap::new();
    for r in refs.iter() {
        *counts
            .entry((r.target, r.attribute.clone()))
            .or_insert(0usize) += 1;
    }
    for r in refs.iter_mut() {
        let Some((types, outputs, _)) = rule(&nodes[r.target].resource_type, &r.attribute) else {
            continue;
        };
        r.service_reference_verified = false;
        if counts[&(r.target, r.attribute.clone())] != 1
            || !current(&nodes[r.target])
            || nodes[r.target].mode != EntityMode::Managed
            || !r.complete
            || !r.instances_explicit
            || !r.issues.is_empty()
            || r.sources.is_empty()
        {
            continue;
        }
        if r.attribute != INSTANCE
            && !super::service_evidence::eligible_with_global_role(nodes, r, true)
        {
            continue;
        }
        let (v, m) = evidence[r.target];
        let first = r.attribute.split('.').next().unwrap_or_default();
        if r.attribute.contains('.')
            && !matches!(first, "log_publishing_options" | "data_source")
            && v[first].as_array().is_none_or(|a| a.len() != 1)
        {
            continue;
        }
        if gate(&r.attribute).is_some_and(|g| !enabled(v, m, g)) {
            continue;
        }
        let Some(actual) = values(v, m, &r.attribute) else {
            continue;
        };
        let expected = r
            .sources
            .iter()
            .map(|&s| {
                let n = &nodes[s];
                if !current(n) || !types.contains(&n.resource_type.as_str()) {
                    return None;
                }
                let attrs = r.source_attributes_by_instance.get(&s)?;
                if attrs.len() != 1 {
                    return None;
                }
                let output = attrs.first()?.as_str();
                if !outputs.contains(&output) {
                    return None;
                }
                if r.attribute == INSTANCE {
                    if n.mode != EntityMode::Data || !lookup_binding(&nodes[r.target], n) {
                        return None;
                    }
                    let set = values(evidence[s].0, evidence[s].1, output)?;
                    return (set.len() == 1).then(|| *set.first().unwrap());
                }
                if n.mode != EntityMode::Managed {
                    return None;
                }
                let output_value = super::service_evidence::known(evidence, s, output)?;
                if r.attribute == "encrypt_at_rest.kms_key_id"
                    && output != "arn"
                    && !actual.contains(output_value)
                {
                    let arn = super::service_evidence::known(evidence, s, "arn")?;
                    return arn
                        .rsplit_once(":key/")
                        .filter(|(_, id)| *id == output_value)
                        .map(|_| arn);
                }
                Some(output_value)
            })
            .collect::<Option<BTreeSet<_>>>();
        r.service_reference_verified =
            expected.is_some_and(|e| e.len() == r.sources.len() && e == actual);
    }
    let valid_network: Vec<_> = nodes
        .iter()
        .enumerate()
        .map(|(t, n)| {
            if !network(&n.resource_type) {
                return false;
            }
            let proof = |path| {
                refs.iter()
                    .find(|r| r.target == t && r.attribute == path && r.service_reference_verified)
            };
            let Some((subnets, groups)) = proof(SUBNETS).zip(proof(GROUPS)) else {
                return false;
            };
            let vpcs = subnets
                .sources
                .iter()
                .chain(&groups.sources)
                .map(|&s| super::service_evidence::known(evidence, s, "vpc_id"))
                .collect::<Option<BTreeSet<_>>>();
            vpcs.is_some_and(|v| v.len() == 1)
        })
        .collect();
    for r in refs {
        if network(&nodes[r.target].resource_type)
            && matches!(r.attribute.as_str(), SUBNETS | GROUPS)
        {
            r.service_reference_verified &= valid_network[r.target];
        }
    }
}

fn proof<'a>(raw: &'a Input, target: usize, path: &str) -> Option<&'a AttributeReference> {
    let mut found = raw
        .attributes
        .iter()
        .filter(|r| r.target == target && r.attribute == path);
    let r = found.next()?;
    (found.next().is_none() && super::service_references::verified(raw, r)).then_some(r)
}

pub(super) fn association(raw: &Input, target: usize) -> Option<(usize, usize)> {
    let package = proof(raw, target, "package_id")?;
    let domain = proof(raw, target, "domain_name")?;
    let ([package], [domain]) = (package.sources.as_slice(), domain.sources.as_slice()) else {
        return None;
    };
    Some((*package, *domain))
}

pub(super) fn present(raw: &Input, graph: &mut Graph) {
    for (target, n) in raw.nodes.iter().enumerate() {
        if !network(&n.resource_type) {
            continue;
        }
        let Some((subnets, groups)) = proof(raw, target, SUBNETS).zip(proof(raw, target, GROUPS))
        else {
            continue;
        };
        let members: Vec<_> = subnets
            .sources
            .iter()
            .chain(&groups.sources)
            .copied()
            .collect();
        let Some(parents) = members
            .iter()
            .map(|&s| super::streaming::vpc(raw, s))
            .collect::<Option<BTreeSet<_>>>()
        else {
            continue;
        };
        if parents.len() != 1 {
            continue;
        }
        let parent = *parents.first().unwrap();
        graph.nodes[target].placement = crate::model::Placement::Spatial;
        let mut evidence = Vec::new();
        for member in members {
            evidence.push(relationships::reference(graph, parent, member));
            evidence.push(relationships::reference(graph, member, target));
        }
        relationships::record(graph, parent, target, EdgeKind::Containment, evidence);
        graph.edges.push(Edge {
            kind: EdgeKind::Containment,
            ..Edge::from((parent, target))
        });
        for &group in &groups.sources {
            let evidence = relationships::reference(graph, group, target);
            for edge in &mut graph.edges {
                if edge.from == group && edge.to == target && edge.kind == EdgeKind::Dependency {
                    edge.kind = EdgeKind::Connection;
                }
            }
            relationships::record(graph, group, target, EdgeKind::Connection, vec![evidence]);
        }
    }
}
