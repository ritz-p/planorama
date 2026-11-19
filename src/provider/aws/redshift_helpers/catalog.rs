use super::{CLUSTER, WORKGROUP, nested_references};
use serde_json::Value;

const APPLICATION: &str = "aws_redshift_idc_application";
const INTEGRATION: &str = "aws_redshift_integration";
const REGISTRATION: &str = "aws_redshift_namespace_registration";
const CONSUMER: &str = "aws_redshift_data_share_consumer_association";
const POLICY: &str = "aws_redshift_resource_policy";
const SERVERLESS_POLICY: &str = "aws_redshiftserverless_resource_policy";
const NAMESPACE: &str = "aws_redshiftserverless_namespace";
pub(super) fn resource(kind: &str) -> bool {
    matches!(
        kind,
        APPLICATION | INTEGRATION | REGISTRATION | CONSUMER | POLICY | SERVERLESS_POLICY
    )
}
pub(super) fn has_indexed(kind: &str) -> bool {
    kind == APPLICATION
}
pub(super) fn indexed(kind: &str, path: &str) -> bool {
    kind == APPLICATION && path == "authorized_token_issuer.0.trusted_token_issuer_arn"
}
pub(super) fn owner(kind: &str) -> Option<(&'static str, &'static str, &'static str)> {
    matches!(kind, POLICY | SERVERLESS_POLICY).then_some((
        "resource_arn",
        "Resource policy owner",
        "Resource policies",
    ))
}
pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    match (kind, path) {
        (APPLICATION, "iam_role_arn") => Some("Identity Center application role"),
        (APPLICATION, "idc_instance_arn") => Some("Identity Center instance"),
        (APPLICATION, "authorized_token_issuer.trusted_token_issuer_arn") => {
            Some("Trusted token issuer")
        }
        (INTEGRATION, "source_arn") => Some("Integration source"),
        (INTEGRATION, "target_arn") => Some("Integration target namespace"),
        (INTEGRATION, "kms_key_id") => Some("Integration encryption key"),
        (REGISTRATION, "provisioned_cluster_identifier") => Some("Registered cluster"),
        (REGISTRATION, "serverless_namespace_identifier") => Some("Registered namespace"),
        (REGISTRATION, "serverless_workgroup_identifier") => Some("Registered workgroup"),
        (CONSUMER, "consumer_arn") => Some("Data share consumer namespace"),
        (POLICY | SERVERLESS_POLICY, "resource_arn") => Some("Resource policy owner"),
        _ => None,
    }
}
pub(super) fn fields(kind: &str, path: &str, source: &str) -> &'static [&'static str] {
    match (kind, path, source) {
        (APPLICATION, "iam_role_arn", "aws_iam_role")
        | (INTEGRATION, "source_arn", "aws_dynamodb_table" | "aws_s3_bucket")
        | (INTEGRATION, "target_arn", NAMESPACE)
        | (POLICY, "resource_arn", NAMESPACE)
        | (CONSUMER, "consumer_arn", NAMESPACE)
        | (SERVERLESS_POLICY, "resource_arn", "aws_redshiftserverless_snapshot" | WORKGROUP) => {
            &["arn"]
        }
        (
            APPLICATION,
            "authorized_token_issuer.trusted_token_issuer_arn",
            "aws_ssoadmin_trusted_token_issuer",
        ) => &["arn", "id"],
        (INTEGRATION, "kms_key_id", "aws_kms_key") => &["arn", "key_id", "id"],
        (INTEGRATION, "target_arn", CLUSTER)
        | (POLICY, "resource_arn", CLUSTER)
        | (CONSUMER, "consumer_arn", CLUSTER) => &["cluster_namespace_arn"],
        (REGISTRATION, "provisioned_cluster_identifier", CLUSTER) => &["cluster_identifier"],
        (REGISTRATION, "serverless_namespace_identifier", NAMESPACE) => {
            &["namespace_id", "namespace_name"]
        }
        (REGISTRATION, "serverless_workgroup_identifier", WORKGROUP) => &["workgroup_name"],
        _ => &[],
    }
}

pub(super) fn valid_mode(
    kind: &str,
    path: &str,
    v: &Value,
    m: &Value,
    source: (&Value, &Value),
) -> bool {
    let field = |p| nested_references::value(v, m, p);
    let known = |p| field(p).and_then(Value::as_str).filter(|s| !s.is_empty());
    let absent = |p| field(p).is_some_and(Value::is_null);
    match (kind, path) {
        (APPLICATION, "authorized_token_issuer.trusted_token_issuer_arn") => {
            let instance = known("idc_instance_arn");
            nested_references::singleton(v, m, "authorized_token_issuer")
                && instance.is_some()
                && instance
                    == nested_references::value(source.0, source.1, "instance_arn")
                        .and_then(Value::as_str)
        }
        (REGISTRATION, "provisioned_cluster_identifier") => {
            known("namespace_type") == Some("provisioned")
                && absent("serverless_namespace_identifier")
                && absent("serverless_workgroup_identifier")
        }
        (REGISTRATION, "serverless_namespace_identifier" | "serverless_workgroup_identifier") => {
            known("namespace_type") == Some("serverless")
                && absent("provisioned_cluster_identifier")
                && known("serverless_namespace_identifier").is_some()
                && known("serverless_workgroup_identifier").is_some()
        }
        (CONSUMER, "consumer_arn") => {
            absent("consumer_region")
                && field("associate_entire_account")
                    .is_some_and(|v| v.is_null() || v.as_bool() == Some(false))
        }
        _ => true,
    }
}
pub(super) fn compatible_endpoint(
    kind: &str,
    path: &str,
    node: &crate::model::TerraformEntity,
    value: Option<&str>,
) -> bool {
    let expected = match (kind, path, node.resource_type.as_str()) {
        (INTEGRATION, "target_arn", CLUSTER)
        | (CONSUMER, "consumer_arn", CLUSTER)
        | (POLICY, "resource_arn", CLUSTER) => ("redshift", "namespace:"),
        (INTEGRATION, "target_arn", NAMESPACE)
        | (CONSUMER, "consumer_arn", NAMESPACE)
        | (POLICY, "resource_arn", NAMESPACE) => ("redshift-serverless", "namespace/"),
        (SERVERLESS_POLICY, "resource_arn", WORKGROUP) => ("redshift-serverless", "workgroup/"),
        (SERVERLESS_POLICY, "resource_arn", "aws_redshiftserverless_snapshot") => {
            ("redshift-serverless", "snapshot/")
        }
        (INTEGRATION, "source_arn", "aws_dynamodb_table") => ("dynamodb", "table/"),
        _ => return true,
    };
    let Some(value) = value else { return false };
    let parts: Vec<_> = value.splitn(6, ':').collect();
    let ["arn", partition, service, region, account, resource] = parts.as_slice() else {
        return false;
    };
    let scope = crate::provider::aws::scope::scope(node);
    !partition.is_empty()
        && *service == expected.0
        && scope
            .as_ref()
            .and_then(|s| s.region.as_ref())
            .is_some_and(|r| r.0 == *region)
        && crate::model::AwsAccountScope::new(account).is_some()
        && node
            .provider_configuration
            .as_ref()
            .and_then(|p| p.account.as_ref())
            .is_none_or(|a| a.as_str() == *account)
        && resource
            .strip_prefix(expected.1)
            .is_some_and(|s| !s.is_empty() && !s.contains([':', '/']))
}
