use crate::model::{Action, AttributeReference, EntityMode, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;
use std::collections::BTreeSet;

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
        ("aws_s3tables_table" | "aws_s3tables_table_bucket", "encryption_configuration") => {
            Some("Table encryption key")
        }
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
        (_, "encryption_configuration", _) if !super::kms::key_outputs(source).is_empty() => {
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
fn verify_set(
    nodes: &[TerraformEntity],
    refs: &[AttributeReference],
    evidence: &[(&Value, &Value)],
    r: &AttributeReference,
) -> Option<()> {
    let current = |node: &TerraformEntity| {
        node.provider.is_aws()
            && node.mode == EntityMode::Managed
            && node.action != Action::Delete
            && node.deposed_key.is_none()
    };
    if !current(&nodes[r.target])
        || !r.complete
        || !r.instances_explicit
        || !r.issues.is_empty()
        || refs
            .iter()
            .filter(|other| other.target == r.target && other.attribute == r.attribute)
            .count()
            != 1
    {
        return None;
    }
    let (v, m) = evidence[r.target];
    let known = |path: &str| super::nested_references::value(v, m, path)?.as_str();
    let actual: BTreeSet<_> = if r.attribute == "encryption_configuration" {
        if known("encryption_configuration.sse_algorithm")? != "aws:kms" {
            return None;
        }
        [known("encryption_configuration.kms_key_arn")?].into()
    } else {
        if v["rule"].as_array()?.len() != 1 {
            return None;
        }
        let rows = v["rule"][0]["destination"].as_array()?;
        if !(1..=5).contains(&rows.len()) {
            return None;
        }
        let values: BTreeSet<_> = (0..rows.len())
            .map(|i| {
                known(&format!(
                    "rule.0.destination.{i}.destination_table_bucket_arn"
                ))
            })
            .collect::<Option<_>>()?;
        if values.len() != rows.len() {
            return None;
        }
        values
    };
    let mut expected = BTreeSet::new();
    for &source in &r.sources {
        let endpoint = &nodes[source];
        let outputs = fields(
            &nodes[r.target].resource_type,
            &r.attribute,
            super::aliases::entity_type(endpoint),
        );
        let attrs = r.source_attributes_by_instance.get(&source)?;
        if source == r.target || !current(endpoint) || attrs.len() != 1 {
            return None;
        }
        let output = attrs.iter().next()?;
        if !outputs.contains(&output.as_str())
            || !expected.insert(super::service_evidence::known(evidence, source, output)?)
        {
            return None;
        }
    }
    (actual == expected && !actual.contains("")).then_some(())
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
            if matches!(
                r.attribute.as_str(),
                "encryption_configuration" | DESTINATION
            ) {
                return verify_set(nodes, refs, evidence, r).is_some();
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
