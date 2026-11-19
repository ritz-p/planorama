use crate::provider::aws::nested_references as nested;
use serde_json::Value;
fn known<'a>(e: &[(&'a Value, &Value)], t: usize, p: &str) -> Option<&'a str> {
    nested::value(e[t].0, e[t].1, p)?.as_str()
}
fn empty(e: &[(&Value, &Value)], t: usize, p: &str) -> bool {
    nested::value(e[t].0, e[t].1, p).is_some_and(|v| {
        v.is_null() || v.as_array().is_some_and(Vec::is_empty) || v.as_str() == Some("")
    })
}
fn choice(e: &[(&Value, &Value)], t: usize, base: &str, selected: &str, branches: &[&str]) -> bool {
    nested::singleton(e[t].0, e[t].1, &format!("{base}.{selected}"))
        && branches
            .iter()
            .filter(|&&b| b != selected)
            .all(|b| empty(e, t, &format!("{base}.{b}")))
}
pub(super) fn path(e: &[(&Value, &Value)], t: usize, kind: &str, p: &str) -> bool {
    match kind {
        super::GROUP if p == "agent_id" => {
            known(e, t, "agent_version") == Some("DRAFT")
                && known(e, t, "action_group_id").is_some_and(|s| !s.is_empty())
        }
        super::ASSOCIATION => {
            known(e, t, "agent_version") == Some("DRAFT")
                && matches!(
                    known(e, t, "knowledge_base_state"),
                    Some("ENABLED" | "DISABLED")
                )
        }
        super::COLLABORATOR => {
            known(e, t, "agent_version") == Some("DRAFT")
                && known(e, t, "collaborator_id").is_some_and(|s| !s.is_empty())
        }
        super::GROUP if p == "action_group_executor.0.lambda" => {
            empty(e, t, "action_group_executor.0.custom_control")
        }
        super::GROUP if p.starts_with("api_schema.") => empty(e, t, "api_schema.0.payload"),
        super::SOURCE if p.starts_with("data_source_configuration.") => {
            let branch = p.split('.').nth(2).unwrap_or("");
            let ty = match branch {
                "s3_configuration" => "S3",
                "confluence_configuration" => "CONFLUENCE",
                "salesforce_configuration" => "SALESFORCE",
                "share_point_configuration" => "SHAREPOINT",
                _ => return false,
            };
            known(e, t, "data_source_configuration.0.type") == Some(ty)
                && choice(
                    e,
                    t,
                    "data_source_configuration.0",
                    branch,
                    &[
                        "s3_configuration",
                        "confluence_configuration",
                        "salesforce_configuration",
                        "share_point_configuration",
                        "web_configuration",
                        "managed_knowledge_base_connector_configuration",
                    ],
                )
        }
        super::SOURCE if p.contains(".context_enrichment_configuration.") => {
            known(
                e,
                t,
                "vector_ingestion_configuration.0.context_enrichment_configuration.0.type",
            ) == Some("BEDROCK_FOUNDATION_MODEL")
        }
        super::SOURCE if p.contains(".transformation.") => {
            known(
                e,
                t,
                "vector_ingestion_configuration.0.custom_transformation_configuration.0.transformation.0.step_to_apply",
            ) == Some("POST_CHUNKING")
        }
        super::KB if p.starts_with("storage_configuration.") => {
            let branch = p.split('.').nth(2).unwrap_or("");
            let discriminator = match branch {
                "opensearch_serverless_configuration" => "OPENSEARCH_SERVERLESS",
                "opensearch_managed_cluster_configuration" => "OPENSEARCH_MANAGED_CLUSTER",
                "rds_configuration" => "RDS",
                "neptune_analytics_configuration" => "NEPTUNE_ANALYTICS",
                "s3_vectors_configuration" => "S3_VECTORS",
                "mongo_db_atlas_configuration" => "MONGO_DB_ATLAS",
                "pinecone_configuration" => "PINECONE",
                "redis_enterprise_cloud_configuration" => "REDIS_ENTERPRISE_CLOUD",
                _ => return false,
            };
            known(e, t, "knowledge_base_configuration.0.type") == Some("VECTOR")
                && known(e, t, "storage_configuration.0.type") == Some(discriminator)
                && choice(
                    e,
                    t,
                    "storage_configuration.0",
                    branch,
                    &[
                        "opensearch_serverless_configuration",
                        "opensearch_managed_cluster_configuration",
                        "rds_configuration",
                        "neptune_analytics_configuration",
                        "s3_vectors_configuration",
                        "mongo_db_atlas_configuration",
                        "pinecone_configuration",
                        "redis_enterprise_cloud_configuration",
                    ],
                )
        }
        super::KB if p.contains(".sql_knowledge_base_configuration.") => {
            let base = "knowledge_base_configuration.0.sql_knowledge_base_configuration.0";
            let query = format!("{base}.redshift_configuration.0.query_engine_configuration.0");
            let provisioned = p.contains(".provisioned_configuration.");
            let branch = if provisioned {
                "provisioned_configuration"
            } else {
                "serverless_configuration"
            };
            known(e, t, "knowledge_base_configuration.0.type") == Some("SQL")
                && choice(
                    e,
                    t,
                    "knowledge_base_configuration.0",
                    "sql_knowledge_base_configuration",
                    &[
                        "sql_knowledge_base_configuration",
                        "kendra_knowledge_base_configuration",
                        "vector_knowledge_base_configuration",
                        "managed_knowledge_base_configuration",
                    ],
                )
                && known(e, t, &format!("{base}.type")) == Some("REDSHIFT")
                && known(e, t, &format!("{query}.type"))
                    == Some(if provisioned {
                        "PROVISIONED"
                    } else {
                        "SERVERLESS"
                    })
                && choice(
                    e,
                    t,
                    &query,
                    branch,
                    &["provisioned_configuration", "serverless_configuration"],
                )
                && (!p.ends_with("username_password_secret_arn")
                    || known(
                        e,
                        t,
                        &format!("{query}.{branch}.0.auth_configuration.0.type"),
                    ) == Some("USERNAME_PASSWORD"))
        }
        super::KB if p.starts_with("knowledge_base_configuration.") => {
            known(e, t, "knowledge_base_configuration.0.type") == Some("KENDRA")
                && choice(
                    e,
                    t,
                    "knowledge_base_configuration.0",
                    "kendra_knowledge_base_configuration",
                    &[
                        "kendra_knowledge_base_configuration",
                        "sql_knowledge_base_configuration",
                        "vector_knowledge_base_configuration",
                        "managed_knowledge_base_configuration",
                    ],
                )
        }
        super::PROMPT if p.starts_with("variant.") => {
            let parts: Vec<_> = p.split('.').collect();
            let base = format!("variant.{}", parts[1]);
            if p.ends_with("model_id") {
                empty(e, t, &format!("{base}.gen_ai_resource"))
            } else {
                empty(e, t, &format!("{base}.model_id"))
                    && choice(
                        e,
                        t,
                        &format!("{base}.gen_ai_resource.0"),
                        "agent",
                        &["agent"],
                    )
            }
        }
        super::FLOW if p.starts_with("definition.") => {
            let parts: Vec<_> = p.split('.').collect();
            let base = format!("definition.0.node.{}", parts[3]);
            let branch = parts.get(6).copied().unwrap_or("");
            let ty = match branch {
                "agent" => "Agent",
                "knowledge_base" => "KnowledgeBase",
                "lambda_function" => "LambdaFunction",
                "lex" => "Lex",
                "prompt" => "Prompt",
                "retrieval" => "Retrieval",
                "storage" => "Storage",
                _ => return false,
            };
            known(e, t, &format!("{base}.type")) == Some(ty)
                && choice(
                    e,
                    t,
                    &format!("{base}.configuration.0"),
                    branch,
                    &[
                        "agent",
                        "knowledge_base",
                        "lambda_function",
                        "lex",
                        "prompt",
                        "retrieval",
                        "storage",
                        "collector",
                        "condition",
                        "inline_code",
                        "input",
                        "iterator",
                        "output",
                    ],
                )
                && (!p.contains(".prompt.")
                    || empty(
                        e,
                        t,
                        &format!("{base}.configuration.0.prompt.0.source_configuration.0.inline"),
                    ))
        }
        _ => true,
    }
}
