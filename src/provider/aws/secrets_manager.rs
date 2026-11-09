use crate::model::{AttributeReference, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/secrets_manager_references.rs"]
mod tests;

pub(crate) fn resource(kind: &str) -> bool {
    matches!(
        kind,
        "aws_secretsmanager_secret"
            | "aws_secretsmanager_secret_policy"
            | "aws_secretsmanager_secret_rotation"
            | "aws_secretsmanager_tag"
    )
}
pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    match (kind, path) {
        ("aws_secretsmanager_secret", "kms_key_id") => Some("Secret encryption key"),
        ("aws_secretsmanager_secret", "replica.kms_key_id") => Some("Replica encryption keys"),
        ("aws_secretsmanager_secret_policy", "secret_arn")
        | ("aws_secretsmanager_secret_rotation" | "aws_secretsmanager_tag", "secret_id") => {
            Some("Secret owner")
        }
        ("aws_secretsmanager_secret_rotation", "rotation_lambda_arn") => {
            Some("Configured rotation function")
        }
        ("aws_secretsmanager_secret_rotation", "external_secret_rotation_role_arn") => {
            Some("Configured partner role")
        }
        _ => None,
    }
}
pub(super) fn fields(kind: &str, path: &str, source: &str) -> &'static [&'static str] {
    match (kind, path, source) {
        ("aws_secretsmanager_secret", "kms_key_id" | "replica.kms_key_id", _) => {
            super::kms::key_outputs(source)
        }
        ("aws_secretsmanager_secret_policy", "secret_arn", "aws_secretsmanager_secret") => &["arn"],
        ("aws_secretsmanager_tag", "secret_id", "aws_secretsmanager_secret") => &["id"],
        ("aws_secretsmanager_secret_rotation", "secret_id", "aws_secretsmanager_secret") => {
            &["id", "arn", "name"]
        }
        ("aws_secretsmanager_secret_rotation", "rotation_lambda_arn", "aws_lambda_function")
        | (
            "aws_secretsmanager_secret_rotation",
            "external_secret_rotation_role_arn",
            "aws_iam_role",
        ) => &["arn"],
        _ => &[],
    }
}
pub(crate) fn indexed(kind: &str, path: &str) -> bool {
    let normal = path
        .split('.')
        .filter(|p| p.parse::<usize>().is_err())
        .collect::<Vec<_>>()
        .join(".");
    normal != path && label(kind, &normal).is_some()
}
pub(crate) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    evidence: &[(&Value, &Value)],
) {
    let results: Vec<_> = refs
        .iter()
        .map(|r| {
            if label(&nodes[r.target].resource_type, &r.attribute).is_none() {
                return r.service_reference_verified;
            }
            super::nested_references::verify(nodes, refs, evidence, r, fields, |source, path| {
                let (value, mask) = evidence[r.target];
                if !super::nested_references::value(value, mask, path).is_some_and(Value::is_string)
                {
                    return false;
                }
                match r.attribute.as_str() {
                    "replica.kms_key_id" => {
                        let region_path = path.replace("kms_key_id", "region");
                        let region = super::nested_references::value(value, mask, &region_path)
                            .and_then(Value::as_str);
                        let primary = super::scope::scope(&nodes[r.target]).and_then(|s| s.region);
                        let replica = super::scope::scope(&nodes[source]).and_then(|s| s.region);
                        region.is_some()
                            && primary
                                .zip(replica)
                                .is_some_and(|(a, b)| a != b && region == Some(b.0.as_str()))
                    }
                    "rotation_lambda_arn" | "external_secret_rotation_role_arn" => {
                        let other = if r.attribute == "rotation_lambda_arn" {
                            "external_secret_rotation_role_arn"
                        } else {
                            "rotation_lambda_arn"
                        };
                        super::nested_references::value(value, mask, "rotation_enabled")
                            == Some(&Value::Bool(true))
                            && super::nested_references::value(value, mask, other)
                                .is_some_and(|v| v.is_null() || v == "")
                    }
                    _ => true,
                }
            })
            .is_some()
        })
        .collect();
    for (r, verified) in refs.iter_mut().zip(results) {
        r.service_reference_verified = verified;
        if verified {
            r.complete = true;
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
        ) && if r.attribute == "replica.kms_key_id" {
            ar.zip(br).is_some_and(|(a, b)| a != b)
        } else if b.resource_type == "aws_iam_role" {
            ar.is_some()
        } else {
            ar.is_some() && ar == br
        }
    })
}
