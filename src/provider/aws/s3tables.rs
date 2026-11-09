use crate::model::{AttributeReference, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/s3tables.rs"]
mod tests;

const DESTINATION: &str = "rule.destination.destination_table_bucket_arn";
pub(crate) fn resource(kind: &str) -> bool {
    matches!(
        kind,
        "aws_s3tables_namespace"
            | "aws_s3tables_table"
            | "aws_s3tables_table_bucket"
            | "aws_s3tables_table_bucket_policy"
            | "aws_s3tables_table_policy"
            | "aws_s3tables_table_bucket_replication"
            | "aws_s3tables_table_replication"
    )
}
pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    match (kind, path) {
        (
            "aws_s3tables_namespace"
            | "aws_s3tables_table"
            | "aws_s3tables_table_bucket_policy"
            | "aws_s3tables_table_bucket_replication",
            "table_bucket_arn",
        ) => Some("Table bucket"),
        ("aws_s3tables_table", "namespace") => Some("Qualified namespace"),
        ("aws_s3tables_table_policy", "name") => Some("Qualified table owner"),
        ("aws_s3tables_table_replication", "table_arn") => Some("Replication source table"),
        ("aws_s3tables_table_bucket_replication" | "aws_s3tables_table_replication", "role") => {
            Some("Configured replication role")
        }
        (
            "aws_s3tables_table_bucket_replication" | "aws_s3tables_table_replication",
            DESTINATION,
        ) => Some("Configured replication destinations"),
        (
            "aws_s3tables_table" | "aws_s3tables_table_bucket",
            "encryption_configuration.kms_key_arn",
        ) => Some("Table encryption key"),
        _ => None,
    }
}
fn fields(kind: &str, path: &str, source: &str) -> &'static [&'static str] {
    match (kind, path, source) {
        (_, "table_bucket_arn" | DESTINATION, "aws_s3tables_table_bucket") => &["arn"],
        ("aws_s3tables_table", "table_bucket_arn", "aws_s3tables_namespace") => {
            &["table_bucket_arn"]
        }
        ("aws_s3tables_table", "namespace", "aws_s3tables_namespace") => &["namespace"],
        ("aws_s3tables_table_policy", "name", "aws_s3tables_table") => &["name"],
        ("aws_s3tables_table_replication", "table_arn", "aws_s3tables_table")
        | (_, "role", "aws_iam_role") => &["arn"],
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
            super::nested_references::verify(nodes, refs, evidence, r, fields, |source, path| {
                let (v, m) = evidence[r.target];
                if !super::nested_references::value(v, m, path).is_some_and(Value::is_string) {
                    return false;
                }
                let equal = |p| {
                    super::service_evidence::known(evidence, r.target, p).is_some_and(|v| {
                        super::service_evidence::known(evidence, source, p) == Some(v)
                    })
                };
                match r.attribute.as_str() {
                    "namespace" => equal("table_bucket_arn"),
                    "table_bucket_arn"
                        if nodes[source].resource_type == "aws_s3tables_namespace" =>
                    {
                        equal("namespace")
                    }
                    "name" => equal("namespace") && equal("table_bucket_arn"),
                    "encryption_configuration.kms_key_arn" => {
                        v["encryption_configuration"].is_object()
                            && super::nested_references::value(
                                v,
                                m,
                                "encryption_configuration.sse_algorithm",
                            )
                            .and_then(Value::as_str)
                                == Some("aws:kms")
                    }
                    DESTINATION => {
                        v["rule"].as_array().is_some_and(|a| a.len() == 1)
                            && v["rule"][0]["destination"]
                                .as_array()
                                .is_some_and(|a| (1..=5).contains(&a.len()))
                    }
                    _ => true,
                }
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
            && if r.attribute == DESTINATION {
                br.is_some()
            } else {
                b.resource_type == "aws_iam_role" || ar == br
            }
    })
}
