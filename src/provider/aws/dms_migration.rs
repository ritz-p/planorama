use crate::model::{AttributeReference, TerraformEntity};
use serde_json::Value;
mod engines;
mod network;
const PROFILE: &str = "aws_dms_instance_profile";
const PROVIDER: &str = "aws_dms_data_provider";
const PROJECT: &str = "aws_dms_migration_project";
const GROUP: &str = "aws_dms_replication_subnet_group";
pub(crate) fn resource(kind: &str) -> bool {
    matches!(kind, PROFILE | PROVIDER | PROJECT)
}
pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    match (kind, path) {
        (PROFILE, "subnet_group_identifier") => Some("DMS profile subnet group"),
        (PROFILE, "vpc_security_group_ids") => Some("DMS profile security groups"),
        (PROFILE, "kms_key_arn") => Some("DMS profile encryption key"),
        (PROJECT, "instance_profile_arn") => Some("DMS migration instance profile"),
        (PROJECT, "source_data_provider_descriptor.data_provider_arn") => {
            Some("DMS migration source provider")
        }
        (PROJECT, "target_data_provider_descriptor.data_provider_arn") => {
            Some("DMS migration target provider")
        }
        (
            PROJECT,
            "source_data_provider_descriptor.secrets_manager_access_role_arn"
            | "target_data_provider_descriptor.secrets_manager_access_role_arn"
            | "schema_conversion_application_attributes.s3_bucket_role_arn",
        ) => Some("DMS migration access role"),
        (
            PROJECT,
            "source_data_provider_descriptor.secrets_manager_secret_id"
            | "target_data_provider_descriptor.secrets_manager_secret_id",
        ) => Some("DMS migration credentials secret"),
        (PROVIDER, _) => engines::field(path).map(|(_, _, label)| label),
        _ => None,
    }
}
fn fields(kind: &str, path: &str, target: &str) -> &'static [&'static str] {
    match (kind, path, target) {
        (PROFILE, "subnet_group_identifier", GROUP) => &["id"],
        (PROFILE, "vpc_security_group_ids", "aws_security_group")
        | (GROUP, "subnet_ids", "aws_subnet") => &["id"],
        (PROFILE, "kms_key_arn", "aws_kms_key")
        | (PROJECT, "instance_profile_arn", PROFILE)
        | (
            PROJECT,
            "source_data_provider_descriptor.data_provider_arn"
            | "target_data_provider_descriptor.data_provider_arn",
            PROVIDER,
        ) => &["arn"],
        (
            PROJECT,
            "source_data_provider_descriptor.secrets_manager_access_role_arn"
            | "target_data_provider_descriptor.secrets_manager_access_role_arn"
            | "schema_conversion_application_attributes.s3_bucket_role_arn",
            "aws_iam_role",
        ) => &["arn"],
        (
            PROJECT,
            "source_data_provider_descriptor.secrets_manager_secret_id"
            | "target_data_provider_descriptor.secrets_manager_secret_id",
            "aws_secretsmanager_secret",
        ) => &["arn", "id", "name"],
        (PROVIDER, _, _) => engines::field(path)
            .filter(|(kind, _, _)| *kind == target)
            .map_or(&[], |(_, fields, _)| fields),
        _ => &[],
    }
}
pub(crate) fn indexed(kind: &str, path: &str) -> bool {
    path.split('.').any(|s| s.parse::<usize>().is_ok())
        && label(
            kind,
            &path
                .split('.')
                .filter(|s| s.parse::<usize>().is_err())
                .collect::<Vec<_>>()
                .join("."),
        )
        .is_some()
}
fn compatible(l: &TerraformEntity, r: &TerraformEntity) -> bool {
    let left = super::scope::scope(l);
    let right = super::scope::scope(r);
    super::eventbridge::bindings(
        l.provider_configuration.as_ref(),
        r.provider_configuration.as_ref(),
        left.as_ref().and_then(|s| s.region.as_ref()),
        right.as_ref().and_then(|s| s.region.as_ref()),
        r.resource_type == "aws_iam_role",
    )
}
fn verify(
    nodes: &[TerraformEntity],
    refs: &[AttributeReference],
    e: &[(&Value, &Value)],
    r: &AttributeReference,
) -> bool {
    super::nested_references::verify_collections(
        nodes,
        refs,
        e,
        r,
        fields,
        |s, path| {
            super::nested_references::value(e[r.target].0, e[r.target].1, path).is_some_and(
                |value| {
                    if r.attribute == "vpc_security_group_ids" {
                        value.is_array()
                    } else {
                        value.is_string()
                    }
                },
            ) && compatible(&nodes[r.target], &nodes[s])
                && (nodes[r.target].resource_type != PROVIDER
                    || engines::qualify(e, r.target, path))
        },
        &[
            "source_data_provider_descriptor",
            "target_data_provider_descriptor",
        ],
    )
    .is_some()
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
            Some(verify(nodes, refs, e, r))
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
    network::annotate(nodes, refs, e);
}
pub(super) fn bindings(raw: &crate::semantic::Input, r: &AttributeReference) -> bool {
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
                    right.resource_type == "aws_iam_role",
                )
            })
}
#[cfg(test)]
#[path = "../../../tests/unit/semantic/dms_migration.rs"]
mod tests;
