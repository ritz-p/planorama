use super::configured_links::{self, Evidence, Rule};
use crate::model::{AttributeReference, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;

pub(crate) fn resource(kind: &str) -> bool {
    matches!(
        kind,
        "aws_glue_data_catalog_encryption_settings" | "aws_glue_security_configuration"
    )
}
fn rule(kind: &str, path: &str) -> Option<Rule> {
    Some(match (kind, path) {
        (
            "aws_glue_data_catalog_encryption_settings",
            "data_catalog_encryption_settings.connection_password_encryption.aws_kms_key_id",
        ) => Rule::new(
            "Connection password key",
            &[("aws_kms_key", &["arn"])],
            false,
            &[],
        ),
        (
            "aws_glue_data_catalog_encryption_settings",
            "data_catalog_encryption_settings.encryption_at_rest.sse_aws_kms_key_id",
        ) => Rule::new(
            "Catalog encryption key",
            &[("aws_kms_key", &["arn"])],
            false,
            &[],
        ),
        (
            "aws_glue_data_catalog_encryption_settings",
            "data_catalog_encryption_settings.encryption_at_rest.catalog_encryption_service_role",
        ) => Rule::new(
            "Catalog encryption role",
            &[("aws_iam_role", &["arn"])],
            false,
            &[],
        ),
        (
            "aws_glue_security_configuration",
            "encryption_configuration.cloudwatch_encryption.kms_key_arn"
            | "encryption_configuration.job_bookmarks_encryption.kms_key_arn"
            | "encryption_configuration.s3_encryption.kms_key_arn",
        ) => Rule::new(
            "Security configuration key",
            &[("aws_kms_key", &["arn"])],
            false,
            &[],
        ),
        _ => return None,
    })
}
pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    rule(kind, path).map(|r| r.label)
}
pub(crate) fn indexed(kind: &str, path: &str) -> bool {
    resource(kind) && configured_links::indexed(kind, path, rule)
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
fn qualifies(e: &Evidence<'_>, r: &AttributeReference, _s: usize, path: &str) -> bool {
    match (e.nodes[r.target].resource_type.as_str(),r.attribute.as_str()){
(_,p) if p.ends_with(".aws_kms_key_id")=>e.value(r.target,"data_catalog_encryption_settings.0.connection_password_encryption.0.return_connection_password_encrypted").and_then(Value::as_bool)==Some(true),
(_,p) if p.ends_with(".sse_aws_kms_key_id")=>matches!(e.text(r.target,"data_catalog_encryption_settings.0.encryption_at_rest.0.catalog_encryption_mode"),Some("SSE-KMS"|"SSE-KMS-WITH-SERVICE-ROLE")),
(_,p) if p.ends_with(".catalog_encryption_service_role")=>e.text(r.target,"data_catalog_encryption_settings.0.encryption_at_rest.0.catalog_encryption_mode")==Some("SSE-KMS-WITH-SERVICE-ROLE"),
(_,p)=>{let (field,mode)=if p.contains(".cloudwatch_encryption."){("cloudwatch_encryption_mode","SSE-KMS")}else if p.contains(".job_bookmarks_encryption."){("job_bookmarks_encryption_mode","CSE-KMS")}else{("s3_encryption_mode","SSE-KMS")};path.rsplit_once('.').is_some_and(|(b,_)|e.text(r.target,&format!("{b}.{field}"))==Some(mode))},
}
}

#[cfg(test)]
#[path = "../../../tests/unit/semantic/glue_encryption.rs"]
mod tests;
