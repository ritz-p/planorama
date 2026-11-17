use crate::model::{AttributeReference, TerraformEntity};
use serde_json::Value;
use std::collections::BTreeSet;

mod evaluation;
mod guardrails;
const CUSTOM: &str = "aws_bedrock_custom_model";
const ENFORCED: &str = "aws_bedrock_enforced_guardrail_configuration";
const EVALUATION: &str = "aws_bedrock_evaluation_job";
const GUARDRAIL: &str = "aws_bedrock_guardrail";
const VERSION: &str = "aws_bedrock_guardrail_version";
const PROFILE: &str = "aws_bedrock_inference_profile";
const BATCH: &str = "aws_bedrock_model_invocation_job";
const LOGGING: &str = "aws_bedrock_model_invocation_logging_configuration";
const THROUGHPUT: &str = "aws_bedrock_provisioned_model_throughput";
const FLOW: &str = "evaluation_config.human.human_workflow_config.flow_definition_arn";
const RETRIEVE: &str =
    "inference_config.rag_config.knowledge_base_config.retrieve_config.knowledge_base_id";
const GENERATE: &str = "inference_config.rag_config.knowledge_base_config.retrieve_and_generate_config.knowledge_base_id";
const SUBNETS: &str = "vpc_config.subnet_ids";
const GROUPS: &str = "vpc_config.security_group_ids";

