use crate::model::{
    Action, AttributeReference, Diagnostic, DiagnosticReason, EntityMode, Graph, ReferenceTarget,
    ResourceReference, TerraformEntity,
};
use crate::semantic::Input;
use serde_json::Value;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/service_references.rs"]
pub(super) mod tests;

fn label(resource: &str, attribute: &str) -> Option<&'static str> {
    match (resource, attribute) {
        ("aws_ecs_service", "cluster") => Some("Cluster membership"),
        ("aws_acmpca_certificate" | "aws_acm_certificate", "certificate_authority_arn") => {
            Some("Signing CA reference")
        }
        ("aws_acmpca_certificate_authority_certificate", "certificate_authority_arn") => {
            Some("Target CA reference")
        }
        ("aws_acmpca_certificate_authority_certificate", "certificate") => {
            Some("Installed certificate reference")
        }
        ("aws_glue_catalog_table", "database_name") => Some("Catalog database"),
        ("aws_glue_crawler", "role") | ("aws_glue_job", "role_arn") => {
            Some("Execution role reference")
        }
        ("aws_kinesis_firehose_delivery_stream", attribute) => super::firehose::label(attribute),
        ("aws_efs_mount_target" | "aws_efs_access_point", "file_system_id") => Some("File system"),
        ("aws_efs_mount_target", "subnet_id") => Some("Mount target subnet"),
        ("aws_volume_attachment", "instance_id") => Some("Attached instance"),
        ("aws_volume_attachment", "volume_id") => Some("Attached volume"),
        ("aws_secretsmanager_secret_version", "secret_id") => Some("Version secret"),
        ("aws_kms_alias", "target_key_id") => Some("Alias key"),
        ("aws_sqs_queue", "kms_master_key_id")
        | ("aws_db_instance" | "aws_rds_cluster", "kms_key_id")
        | (
            "aws_s3_bucket_server_side_encryption_configuration",
            "rule.apply_server_side_encryption_by_default.kms_master_key_id",
        ) => Some("Encryption key reference"),
        ("aws_s3_bucket_server_side_encryption_configuration", "bucket") => {
            Some("Configured bucket")
        }
        ("aws_acm_certificate_validation" | "aws_lb_listener", "certificate_arn")
        | ("aws_cloudfront_distribution", "viewer_certificate.acm_certificate_arn") => {
            Some("Certificate reference")
        }
        ("aws_route53_record", "zone_id") => Some("Hosted zone"),
        ("aws_route53_record", "alias.name") => Some("Alias target"),
        ("aws_route53_zone", "vpc.vpc_id") => Some("Associated VPC"),
        _ => super::iam::label(resource, attribute),
    }
}

