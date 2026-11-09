use crate::model::{AttributeReference, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/kms_references.rs"]
mod tests;

pub(super) fn key_outputs(kind: &str) -> &'static [&'static str] {
    match kind {
        "aws_kms_key" | "aws_kms_replica_key" | "aws_kms_replica_external_key" => {
            &["id", "key_id", "arn"]
        }
        "aws_kms_external_key" => &["id", "arn"],
        _ => &[],
    }
}

pub(super) fn resource(kind: &str) -> bool {
    !key_outputs(kind).is_empty()
        || matches!(
            kind,
            "aws_kms_alias"
                | "aws_kms_ciphertext"
                | "aws_kms_custom_key_store"
                | "aws_kms_grant"
                | "aws_kms_key_policy"
        )
}

pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    match (kind, path) {
        ("aws_kms_alias", "target_key_id") => Some("Alias key"),
        ("aws_kms_ciphertext" | "aws_kms_grant" | "aws_kms_key_policy", "key_id") => {
            Some("Configured KMS key")
        }
        ("aws_kms_key", "custom_key_store_id") => Some("Custom key store"),
        ("aws_kms_custom_key_store", "cloud_hsm_cluster_id") => Some("CloudHSM cluster"),
        ("aws_kms_custom_key_store", "xks_proxy_vpc_endpoint_service_name") => {
            Some("External key proxy service")
        }
        ("aws_kms_grant", "grantee_principal") => Some("Grantee principal"),
        ("aws_kms_grant", "retiring_principal") => Some("Retiring principal"),
        ("aws_kms_replica_key" | "aws_kms_replica_external_key", "primary_key_arn") => {
            Some("Primary KMS key")
        }
        _ => None,
    }
}

pub(super) fn fields(kind: &str, path: &str, source: &str) -> &'static [&'static str] {
    match (kind, path, source) {
        ("aws_kms_alias", "target_key_id", _)
        | ("aws_kms_ciphertext" | "aws_kms_grant" | "aws_kms_key_policy", "key_id", _) => {
            key_outputs(source)
        }
        ("aws_kms_key", "custom_key_store_id", "aws_kms_custom_key_store") => &["id"],
        ("aws_kms_custom_key_store", "cloud_hsm_cluster_id", "aws_cloudhsm_v2_cluster") => {
            &["cluster_id", "id"]
        }
        (
            "aws_kms_custom_key_store",
            "xks_proxy_vpc_endpoint_service_name",
            "aws_vpc_endpoint_service",
        ) => &["service_name"],
        (
            "aws_kms_grant",
            "grantee_principal" | "retiring_principal",
            "aws_iam_role" | "aws_iam_user",
        )
        | ("aws_kms_replica_key", "primary_key_arn", "aws_kms_key")
        | ("aws_kms_replica_external_key", "primary_key_arn", "aws_kms_external_key") => &["arn"],
        _ => &[],
    }
}

pub(crate) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    evidence: &[(&Value, &Value)],
) {
    let mut counts = std::collections::BTreeMap::new();
    for r in refs.iter() {
        *counts
            .entry((r.target, r.attribute.clone()))
            .or_insert(0usize) += 1;
    }
    for r in refs {
        if label(&nodes[r.target].resource_type, &r.attribute).is_none() {
            continue;
        }
        let [source] = r.sources.as_slice() else {
            r.service_reference_verified = false;
            continue;
        };
        let known = |i, p| super::service_evidence::known(evidence, i, p);
        let gate = match r.attribute.as_str() {
            "cloud_hsm_cluster_id" => {
                known(r.target, "custom_key_store_type") == Some("AWS_CLOUDHSM")
            }
            "xks_proxy_vpc_endpoint_service_name" => {
                known(r.target, "custom_key_store_type") == Some("EXTERNAL_KEY_STORE")
                    && known(r.target, "xks_proxy_connectivity") == Some("VPC_ENDPOINT_SERVICE")
            }
            "primary_key_arn" => {
                super::service_evidence::known_value(evidence, *source, &["multi_region"])
                    == Some(&Value::Bool(true))
            }
            _ => true,
        };
        r.service_reference_verified &= counts[&(r.target, r.attribute.clone())] == 1
            && gate
            && known(r.target, &r.attribute).is_some()
            && r.source_attributes
                .iter()
                .all(|p| known(*source, p).is_some());
    }
}

pub(super) fn bindings(raw: &Input, r: &AttributeReference) -> bool {
    let node = &raw.nodes[r.target];
    if label(&node.resource_type, &r.attribute).is_none() {
        return true;
    }
    r.sources.iter().all(|&s| {
        let endpoint = &raw.nodes[s];
        let region = node.entity.scope.as_ref().and_then(|s| s.region.as_ref());
        let other = endpoint
            .entity
            .scope
            .as_ref()
            .and_then(|s| s.region.as_ref());
        super::iam_policies::same_account(
            node.provider_configuration.as_ref(),
            endpoint.provider_configuration.as_ref(),
        ) && if r.attribute == "primary_key_arn" {
            region.zip(other).is_some_and(|(a, b)| a != b)
        } else if matches!(
            endpoint.resource_type.as_str(),
            "aws_iam_role" | "aws_iam_user"
        ) {
            region.is_some()
        } else {
            region.is_some() && region == other
        }
    })
}
