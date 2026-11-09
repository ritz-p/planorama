use crate::model::{AttributeReference, Edge, EdgeKind, Graph, TerraformEntity};
use crate::semantic::{Input, relationships};
use serde_json::Value;
use std::collections::BTreeSet;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/s3files.rs"]
mod tests;

pub(super) fn resource(kind: &str) -> bool {
    matches!(
        kind,
        "aws_s3files_file_system"
            | "aws_s3files_access_point"
            | "aws_s3files_file_system_policy"
            | "aws_s3files_synchronization_configuration"
            | "aws_s3files_mount_target"
    )
}
pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    match (kind, path) {
        ("aws_s3files_file_system", "bucket") => Some("Backing bucket"),
        ("aws_s3files_file_system", "role_arn") => Some("Configured access role"),
        ("aws_s3files_file_system", "kms_key_id") => Some("File system encryption key"),
        (
            "aws_s3files_access_point"
            | "aws_s3files_file_system_policy"
            | "aws_s3files_synchronization_configuration"
            | "aws_s3files_mount_target",
            "file_system_id",
        ) => Some("S3 Files file system"),
        ("aws_s3files_mount_target", "subnet_id") => Some("Mount target subnet"),
        ("aws_s3files_mount_target", "security_groups") => Some("Mount target security groups"),
        _ => None,
    }
}
fn fields(kind: &str, path: &str, source: &str) -> &'static [&'static str] {
    match (kind, path, source) {
        ("aws_s3files_file_system", "bucket", "aws_s3_bucket")
        | ("aws_s3files_file_system", "role_arn", "aws_iam_role") => &["arn"],
        ("aws_s3files_file_system", "kms_key_id", _)
            if !super::kms::key_outputs(source).is_empty() =>
        {
            &["arn"]
        }
        (_, "file_system_id", "aws_s3files_file_system") if resource(kind) => &["id"],
        ("aws_s3files_mount_target", "subnet_id", "aws_subnet")
        | ("aws_s3files_mount_target", "security_groups", "aws_security_group") => &["id"],
        _ => &[],
    }
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
            let verified =
                super::nested_references::verify(nodes, refs, evidence, r, fields, |_, path| {
                    let (v, m) = evidence[r.target];
                    super::nested_references::value(v, m, path).is_some_and(|v| {
                        if r.attribute == "security_groups" {
                            v.is_array()
                        } else {
                            v.is_string()
                        }
                    })
                })
                .is_some();
            let context =
                if verified && matches!(r.attribute.as_str(), "subnet_id" | "security_groups") {
                    r.sources
                        .iter()
                        .map(|&s| super::lambda_core::parent(nodes, refs, evidence, s))
                        .collect::<Option<BTreeSet<_>>>()
                        .filter(|v| v.len() == 1)
                        .map(|v| v.into_iter().collect())
                        .unwrap_or_default()
                } else {
                    Vec::new()
                };
            Some((verified, context))
        })
        .collect();
    for (r, result) in refs.iter_mut().zip(results) {
        if let Some((valid, context)) = result {
            r.service_reference_verified = valid;
            r.endpoint_network_context = context;
        }
    }
    let network: Vec<_> = refs
        .iter()
        .map(|r| {
            if nodes[r.target].resource_type != "aws_s3files_mount_target"
                || !matches!(r.attribute.as_str(), "subnet_id" | "security_groups")
            {
                return None;
            }
            let proof = |p| {
                super::lambda_core::unique(refs, r.target, p).filter(|r| {
                    r.service_reference_verified && r.endpoint_network_context.len() == 1
                })
            };
            Some(
                proof("subnet_id")
                    .zip(proof("security_groups"))
                    .is_some_and(|(a, b)| a.endpoint_network_context == b.endpoint_network_context),
            )
        })
        .collect();
    for (r, valid) in refs.iter_mut().zip(network) {
        if let Some(valid) = valid {
            r.service_reference_verified &= valid;
        }
    }
}
pub(super) fn bindings(raw: &Input, r: &AttributeReference) -> bool {
    if label(&raw.nodes[r.target].resource_type, &r.attribute).is_none() {
        return true;
    }
    r.sources.iter().all(|&s| {
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
    for (target, node) in raw
        .nodes
        .iter()
        .enumerate()
        .filter(|(_, n)| n.resource_type == "aws_s3files_mount_target" && n.provider.is_aws())
    {
        let proof = |path| {
            super::lambda_core::unique(&raw.attributes, target, path)
                .filter(|r| super::service_references::verified(raw, r))
        };
        let Some((subnet, groups)) = proof("subnet_id").zip(proof("security_groups")) else {
            continue;
        };
        let ([s], [vpc]) = (
            subnet.sources.as_slice(),
            subnet.endpoint_network_context.as_slice(),
        ) else {
            continue;
        };
        if groups.endpoint_network_context != [*vpc] {
            continue;
        }
        let parent = &raw.nodes[*vpc];
        if !super::service_references::cluster_binding_matches(
            node.provider_configuration.as_ref(),
            parent.provider_configuration.as_ref(),
            node.entity.scope.as_ref().and_then(|s| s.region.as_ref()),
            parent.entity.scope.as_ref().and_then(|s| s.region.as_ref()),
        ) {
            continue;
        }
        let evidence = std::iter::once(relationships::reference(graph, *s, target))
            .chain(
                groups
                    .sources
                    .iter()
                    .map(|&g| relationships::reference(graph, g, target)),
            )
            .collect();
        relationships::record(graph, *s, target, EdgeKind::Containment, evidence);
        graph.edges.push(Edge {
            kind: EdgeKind::Containment,
            ..Edge::from((*s, target))
        });
    }
}