fn source_fields(resource: &str, attribute: &str, source: &str) -> &'static [&'static str] {
    match (resource, attribute, source) {
        ("aws_ecs_service", "cluster", "aws_ecs_cluster") => &["id", "arn", "name"],
        (
            "aws_acmpca_certificate"
            | "aws_acm_certificate"
            | "aws_acmpca_certificate_authority_certificate",
            "certificate_authority_arn",
            "aws_acmpca_certificate_authority",
        ) => &["arn"],
        (
            "aws_acmpca_certificate_authority_certificate",
            "certificate",
            "aws_acmpca_certificate",
        ) => &["certificate"],
        ("aws_glue_catalog_table", "database_name", "aws_glue_catalog_database") => &["name"],
        ("aws_glue_crawler", "role", "aws_iam_role") => &["arn"],
        ("aws_glue_job", "role_arn", "aws_iam_role") => &["arn"],
        (
            "aws_kinesis_firehose_delivery_stream",
            "kinesis_source_configuration.kinesis_stream_arn",
            "aws_kinesis_stream",
        )
        | (
            "aws_kinesis_firehose_delivery_stream",
            "extended_s3_configuration.bucket_arn"
            | "extended_s3_configuration.s3_backup_configuration.bucket_arn",
            "aws_s3_bucket",
        ) => &["arn"],
        (
            "aws_efs_mount_target" | "aws_efs_access_point",
            "file_system_id",
            "aws_efs_file_system",
        )
        | ("aws_efs_mount_target", "subnet_id", "aws_subnet") => &["id"],
        ("aws_volume_attachment", "instance_id", "aws_instance")
        | ("aws_volume_attachment", "volume_id", "aws_ebs_volume") => &["id"],
        ("aws_secretsmanager_secret_version", "secret_id", "aws_secretsmanager_secret") => {
            &["id", "arn"]
        }
        ("aws_kms_alias", "target_key_id", "aws_kms_key")
        | ("aws_sqs_queue", "kms_master_key_id", "aws_kms_key")
        | ("aws_db_instance" | "aws_rds_cluster", "kms_key_id", "aws_kms_key")
        | (
            "aws_s3_bucket_server_side_encryption_configuration",
            "rule.apply_server_side_encryption_by_default.kms_master_key_id",
            "aws_kms_key",
        ) => &["id", "key_id", "arn"],
        ("aws_s3_bucket_server_side_encryption_configuration", "bucket", "aws_s3_bucket") => {
            &["id", "bucket"]
        }
        (
            "aws_acm_certificate_validation" | "aws_lb_listener",
            "certificate_arn",
            "aws_acm_certificate",
        )
        | (
            "aws_cloudfront_distribution",
            "viewer_certificate.acm_certificate_arn",
            "aws_acm_certificate",
        ) => &["arn", "id"],
        ("aws_lb_listener", "certificate_arn", "aws_acm_certificate_validation")
        | (
            "aws_cloudfront_distribution",
            "viewer_certificate.acm_certificate_arn",
            "aws_acm_certificate_validation",
        ) => &["certificate_arn"],
        ("aws_route53_record", "zone_id", "aws_route53_zone") => &["zone_id", "id"],
        ("aws_route53_record", "alias.name", "aws_lb") => &["dns_name"],
        ("aws_route53_record", "alias.name", "aws_cloudfront_distribution") => &["domain_name"],
        ("aws_route53_record", "alias.zone_id", "aws_lb") => &["zone_id"],
        ("aws_route53_record", "alias.zone_id", "aws_cloudfront_distribution") => {
            &["hosted_zone_id"]
        }
        ("aws_route53_zone", "vpc.vpc_id", "aws_vpc") => &["id"],
        _ => &[],
    }
}

fn cluster_binding_matches(
    consumer: Option<&crate::model::ProviderConfiguration>,
    endpoint: Option<&crate::model::ProviderConfiguration>,
) -> bool {
    consumer.is_some_and(|config| {
        !config.account_conflict && config.region.is_some() && endpoint == Some(config)
    })
}

pub(super) fn scalar<'a>(value: &'a Value, unknown: &Value, path: &[&str]) -> Option<&'a str> {
    if unknown == &Value::Bool(true) {
        return None;
    }
    if path.is_empty() {
        return value.as_str().filter(|s| !s.is_empty());
    }
    if let Some(values) = value.as_array() {
        let [value] = values.as_slice() else {
            return None;
        };
        if unknown.as_array().is_some_and(|u| u.len() != 1) {
            return None;
        }
        return scalar(value, &unknown[0], path);
    }
    scalar(&value[path[0]], &unknown[path[0]], &path[1..])
}

pub(crate) fn annotate(
    nodes: &[TerraformEntity],
    references: &mut [AttributeReference],
    evidence: &[(&Value, &Value)],
) {
    for reference in references.iter_mut() {
        let node = &nodes[reference.target];
        if super::iam::label(&node.resource_type, &reference.attribute).is_some() {
            reference.service_reference_verified = super::iam::verified(nodes, reference, evidence);
            continue;
        }
        let [source] = reference.sources.as_slice() else {
            continue;
        };
        let endpoint = &nodes[*source];
        if !reference.complete
            || !reference.instances_explicit
            || !reference.issues.is_empty()
            || !node.provider.is_aws()
            || !endpoint.provider.is_aws()
            || (node.mode != EntityMode::Managed
                && node.resource_type != "aws_secretsmanager_secret_version")
            || endpoint.mode != EntityMode::Managed
            || node.deposed_key.is_some()
            || endpoint.deposed_key.is_some()
            || node.action == Action::Delete
            || endpoint.action == Action::Delete
        {
            continue;
        }
        let fields = source_fields(
            &node.resource_type,
            &reference.attribute,
            &endpoint.resource_type,
        );
        let (value, unknown) = evidence[reference.target];
        let (target, target_unknown) = evidence[*source];
        let actual = scalar(
            value,
            unknown,
            &reference.attribute.split('.').collect::<Vec<_>>(),
        );
        reference.service_reference_verified = fields.iter().any(|field| {
            reference.source_attributes.len() == 1
                && reference.source_attributes.contains(*field)
                && actual.is_some()
                && actual == scalar(target, target_unknown, &[*field])
        });
        if node.resource_type == "aws_ecs_service" && reference.attribute == "cluster" {
            reference.service_reference_verified &= cluster_binding_matches(
                node.provider_configuration.as_ref(),
                endpoint.provider_configuration.as_ref(),
            );
        }
        if node.resource_type == "aws_glue_catalog_table" && reference.attribute == "database_name"
        {
            reference.service_reference_verified &= super::glue::catalog_verified(
                node,
                endpoint,
                value,
                unknown,
                target,
                target_unknown,
            );
        }
        if node.resource_type == "aws_kinesis_firehose_delivery_stream" {
            reference.service_reference_verified &=
                super::firehose::configuration_verified(&reference.attribute, value, unknown);
        }
        if reference.service_reference_verified
            && reference.attribute == "alias.name"
            && endpoint.resource_type == "aws_lb"
        {
            reference.service_reference_verified = matches!(
                scalar(target, target_unknown, &["load_balancer_type"]),
                Some("application" | "network")
            );
        }
    }
    let aliases: Vec<_> = references
        .iter()
        .filter(|r| r.attribute == "alias.zone_id" && r.service_reference_verified)
        .map(|r| (r.target, r.sources.clone()))
        .collect();
    for reference in references
        .iter_mut()
        .filter(|r| r.attribute == "alias.name")
    {
        reference.service_reference_verified &= aliases
            .iter()
            .any(|(target, sources)| *target == reference.target && *sources == reference.sources);
    }
}

