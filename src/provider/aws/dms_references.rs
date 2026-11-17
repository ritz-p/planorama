use crate::model::{AttributeReference, AwsAccountScope, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/dms_references.rs"]
mod tests;

const ENDPOINT: &str = "aws_dms_endpoint";
const S3: &str = "aws_dms_s3_endpoint";

pub(crate) fn resource(kind: &str) -> bool {
    matches!(kind, ENDPOINT | S3)
}

pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    match (kind, path) {
        (ENDPOINT | S3, "certificate_arn") => Some("DMS certificate"),
        (ENDPOINT | S3, "kms_key_arn") => Some("Connection encryption key"),
        (ENDPOINT, "secrets_manager_arn") => Some("Endpoint credentials secret"),
        (
            ENDPOINT,
            "secrets_manager_access_role_arn"
            | "service_access_role"
            | "kinesis_settings.service_access_role_arn"
            | "redshift_settings.service_access_role_arn",
        )
        | (S3, "service_access_role_arn") => Some("DMS access role"),
        (ENDPOINT, "kinesis_settings.stream_arn") => Some("Kinesis endpoint stream"),
        (ENDPOINT, "redshift_settings.bucket_name") | (S3, "bucket_name") => {
            Some("DMS storage bucket")
        }
        (ENDPOINT, "redshift_settings.server_side_encryption_kms_key_id")
        | (S3, "server_side_encryption_kms_key_id") => Some("Storage encryption key"),
        _ => None,
    }
}

pub(crate) fn indexed(kind: &str, path: &str) -> bool {
    path.split('.').any(|part| part.parse::<usize>().is_ok())
        && label(
            kind,
            &path
                .split('.')
                .filter(|part| part.parse::<usize>().is_err())
                .collect::<Vec<_>>()
                .join("."),
        )
        .is_some()
}

fn fields(kind: &str, path: &str, source: &str) -> &'static [&'static str] {
    match (kind, path, source) {
        (ENDPOINT | S3, "certificate_arn", "aws_dms_certificate") => &["certificate_arn"],
        (ENDPOINT | S3, "kms_key_arn", "aws_kms_key") => &["arn"],
        (ENDPOINT, "secrets_manager_arn", "aws_secretsmanager_secret") => &["arn", "id", "name"],
        (
            ENDPOINT,
            "secrets_manager_access_role_arn"
            | "service_access_role"
            | "kinesis_settings.service_access_role_arn"
            | "redshift_settings.service_access_role_arn",
            "aws_iam_role",
        )
        | (S3, "service_access_role_arn", "aws_iam_role") => &["arn"],
        (ENDPOINT, "kinesis_settings.stream_arn", "aws_kinesis_stream") => &["arn"],
        (ENDPOINT, "redshift_settings.bucket_name", "aws_s3_bucket")
        | (S3, "bucket_name", "aws_s3_bucket") => &["bucket", "id"],
        (ENDPOINT, "redshift_settings.server_side_encryption_kms_key_id", "aws_kms_key") => {
            &["arn", "key_id", "id"]
        }
        (S3, "server_side_encryption_kms_key_id", "aws_kms_key") => &["arn"],
        _ => &[],
    }
}

fn configuration(
    kind: &str,
    path: &str,
    value: &Value,
    mask: &Value,
) -> Option<Option<AwsAccountScope>> {
    let get = |path| super::nested_references::value(value, mask, path);
    let scalar_path = path.split_once('.').map_or_else(
        || path.to_owned(),
        |(block, field)| format!("{block}.0.{field}"),
    );
    if get(&scalar_path)?.as_str()?.is_empty() {
        return None;
    }
    let direction = get("endpoint_type")?.as_str()?;
    if !matches!(direction, "source" | "target") {
        return None;
    }
    if kind == ENDPOINT {
        for block in ["kinesis_settings", "redshift_settings"] {
            if path.starts_with(&format!("{block}."))
                && !super::nested_references::singleton(value, mask, block)
            {
                return None;
            }
        }
        let engine = get("engine_name")?.as_str()?;
        if !matches!(
            engine,
            "aurora"
                | "aurora-postgresql"
                | "aurora-serverless"
                | "aurora-postgresql-serverless"
                | "azuredb"
                | "azure-sql-managed-instance"
                | "babelfish"
                | "db2"
                | "db2-zos"
                | "docdb"
                | "dynamodb"
                | "elasticsearch"
                | "kafka"
                | "kinesis"
                | "mariadb"
                | "mongodb"
                | "mysql"
                | "opensearch"
                | "oracle"
                | "postgres"
                | "redis"
                | "redshift"
                | "redshift-serverless"
                | "sqlserver"
                | "neptune"
                | "sybase"
        ) {
            return None;
        }
        if path.starts_with("kinesis_settings.") && (engine != "kinesis" || direction != "target") {
            return None;
        }
        if path.starts_with("redshift_settings.") && (engine != "redshift" || direction != "target")
        {
            return None;
        }
        if path == "service_access_role" && (engine != "dynamodb" || direction != "target") {
            return None;
        }
        if path.starts_with("secrets_manager_")
            && (!matches!(
                engine,
                "aurora"
                    | "aurora-postgresql"
                    | "mariadb"
                    | "mongodb"
                    | "mysql"
                    | "oracle"
                    | "postgres"
                    | "redshift"
                    | "sqlserver"
                    | "babelfish"
                    | "sybase"
                    | "db2"
                    | "db2-zos"
            ) || get("secrets_manager_arn")?.as_str()?.is_empty()
                || get("secrets_manager_access_role_arn")?.as_str()?.is_empty())
        {
            return None;
        }
    }
    if path.ends_with("server_side_encryption_kms_key_id") {
        let mode = if kind == S3 {
            "encryption_mode"
        } else {
            "redshift_settings.0.encryption_mode"
        };
        if direction != "target" || get(mode)?.as_str()? != "SSE_KMS" {
            return None;
        }
    }
    if kind == S3 && path == "bucket_name" {
        let owner = get("expected_bucket_owner")?;
        if !owner.is_null() && owner.as_str() != Some("") {
            return Some(Some(AwsAccountScope::new(owner.as_str()?)?));
        }
    }
    Some(None)
}

pub(super) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    evidence: &[(&Value, &Value)],
) {
    let results: Vec<_> = refs
        .iter()
        .map(|r| {
            let kind = &nodes[r.target].resource_type;
            label(kind, &r.attribute)?;
            let (v, m) = evidence[r.target];
            Some(configuration(kind, &r.attribute, v, m).filter(|_| {
                super::nested_references::verify(nodes, refs, evidence, r, fields, |_, _| true)
                    .is_some()
            }))
        })
        .collect();
    for (r, result) in refs.iter_mut().zip(results) {
        if let Some(proof) = result {
            r.service_reference_verified = proof.is_some();
            r.service_reference_owner_account = proof.flatten();
        }
    }
}

pub(super) fn bindings(raw: &Input, reference: &AttributeReference) -> bool {
    label(
        &raw.nodes[reference.target].resource_type,
        &reference.attribute,
    )
    .is_none()
        || super::databases::bindings(raw, reference)
}
