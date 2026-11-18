use crate::model::{AttributeReference, TerraformEntity};
use serde_json::Value;
mod objects;
mod qualify;
const AGENT: &str = "aws_bedrockagent_agent";
const GROUP: &str = "aws_bedrockagent_agent_action_group";
const ALIAS: &str = "aws_bedrockagent_agent_alias";
const COLLABORATOR: &str = "aws_bedrockagent_agent_collaborator";
const ASSOCIATION: &str = "aws_bedrockagent_agent_knowledge_base_association";
const SOURCE: &str = "aws_bedrockagent_data_source";
const FLOW: &str = "aws_bedrockagent_flow";
const KB: &str = "aws_bedrockagent_knowledge_base";
const PROMPT: &str = "aws_bedrockagent_prompt";
pub(crate) fn resource(kind: &str) -> bool {
    matches!(
        kind,
        AGENT | GROUP | ALIAS | COLLABORATOR | ASSOCIATION | SOURCE | FLOW | KB | PROMPT
    )
}
pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    match (kind, path) {
        (AGENT, "guardrail_configuration") => Some("Agent guardrail configuration"),
        (AGENT, "prompt_override_configuration") => Some("Agent parser Lambda"),
        (ALIAS, "routing_configuration") => Some("Alias provisioned throughput"),
        (AGENT, "agent_resource_role_arn") | (FLOW, "execution_role_arn") | (KB, "role_arn") => {
            Some("Bedrock execution role")
        }
        (AGENT | FLOW | PROMPT, "customer_encryption_key_arn")
        | (SOURCE, "server_side_encryption_configuration.kms_key_arn") => {
            Some("Bedrock encryption key")
        }
        (GROUP | ALIAS | COLLABORATOR | ASSOCIATION, "agent_id") => Some("Bedrock agent owner"),
        (ASSOCIATION | SOURCE, "knowledge_base_id") => Some("Bedrock knowledge base"),
        (COLLABORATOR, "agent_descriptor.alias_arn") => Some("Collaborator agent alias"),
        (GROUP, "action_group_executor.lambda") => Some("Action group Lambda executor"),
        (GROUP, "api_schema.s3.s3_bucket_name") => Some("Action group schema bucket"),
        (SOURCE, "data_source_configuration.s3_configuration.bucket_arn") => {
            Some("Knowledge ingestion bucket")
        }
        (FLOW, "definition.node.configuration.agent.agent_alias_arn") => Some("Flow agent alias"),
        (FLOW, "definition.node.configuration.knowledge_base.knowledge_base_id") => {
            Some("Flow knowledge base")
        }
        (FLOW, "definition.node.configuration.lambda_function.lambda_arn") => {
            Some("Flow Lambda function")
        }
        (FLOW, "definition.node.configuration.prompt.source_configuration.resource.prompt_arn") => {
            Some("Flow prompt")
        }
        (
            FLOW,
            "definition.node.configuration.retrieval.service_configuration.s3.bucket_name"
            | "definition.node.configuration.storage.service_configuration.s3.bucket_name",
        ) => Some("Flow S3 bucket"),
        (
            KB,
            "storage_configuration.opensearch_serverless_configuration.collection_arn"
            | "storage_configuration.opensearch_managed_cluster_configuration.domain_arn"
            | "storage_configuration.rds_configuration.resource_arn"
            | "storage_configuration.neptune_analytics_configuration.graph_arn"
            | "storage_configuration.s3_vectors_configuration.index_arn",
        ) => Some("Knowledge storage backend"),
        (
            KB,
            "knowledge_base_configuration.kendra_knowledge_base_configuration.kendra_index_arn",
        ) => Some("Knowledge Kendra index"),
        (PROMPT, "variant.model_id") => Some("Prompt inference profile"),
        (PROMPT, "variant.gen_ai_resource.agent.agent_identifier") => Some("Prompt agent"),
        _ => extra(kind, path).map(|(_, _, label)| label),
    }
}
fn fields(kind: &str, path: &str, source: &str) -> &'static [&'static str] {
    match (kind, path, source) {
        (AGENT, "agent_resource_role_arn", "aws_iam_role")
        | (FLOW, "execution_role_arn", "aws_iam_role")
        | (KB, "role_arn", "aws_iam_role") => &["arn"],
        (AGENT | FLOW | PROMPT, "customer_encryption_key_arn", "aws_kms_key")
        | (SOURCE, "server_side_encryption_configuration.kms_key_arn", "aws_kms_key") => &["arn"],
        (GROUP | ALIAS | COLLABORATOR | ASSOCIATION, "agent_id", AGENT) => &["agent_id"],
        (SOURCE | ASSOCIATION, "knowledge_base_id", KB) => &["id"],
        (COLLABORATOR, "agent_descriptor.alias_arn", ALIAS)
        | (FLOW, "definition.node.configuration.agent.agent_alias_arn", ALIAS) => {
            &["agent_alias_arn"]
        }
        (GROUP, "action_group_executor.lambda", "aws_lambda_function")
        | (
            FLOW,
            "definition.node.configuration.lambda_function.lambda_arn",
            "aws_lambda_function",
        ) => &["arn"],
        (GROUP, "api_schema.s3.s3_bucket_name", "aws_s3_bucket")
        | (
            FLOW,
            "definition.node.configuration.retrieval.service_configuration.s3.bucket_name"
            | "definition.node.configuration.storage.service_configuration.s3.bucket_name",
            "aws_s3_bucket",
        ) => &["id", "bucket"],
        (SOURCE, "data_source_configuration.s3_configuration.bucket_arn", "aws_s3_bucket") => {
            &["arn"]
        }
        (FLOW, "definition.node.configuration.knowledge_base.knowledge_base_id", KB) => &["id"],
        (
            FLOW,
            "definition.node.configuration.prompt.source_configuration.resource.prompt_arn",
            PROMPT,
        ) => &["arn"],
        (
            KB,
            "storage_configuration.opensearch_serverless_configuration.collection_arn",
            "aws_opensearchserverless_collection",
        )
        | (
            KB,
            "storage_configuration.opensearch_managed_cluster_configuration.domain_arn",
            "aws_opensearch_domain",
        )
        | (KB, "storage_configuration.rds_configuration.resource_arn", "aws_rds_cluster")
        | (
            KB,
            "storage_configuration.neptune_analytics_configuration.graph_arn",
            "aws_neptunegraph_graph",
        )
        | (
            KB,
            "knowledge_base_configuration.kendra_knowledge_base_configuration.kendra_index_arn",
            "aws_kendra_index",
        ) => &["arn"],
        (KB, "storage_configuration.s3_vectors_configuration.index_arn", "aws_s3vectors_index") => {
            &["index_arn"]
        }
        (PROMPT, "variant.model_id", "aws_bedrock_inference_profile") => &["arn", "id"],
        (PROMPT, "variant.gen_ai_resource.agent.agent_identifier", AGENT) => &["agent_arn"],
        _ => extra(kind, path)
            .filter(|(target, _, _)| *target == source)
            .map_or(&[], |(_, fields, _)| fields),
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
fn compatible(left: &TerraformEntity, right: &TerraformEntity) -> bool {
    let l = super::scope::scope(left);
    let r = super::scope::scope(right);
    super::eventbridge::bindings(
        left.provider_configuration.as_ref(),
        right.provider_configuration.as_ref(),
        l.as_ref().and_then(|s| s.region.as_ref()),
        r.as_ref().and_then(|s| s.region.as_ref()),
        right.resource_type == "aws_iam_role",
    )
}
fn verify(
    nodes: &[TerraformEntity],
    refs: &[AttributeReference],
    e: &[(&Value, &Value)],
    r: &AttributeReference,
) -> bool {
    if matches!(
        r.attribute.as_str(),
        "guardrail_configuration" | "prompt_override_configuration" | "routing_configuration"
    ) {
        return objects::verify(nodes, refs, e, r);
    }
    super::nested_references::verify_collections(
        nodes,
        refs,
        e,
        r,
        fields,
        |s, path| {
            super::nested_references::value(e[r.target].0, e[r.target].1, path)
                .is_some_and(Value::is_string)
                && compatible(&nodes[r.target], &nodes[s])
                && (!r.attribute.contains(".context_enrichment_configuration.")
                    || super::bedrock_models::system_profile(e, s))
                && qualify::path(e, r.target, &nodes[r.target].resource_type, path)
        },
        &["definition.node", "variant"],
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
            let valid = verify(nodes, refs, e, r);
            let required = match nodes[r.target].resource_type.as_str() {
                ASSOCIATION => Some(if r.attribute == "agent_id" {
                    "knowledge_base_id"
                } else {
                    "agent_id"
                }),
                COLLABORATOR => Some(if r.attribute == "agent_id" {
                    "agent_descriptor.alias_arn"
                } else {
                    "agent_id"
                }),
                _ => None,
            };
            Some(
                valid
                    && required.is_none_or(|path| {
                        super::lambda_core::unique(refs, r.target, path)
                            .is_some_and(|other| verify(nodes, refs, e, other))
                    }),
            )
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
}
pub(super) fn bindings(raw: &crate::semantic::Input, r: &AttributeReference) -> bool {
    label(&raw.nodes[r.target].resource_type, &r.attribute).is_none()
        || r.sources.iter().all(|&s| {
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
#[path = "../../../tests/unit/semantic/bedrock_agents.rs"]
mod tests;

fn extra(kind: &str, path: &str) -> Option<(&'static str, &'static [&'static str], &'static str)> {
    match (kind, path) {
        (
            SOURCE,
            "data_source_configuration.confluence_configuration.source_configuration.credentials_secret_arn",
        ) => Some((
            "aws_secretsmanager_secret",
            &["arn"],
            "Knowledge connector secret",
        )),
        (
            SOURCE,
            "data_source_configuration.salesforce_configuration.source_configuration.credentials_secret_arn",
        ) => Some((
            "aws_secretsmanager_secret",
            &["arn"],
            "Knowledge connector secret",
        )),
        (
            SOURCE,
            "data_source_configuration.share_point_configuration.source_configuration.credentials_secret_arn",
        ) => Some((
            "aws_secretsmanager_secret",
            &["arn"],
            "Knowledge connector secret",
        )),
        (
            SOURCE,
            "vector_ingestion_configuration.context_enrichment_configuration.bedrock_foundation_model_configuration.model_arn",
        ) => Some((
            "aws_bedrock_inference_profile",
            &["arn"],
            "Enrichment inference profile",
        )),
        (
            SOURCE,
            "vector_ingestion_configuration.custom_transformation_configuration.transformation.transformation_function.transformation_lambda_configuration.lambda_arn",
        ) => Some((
            "aws_lambda_function",
            &["arn"],
            "Ingestion transformation Lambda",
        )),
        (KB, "storage_configuration.mongo_db_atlas_configuration.credentials_secret_arn") => {
            Some((
                "aws_secretsmanager_secret",
                &["arn"],
                "Knowledge storage secret",
            ))
        }
        (KB, "storage_configuration.pinecone_configuration.credentials_secret_arn") => Some((
            "aws_secretsmanager_secret",
            &["arn"],
            "Knowledge storage secret",
        )),
        (KB, "storage_configuration.rds_configuration.credentials_secret_arn") => Some((
            "aws_secretsmanager_secret",
            &["arn"],
            "Knowledge storage secret",
        )),
        (
            KB,
            "storage_configuration.redis_enterprise_cloud_configuration.credentials_secret_arn",
        ) => Some((
            "aws_secretsmanager_secret",
            &["arn"],
            "Knowledge storage secret",
        )),
        (
            KB,
            "knowledge_base_configuration.sql_knowledge_base_configuration.redshift_configuration.query_engine_configuration.provisioned_configuration.cluster_identifier",
        ) => Some((
            "aws_redshift_cluster",
            &["cluster_identifier"],
            "Knowledge Redshift engine",
        )),
        (
            KB,
            "knowledge_base_configuration.sql_knowledge_base_configuration.redshift_configuration.query_engine_configuration.serverless_configuration.workgroup_arn",
        ) => Some((
            "aws_redshiftserverless_workgroup",
            &["arn"],
            "Knowledge Redshift engine",
        )),
        (
            KB,
            "knowledge_base_configuration.sql_knowledge_base_configuration.redshift_configuration.query_engine_configuration.provisioned_configuration.auth_configuration.username_password_secret_arn",
        ) => Some((
            "aws_secretsmanager_secret",
            &["arn"],
            "Knowledge query secret",
        )),
        (
            KB,
            "knowledge_base_configuration.sql_knowledge_base_configuration.redshift_configuration.query_engine_configuration.serverless_configuration.auth_configuration.username_password_secret_arn",
        ) => Some((
            "aws_secretsmanager_secret",
            &["arn"],
            "Knowledge query secret",
        )),
        _ => None,
    }
}
