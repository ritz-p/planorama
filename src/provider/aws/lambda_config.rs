use crate::model::{Action, AttributeReference, EntityMode, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;
use std::collections::BTreeSet;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/lambda_config.rs"]
mod tests;

pub(super) fn label(resource: &str, attribute: &str) -> Option<&'static str> {
    match (resource, attribute) {
        ("aws_lambda_code_signing_config", "allowed_publishers.signing_profile_version_arns") => {
            Some("Signing profile versions")
        }
        ("aws_lambda_function", "role") => Some("Execution role reference"),
        ("aws_lambda_function", "code_signing_config_arn") => Some("Code signing configuration"),
        ("aws_lambda_function", "dead_letter_config.target_arn") => Some("Dead-letter destination"),
        ("aws_lambda_function", "file_system_config.arn") => Some("File system access point"),
        ("aws_lambda_function", "logging_config.log_group") => Some("Configured log group"),
        ("aws_lambda_function", "kms_key_arn") => Some("Encryption key reference"),
        ("aws_lambda_function", "source_kms_key_arn") => Some("Source encryption key"),
        (
            "aws_lambda_function",
            "capacity_provider_config.lambda_managed_instances_capacity_provider_config.capacity_provider_arn",
        ) => Some("Lambda capacity provider"),
        ("aws_lambda_function", "vpc_config.subnet_ids") => Some("Configured subnets"),
        ("aws_lambda_function", "vpc_config.security_group_ids") => {
            Some("Configured security groups")
        }
        _ => None,
    }
}

fn field(attribute: &str, kind: &str) -> Option<(&'static str, bool)> {
    match (attribute, kind) {
        ("allowed_publishers.signing_profile_version_arns", "aws_signer_signing_profile") => {
            Some(("version_arn", true))
        }
        ("role", "aws_iam_role")
        | ("code_signing_config_arn", "aws_lambda_code_signing_config")
        | ("dead_letter_config.target_arn", "aws_sns_topic" | "aws_sqs_queue")
        | ("file_system_config.arn", "aws_efs_access_point" | "aws_s3files_access_point")
        | ("kms_key_arn" | "source_kms_key_arn", "aws_kms_key")
        | (
            "capacity_provider_config.lambda_managed_instances_capacity_provider_config.capacity_provider_arn",
            "aws_lambda_capacity_provider",
        ) => Some(("arn", false)),
        ("logging_config.log_group", "aws_cloudwatch_log_group") => Some(("name", false)),
        ("vpc_config.subnet_ids", "aws_subnet")
        | ("vpc_config.security_group_ids", "aws_security_group") => Some(("id", true)),
        _ => None,
    }
}

pub(super) fn bindings(raw: &Input, reference: &AttributeReference) -> bool {
    let node = &raw.nodes[reference.target];
    reference.sources.iter().all(|&source| {
        let endpoint = &raw.nodes[source];
        super::eventbridge::bindings(
            node.provider_configuration.as_ref(),
            endpoint.provider_configuration.as_ref(),
            node.entity.scope.as_ref().and_then(|s| s.region.as_ref()),
            endpoint
                .entity
                .scope
                .as_ref()
                .and_then(|s| s.region.as_ref()),
            endpoint.resource_type == "aws_iam_role",
        )
    })
}

pub(crate) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    evidence: &[(&Value, &Value)],
) {
    let current = |n: &TerraformEntity| {
        n.provider.is_aws()
            && n.mode == EntityMode::Managed
            && n.action != Action::Delete
            && n.deposed_key.is_none()
    };
    for r in refs {
        let node = &nodes[r.target];
        if label(&node.resource_type, &r.attribute).is_none() {
            continue;
        }
        r.service_reference_verified = false;
        if !current(node)
            || !r.complete
            || !r.instances_explicit
            || !r.issues.is_empty()
            || r.sources.is_empty()
            || r.source_attributes.len() != 1
        {
            continue;
        }
        let Some((output, collection)) = field(&r.attribute, &nodes[r.sources[0]].resource_type)
        else {
            continue;
        };
        if !r.source_attributes.contains(output) || (!collection && r.sources.len() != 1) {
            continue;
        }
        let (value, mask) = evidence[r.target];
        let mut block = value;
        if !r
            .attribute
            .split('.')
            .collect::<Vec<_>>()
            .split_last()
            .unwrap()
            .1
            .iter()
            .all(|field| {
                let Some(entries) = block[*field].as_array() else {
                    return false;
                };
                let [entry] = entries.as_slice() else {
                    return false;
                };
                block = entry;
                true
            })
        {
            continue;
        }
        let Some(actual) = super::streaming::values(value, mask, &r.attribute, collection) else {
            continue;
        };
        let expected = r
            .sources
            .iter()
            .map(|&source| {
                let endpoint = &nodes[source];
                (current(endpoint)
                    && field(&r.attribute, &endpoint.resource_type) == Some((output, collection))
                    && super::eventbridge::bindings(
                        node.provider_configuration.as_ref(),
                        endpoint.provider_configuration.as_ref(),
                        super::scope::scope(node)
                            .as_ref()
                            .and_then(|s| s.region.as_ref()),
                        super::scope::scope(endpoint)
                            .as_ref()
                            .and_then(|s| s.region.as_ref()),
                        endpoint.resource_type == "aws_iam_role",
                    ))
                .then(|| super::service_evidence::known(evidence, source, output))
                .flatten()
            })
            .collect::<Option<BTreeSet<_>>>();
        r.service_reference_verified = expected
            .is_some_and(|expected| expected.len() == r.sources.len() && expected == actual);
    }
}
