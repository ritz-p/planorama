use crate::model::{AttributeReference, Edge, EdgeKind, Graph, TerraformEntity};
use crate::semantic::{Input, relationships};
use serde_json::Value;
const CONFIG: &str = "aws_dms_replication_config";
const SUBNETS: &str = "compute_config.replication_subnet_group_id";
const GROUPS: &str = "compute_config.vpc_security_group_ids";
const KEY: &str = "compute_config.kms_key_id";
pub(crate) fn resource(kind: &str) -> bool {
    kind == CONFIG
}
pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    if kind != CONFIG {
        return None;
    }
    match path {
        "source_endpoint_arn" => Some("Serverless source endpoint"),
        "target_endpoint_arn" => Some("Serverless target endpoint"),
        SUBNETS => Some("Serverless replication subnet group"),
        GROUPS => Some("Serverless replication security groups"),
        KEY => Some("Serverless replication encryption key"),
        _ => None,
    }
}
pub(crate) fn indexed(kind: &str, path: &str) -> bool {
    kind == CONFIG
        && matches!(
            path,
            "compute_config.0.replication_subnet_group_id"
                | "compute_config.0.vpc_security_group_ids"
                | "compute_config.0.kms_key_id"
        )
}
fn fields(_: &str, path: &str, source: &str) -> &'static [&'static str] {
    match (path, source) {
        (
            "source_endpoint_arn" | "target_endpoint_arn",
            "aws_dms_endpoint" | "aws_dms_s3_endpoint",
        ) => &["endpoint_arn"],
        (SUBNETS, "aws_dms_replication_subnet_group") | (GROUPS, "aws_security_group") => &["id"],
        (KEY, "aws_kms_key") => &["arn"],
        _ => &[],
    }
}
fn compatible(l: &TerraformEntity, r: &TerraformEntity) -> bool {
    let left = super::scope::scope(l);
    let right = super::scope::scope(r);
    super::eventbridge::bindings(
        l.provider_configuration.as_ref(),
        r.provider_configuration.as_ref(),
        left.as_ref().and_then(|s| s.region.as_ref()),
        right.as_ref().and_then(|s| s.region.as_ref()),
        false,
    )
}
pub(super) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    e: &[(&Value, &Value)],
) {
    let results: Vec<_> = refs
        .iter()
        .map(|r| {
            label(&nodes[r.target].resource_type, &r.attribute)?;
            let valid =
                matches!(
                    super::service_evidence::known(e, r.target, "replication_type"),
                    Some("full-load" | "cdc" | "full-load-and-cdc")
                ) && super::nested_references::verify(nodes, refs, e, r, fields, |s, path| {
                    let shape = super::nested_references::value(e[r.target].0, e[r.target].1, path)
                        .is_some_and(|v| {
                            if r.attribute == GROUPS {
                                v.is_array()
                            } else {
                                v.is_string()
                            }
                        });
                    let direction = match r.attribute.as_str() {
                        "source_endpoint_arn" => Some("source"),
                        "target_endpoint_arn" => Some("target"),
                        _ => None,
                    };
                    shape
                        && compatible(&nodes[r.target], &nodes[s])
                        && direction.is_none_or(|expected| {
                            super::service_evidence::known(e, s, "endpoint_type") == Some(expected)
                        })
                })
                .is_some();
            Some(valid)
        })
        .collect();
    for (r, valid) in refs.iter_mut().zip(results) {
        if let Some(valid) = valid {
            r.service_reference_verified = valid;
            if valid {
                r.complete = true;
            }
        }
    }
    let contexts: Vec<_> = refs
        .iter()
        .map(|r| {
            (nodes[r.target].resource_type == CONFIG
                && matches!(r.attribute.as_str(), SUBNETS | GROUPS))
            .then(|| {
                super::dms_migration::network_context(nodes, refs, e, r.target, (SUBNETS, GROUPS))
                    .into_iter()
                    .collect::<Vec<_>>()
            })
        })
        .collect();
    for (r, context) in refs.iter_mut().zip(contexts) {
        if let Some(context) = context {
            r.service_reference_verified &= context.len() == 1;
            r.endpoint_network_context = context;
        }
    }
}
pub(super) fn bindings(raw: &Input, r: &AttributeReference) -> bool {
    label(&raw.nodes[r.target].resource_type, &r.attribute).is_none()
        || r.sources
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
                    false,
                )
            })
}
pub(super) fn present(raw: &Input, graph: &mut Graph) {
    use super::{lambda_core, service_references};
    for (t, _) in raw
        .nodes
        .iter()
        .enumerate()
        .filter(|(_, n)| n.resource_type == CONFIG)
    {
        let proof = |path| {
            lambda_core::unique(&raw.attributes, t, path)
                .filter(|r| service_references::verified(raw, r))
        };
        let Some((group, groups)) = proof(SUBNETS).zip(proof(GROUPS)) else {
            continue;
        };
        let ([s], [parent]) = (
            group.sources.as_slice(),
            group.endpoint_network_context.as_slice(),
        ) else {
            continue;
        };
        if groups.endpoint_network_context != [*parent] {
            continue;
        }
        let Some(subnets) = lambda_core::unique(&raw.attributes, *s, "subnet_ids")
            .filter(|r| service_references::verified(raw, r))
        else {
            continue;
        };
        if !subnets
            .sources
            .iter()
            .chain(&groups.sources)
            .all(|&member| {
                lambda_core::unique(&raw.attributes, member, "vpc_id")
                    .is_some_and(|r| super::apprunner::bindings(raw, r))
            })
        {
            continue;
        }
        let mut edges = vec![(*s, t)];
        for &subnet in &subnets.sources {
            edges.extend([(*parent, subnet), (subnet, *s)]);
        }
        for &sg in &groups.sources {
            edges.extend([(*parent, sg), (sg, t)]);
        }
        let evidence = edges
            .into_iter()
            .map(|(a, b)| relationships::reference(graph, a, b))
            .collect();
        relationships::record(graph, *parent, t, EdgeKind::Containment, evidence);
        graph.edges.push(Edge {
            kind: EdgeKind::Containment,
            ..Edge::from((*parent, t))
        });
    }
}
#[cfg(test)]
#[path = "../../../tests/unit/semantic/dms_serverless.rs"]
mod tests;