pub(crate) fn resource(kind: &str) -> bool {
    matches!(
        kind,
        CUSTOM
            | ENFORCED
            | EVALUATION
            | GUARDRAIL
            | VERSION
            | PROFILE
            | BATCH
            | LOGGING
            | THROUGHPUT
    )
}
pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    match (kind, path) {
        (CUSTOM, "base_model_identifier") => Some("Customization base model"),
        (CUSTOM | BATCH, "role_arn") | (EVALUATION, "role_arn") => Some("Bedrock job role"),
        (CUSTOM, "custom_model_kms_key_id")
        | (EVALUATION, "customer_encryption_key_id")
        | (GUARDRAIL, "kms_key_arn")
        | (BATCH, "output_data_config.s3_output_data_config.s3_encryption_key_id") => {
            Some("Bedrock encryption key")
        }
        (CUSTOM | BATCH, SUBNETS) => Some("Bedrock job data subnets"),
        (CUSTOM | BATCH, GROUPS) => Some("Bedrock job data security groups"),
        (ENFORCED, "guardrail_arn") => Some("Enforced guardrail"),
        (ENFORCED, "guardrail_version") => Some("Enforced guardrail version"),
        (VERSION, "guardrail_arn") => Some("Version guardrail"),
        (EVALUATION, FLOW) => Some("Human evaluation flow"),
        (EVALUATION, RETRIEVE | GENERATE) => Some("Evaluation knowledge base"),
        (PROFILE, "model_source.copy_from") => Some("Inference profile source"),
        (BATCH, "model_id") => Some("Batch inference profile"),
        (LOGGING, "logging_config.cloudwatch_config.log_group_name") => Some("Bedrock log group"),
        (LOGGING, "logging_config.cloudwatch_config.role_arn") => Some("Bedrock logging role"),
        (
            LOGGING,
            "logging_config.s3_config.bucket_name"
            | "logging_config.cloudwatch_config.large_data_delivery_s3_config.bucket_name",
        ) => Some("Bedrock logging bucket"),
        (THROUGHPUT, "model_arn") => Some("Provisioned custom model"),
        _ => None,
    }
}
fn fields(kind: &str, path: &str, target: &str) -> &'static [&'static str] {
    match (kind, path, target) {
        (CUSTOM, "base_model_identifier", CUSTOM) | (THROUGHPUT, "model_arn", CUSTOM) => {
            &["custom_model_arn"]
        }
        (CUSTOM | BATCH | EVALUATION, "role_arn", "aws_iam_role")
        | (LOGGING, "logging_config.cloudwatch_config.role_arn", "aws_iam_role") => &["arn"],
        (CUSTOM, "custom_model_kms_key_id", "aws_kms_key")
        | (EVALUATION, "customer_encryption_key_id", "aws_kms_key")
        | (GUARDRAIL, "kms_key_arn", "aws_kms_key")
        | (BATCH, "output_data_config.s3_output_data_config.s3_encryption_key_id", "aws_kms_key") => {
            &["arn"]
        }
        (CUSTOM | BATCH, SUBNETS, "aws_subnet")
        | (CUSTOM | BATCH, GROUPS, "aws_security_group") => &["id"],
        (ENFORCED | VERSION, "guardrail_arn", GUARDRAIL) | (ENFORCED, "guardrail_arn", VERSION) => {
            &["guardrail_arn"]
        }
        (ENFORCED, "guardrail_version", VERSION) => &["version"],
        (EVALUATION, FLOW, "aws_sagemaker_flow_definition") => &["arn"],
        (EVALUATION, RETRIEVE | GENERATE, "aws_bedrockagent_knowledge_base") => &["id"],
        (PROFILE, "model_source.copy_from", PROFILE) => &["arn"],
        (BATCH, "model_id", PROFILE) => &["arn", "id"],
        (
            LOGGING,
            "logging_config.cloudwatch_config.log_group_name",
            "aws_cloudwatch_log_group",
        ) => &["name"],
        (
            LOGGING,
            "logging_config.s3_config.bucket_name"
            | "logging_config.cloudwatch_config.large_data_delivery_s3_config.bucket_name",
            "aws_s3_bucket",
        ) => &["id", "bucket"],
        _ => &[],
    }
}
pub(crate) fn indexed(kind: &str, path: &str) -> bool {
    resource(kind)
        && path.split('.').any(|s| s.parse::<usize>().is_ok())
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
fn known<'a>(e: &[(&'a Value, &Value)], index: usize, path: &str) -> Option<&'a str> {
    super::nested_references::value(e[index].0, e[index].1, path)
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
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
fn qualify(
    nodes: &[TerraformEntity],
    refs: &[AttributeReference],
    e: &[(&Value, &Value)],
    r: &AttributeReference,
) -> bool {
    match nodes[r.target].resource_type.as_str() {
        VERSION => guardrails::published(known(e, r.target, "version")),
        ENFORCED => guardrails::enforced(nodes, refs, e, r),
        EVALUATION => evaluation::qualify(e, r.target, &r.attribute),
        _ => true,
    }
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
            Some(
                qualify(nodes, refs, e, r)
                    && super::nested_references::verify(nodes, refs, e, r, fields, |s, _| {
                        compatible(&nodes[r.target], &nodes[s])
                    })
                    .is_some(),
            )
        })
        .collect();
    for (r, result) in refs.iter_mut().zip(results) {
        if let Some(valid) = result {
            r.service_reference_verified = valid;
            if valid {
                r.complete = true;
            }
        }
    }
    let contexts: Vec<_> = refs
        .iter()
        .map(|r| {
            if !matches!(nodes[r.target].resource_type.as_str(), CUSTOM | BATCH)
                || !matches!(r.attribute.as_str(), SUBNETS | GROUPS)
            {
                return None;
            }
            let context = || {
                let subnets = super::lambda_core::unique(refs, r.target, SUBNETS)?;
                let groups = super::lambda_core::unique(refs, r.target, GROUPS)?;
                if !subnets.service_reference_verified || !groups.service_reference_verified {
                    return None;
                }
                let parents = subnets
                    .sources
                    .iter()
                    .chain(&groups.sources)
                    .map(|&s| super::lambda_core::parent(nodes, refs, e, s))
                    .collect::<Option<BTreeSet<_>>>()?;
                (parents.len() == 1).then(|| *parents.first().unwrap())
            };
            Some(context().into_iter().collect::<Vec<_>>())
        })
        .collect();
    for (r, context) in refs.iter_mut().zip(contexts) {
        if let Some(context) = context {
            r.service_reference_verified &= context.len() == 1;
            r.endpoint_network_context = context;
        }
    }
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
#[path = "../../../tests/unit/semantic/bedrock_models.rs"]
mod tests;
