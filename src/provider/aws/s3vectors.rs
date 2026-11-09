use crate::model::{AttributeReference, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/s3vectors.rs"]
mod tests;

pub(crate) fn resource(kind: &str) -> bool {
    matches!(
        kind,
        "aws_s3vectors_index"
            | "aws_s3vectors_vector_bucket"
            | "aws_s3vectors_vector_bucket_policy"
    )
}
pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    match (kind, path) {
        ("aws_s3vectors_index", "vector_bucket_name")
        | ("aws_s3vectors_vector_bucket_policy", "vector_bucket_arn") => {
            Some("Vector bucket owner")
        }
        (
            "aws_s3vectors_index" | "aws_s3vectors_vector_bucket",
            "encryption_configuration.kms_key_arn",
        ) => Some("Vector encryption key"),
        _ => None,
    }
}
fn fields(kind: &str, path: &str, source: &str) -> &'static [&'static str] {
    match (kind, path, source) {
        ("aws_s3vectors_index", "vector_bucket_name", "aws_s3vectors_vector_bucket") => {
            &["vector_bucket_name"]
        }
        (
            "aws_s3vectors_vector_bucket_policy",
            "vector_bucket_arn",
            "aws_s3vectors_vector_bucket",
        ) => &["vector_bucket_arn"],
        (_, "encryption_configuration.kms_key_arn", _)
            if !super::kms::key_outputs(source).is_empty() =>
        {
            &["arn"]
        }
        _ => &[],
    }
}
pub(crate) fn indexed(kind: &str, path: &str) -> bool {
    let normal = path
        .split('.')
        .filter(|s| s.parse::<usize>().is_err())
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
            super::nested_references::verify(nodes, refs, evidence, r, fields, |_, path| {
                let (v, m) = evidence[r.target];
                super::nested_references::value(v, m, path).is_some_and(Value::is_string)
                    && (r.attribute != "encryption_configuration.kms_key_arn"
                        || (v["encryption_configuration"]
                            .as_array()
                            .is_some_and(|a| a.len() == 1)
                            && super::nested_references::value(
                                v,
                                m,
                                "encryption_configuration.0.sse_type",
                            )
                            .and_then(Value::as_str)
                                == Some("aws:kms")))
            })
            .is_some()
        })
        .collect();
    for (r, valid) in refs.iter_mut().zip(results) {
        r.service_reference_verified = valid;
        if valid {
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
        ) && ar.is_some()
            && ar == br
    })
}
