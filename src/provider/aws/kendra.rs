use super::configured_links::{self, Evidence, Rule};
use crate::model::{AttributeReference, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;

fn rule(kind: &str, path: &str) -> Option<Rule> {
    Some(match (kind, path) {
        ("aws_kendra_index", "server_side_encryption_configuration.kms_key_id") => Rule::new(
            "Index encryption key",
            &[
                ("aws_kms_key", &["arn", "key_id", "id"]),
                ("aws_kms_replica_key", &["arn", "key_id", "id"]),
                ("aws_kms_external_key", &["arn", "id"]),
            ],
            false,
            &[],
        ),
        (
            "aws_kendra_index",
            "user_token_configurations.jwt_token_type_configuration.secrets_manager_arn",
        ) => Rule::new(
            "JWT signing key secret",
            &[("aws_secretsmanager_secret", &["arn"])],
            false,
            &[],
        ),
        (
            "aws_kendra_data_source"
            | "aws_kendra_experience"
            | "aws_kendra_faq"
            | "aws_kendra_query_suggestions_block_list"
            | "aws_kendra_thesaurus",
            "index_id",
        ) => Rule::new("Kendra index", &[("aws_kendra_index", &["id"])], false, &[]),
        (
            "aws_kendra_index"
            | "aws_kendra_data_source"
            | "aws_kendra_experience"
            | "aws_kendra_faq"
            | "aws_kendra_query_suggestions_block_list"
            | "aws_kendra_thesaurus",
            "role_arn",
        ) => Rule::new(
            "Kendra execution role",
            &[("aws_iam_role", &["arn"])],
            false,
            &[],
        ),
        ("aws_kendra_data_source", "configuration.s3_configuration.bucket_name") => Rule::new(
            "Data source bucket",
            &[("aws_s3_bucket", &["bucket", "id"])],
            false,
            &[],
        ),
        ("aws_kendra_data_source", "custom_document_enrichment_configuration.role_arn") => {
            Rule::new(
                "Document enrichment role",
                &[("aws_iam_role", &["arn"])],
                false,
                &[],
            )
        }
        (
            "aws_kendra_data_source",
            "custom_document_enrichment_configuration.pre_extraction_hook_configuration.lambda_arn",
        ) => Rule::new(
            "Pre-extraction function",
            &[("aws_lambda_function", &["arn"])],
            false,
            &[],
        ),
        (
            "aws_kendra_data_source",
            "custom_document_enrichment_configuration.post_extraction_hook_configuration.lambda_arn",
        ) => Rule::new(
            "Post-extraction function",
            &[("aws_lambda_function", &["arn"])],
            false,
            &[],
        ),
        (
            "aws_kendra_data_source",
            "custom_document_enrichment_configuration.pre_extraction_hook_configuration.s3_bucket",
        ) => Rule::new(
            "Pre-extraction bucket",
            &[("aws_s3_bucket", &["bucket", "id"])],
            false,
            &[],
        ),
        (
            "aws_kendra_data_source",
            "custom_document_enrichment_configuration.post_extraction_hook_configuration.s3_bucket",
        ) => Rule::new(
            "Post-extraction bucket",
            &[("aws_s3_bucket", &["bucket", "id"])],
            false,
            &[],
        ),
        ("aws_kendra_experience", "configuration.content_source_configuration.data_source_ids") => {
            Rule::new(
                "Experience data sources",
                &[("aws_kendra_data_source", &["data_source_id"])],
                true,
                &[],
            )
        }
        ("aws_kendra_experience", "configuration.content_source_configuration.faq_ids") => {
            Rule::new(
                "Experience FAQs",
                &[("aws_kendra_faq", &["faq_id"])],
                true,
                &[],
            )
        }
        ("aws_kendra_faq", "s3_path.bucket") => Rule::new(
            "Source bucket",
            &[("aws_s3_bucket", &["bucket"])],
            false,
            &[],
        ),
        (
            "aws_kendra_query_suggestions_block_list" | "aws_kendra_thesaurus",
            "source_s3_path.bucket",
        ) => Rule::new(
            "Source bucket",
            &[("aws_s3_bucket", &["bucket"])],
            false,
            &[],
        ),
        _ => return None,
    })
}
pub(crate) fn resource(kind: &str) -> bool {
    matches!(
        kind,
        "aws_kendra_index"
            | "aws_kendra_data_source"
            | "aws_kendra_experience"
            | "aws_kendra_faq"
            | "aws_kendra_query_suggestions_block_list"
            | "aws_kendra_thesaurus"
    )
}
pub(crate) fn indexed(kind: &str, path: &str) -> bool {
    resource(kind) && configured_links::indexed(kind, path, rule)
}
pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    rule(kind, path).map(|r| r.label)
}
pub(super) fn bindings(raw: &Input, r: &AttributeReference) -> bool {
    configured_links::bindings(raw, r, rule)
}
pub(super) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    values: &[(&Value, &Value)],
) {
    configured_links::annotate(nodes, refs, values, rule, qualifies)
}
fn qualifies(e: &Evidence<'_>, r: &AttributeReference, source: usize, _path: &str) -> bool {
    match (
        e.nodes[r.target].resource_type.as_str(),
        r.attribute.as_str(),
    ) {
        ("aws_kendra_index", "server_side_encryption_configuration.kms_key_id") => {
            let (key, mask) = e.values[source];
            super::kms::symmetric_key(&e.nodes[source].resource_type, key, mask)
        }
        (
            "aws_kendra_index",
            "user_token_configurations.jwt_token_type_configuration.secrets_manager_arn",
        ) => {
            e.text(
                r.target,
                "user_token_configurations.0.jwt_token_type_configuration.0.key_location",
            ) == Some("SECRET_MANAGER")
        }
        ("aws_kendra_data_source", "role_arn") => e
            .text(r.target, "type")
            .is_some_and(|kind| kind != "CUSTOM"),
        ("aws_kendra_data_source", "configuration.s3_configuration.bucket_name") => {
            e.text(r.target, "type") == Some("S3")
                && e.value(r.target, "configuration.0.template_configuration")
                    .is_some_and(|value| {
                        value.is_null() || value.as_array().is_some_and(Vec::is_empty)
                    })
        }
        (
            "aws_kendra_experience",
            "configuration.content_source_configuration.data_source_ids"
            | "configuration.content_source_configuration.faq_ids",
        ) => e.same_endpoint(
            r.target,
            "index_id",
            source,
            "index_id",
            "aws_kendra_index",
            "id",
        ),
        _ => true,
    }
}
#[cfg(test)]
#[path = "../../../tests/unit/semantic/kendra.rs"]
mod tests;
