use super::{WORKGROUP, nested_references};
use serde_json::Value;

const DOMAIN: &str = "aws_redshiftserverless_custom_domain_association";
const NAMESPACE: &str = "aws_redshiftserverless_namespace";
const LIMIT: &str = "aws_redshiftserverless_usage_limit";
pub(super) fn resource(kind: &str) -> bool {
    matches!(kind, DOMAIN | NAMESPACE | LIMIT)
}
pub(super) fn owner(kind: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match kind {
        DOMAIN => Some((
            "workgroup_name",
            "Custom domain workgroup",
            "Custom domains",
        )),
        LIMIT => Some(("resource_arn", "Usage limit workgroup", "Usage limits")),
        _ => None,
    }
}
pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    match (kind, path) {
        (DOMAIN, "workgroup_name") | (LIMIT, "resource_arn") => {
            owner(kind).map(|(_, label, _)| label)
        }
        (DOMAIN, "custom_domain_certificate_arn") => Some("Custom domain certificate"),
        (NAMESPACE, "iam_roles") => Some("Namespace IAM roles"),
        (NAMESPACE, "default_iam_role_arn") => Some("Namespace default role"),
        (NAMESPACE, "kms_key_id") => Some("Namespace data encryption key"),
        (NAMESPACE, "admin_password_secret_kms_key_id") => {
            Some("Namespace credentials encryption key")
        }
        _ => None,
    }
}
pub(super) fn fields(kind: &str, path: &str, source: &str) -> &'static [&'static str] {
    match (kind, path, source) {
        (DOMAIN, "workgroup_name", WORKGROUP) => &["workgroup_name"],
        (DOMAIN, "custom_domain_certificate_arn", "aws_acm_certificate")
        | (LIMIT, "resource_arn", WORKGROUP)
        | (NAMESPACE, "iam_roles" | "default_iam_role_arn", "aws_iam_role")
        | (NAMESPACE, "kms_key_id", "aws_kms_key") => &["arn"],
        (NAMESPACE, "admin_password_secret_kms_key_id", "aws_kms_key") => &["arn", "key_id", "id"],
        _ => &[],
    }
}
pub(super) fn valid_mode(kind: &str, path: &str, v: &Value, m: &Value) -> bool {
    if (kind, path) == (LIMIT, "resource_arn") {
        return matches!(
            nested_references::value(v, m, "usage_type").and_then(Value::as_str),
            Some("serverless-compute" | "cross-region-datasharing")
        );
    }
    true
}