pub(super) fn verified(raw: &Input, reference: &AttributeReference) -> bool {
    reference.service_reference_verified
        && super::iam::bindings_compatible(&raw.nodes, reference)
        && (raw.nodes[reference.target].resource_type != "aws_ecs_service"
            || reference.sources.iter().all(|&source| {
                cluster_binding_matches(
                    raw.nodes[reference.target].provider_configuration.as_ref(),
                    raw.nodes[source].provider_configuration.as_ref(),
                )
            }))
}

pub(super) fn present(raw: &Input, graph: &mut Graph) {
    for node in &mut graph.nodes {
        for reference in &mut node.references {
            reference.show_spatial |= reference.targets.iter().any(|target| {
                raw.nodes.iter().any(|endpoint| {
                    endpoint.provider.is_aws()
                        && matches!(
                            endpoint.resource_type.as_str(),
                            "aws_cloudwatch_log_group" | "aws_ecs_cluster"
                        )
                        && endpoint.entity.provenance.contains(&target.id)
                })
            });
        }
    }
    for reference in &raw.attributes {
        let node = &raw.nodes[reference.target];
        if !node.provider.is_aws() {
            continue;
        }
        let Some(label) = label(&node.resource_type, &reference.attribute) else {
            continue;
        };
        let verified = verified(raw, reference);
        if verified {
            graph.nodes[reference.target]
                .references
                .retain(|r| r.dependency || r.attribute != reference.attribute);
        }
        graph.nodes[reference.target]
            .references
            .push(ResourceReference {
                kind: if verified
                    && matches!(
                        node.resource_type.as_str(),
                        "aws_iam_group_membership"
                            | "aws_iam_user_group_membership"
                            | "aws_ecs_service"
                    ) {
                    crate::model::ReferenceKind::Membership
                } else {
                    crate::model::ReferenceKind::Configured
                },
                show_spatial: true,
                label,
                attribute: reference.attribute.clone(),
                targets: if verified {
                    reference
                        .sources
                        .iter()
                        .map(|&source| ReferenceTarget {
                            state: None,
                            id: raw.nodes[source].entity.provenance[0].clone(),
                            placement: raw.nodes[source].placement,
                        })
                        .collect()
                } else {
                    Vec::new()
                },
                complete: verified,
                dependency: false,
                issues: if verified {
                    Default::default()
                } else {
                    [DiagnosticReason::UnprovenEndpointValue].into()
                },
            });
    }
}

pub(super) fn diagnostics(raw: &Input) -> Vec<Diagnostic> {
    raw.attributes
        .iter()
        .filter_map(|reference| {
            let node = &raw.nodes[reference.target];
            (node.provider.is_aws()
                && label(&node.resource_type, &reference.attribute).is_some()
                && !verified(raw, reference))
            .then(|| Diagnostic {
                address: node.address.clone(),
                deposed_key: node.deposed_key.clone(),
                attribute: reference.attribute.clone(),
                reason: DiagnosticReason::UnprovenEndpointValue,
            })
        })
        .collect()
}
