use crate::model::{AttributeReference, AwsAccountScope, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/redshift_cluster.rs"]
mod tests;
const CLUSTER: &str = "aws_redshift_cluster";
const HELPER: &str = "aws_redshift_cluster_iam_roles";

pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    match (kind, path) {
        (CLUSTER, "iam_roles") | (HELPER, "iam_role_arns") => Some("Cluster IAM roles"),
        (CLUSTER | HELPER, "default_iam_role_arn") => Some("Default cluster role"),
        (HELPER, "cluster_identifier") => Some("IAM roles cluster"),
        (CLUSTER, "kms_key_id") => Some("Cluster encryption key"),
        (CLUSTER, "master_password_secret_kms_key_id") => Some("Credentials encryption key"),
        (CLUSTER, "snapshot_arn" | "snapshot_identifier") => Some("Restore snapshot"),
        _ => None,
    }
}
fn fields(kind: &str, path: &str, source: &str) -> &'static [&'static str] {
    match (kind, path, source) {
        (CLUSTER, "iam_roles", "aws_iam_role")
        | (HELPER, "iam_role_arns", "aws_iam_role")
        | (CLUSTER | HELPER, "default_iam_role_arn", "aws_iam_role")
        | (CLUSTER, "kms_key_id", "aws_kms_key")
        | (CLUSTER, "snapshot_arn", "aws_redshift_cluster_snapshot") => &["arn"],
        (CLUSTER, "master_password_secret_kms_key_id", "aws_kms_key") => &["key_id", "arn"],
        (HELPER, "cluster_identifier", CLUSTER) => &["cluster_identifier"],
        (CLUSTER, "snapshot_identifier", "aws_redshift_cluster_snapshot") => {
            &["snapshot_identifier"]
        }
        _ => &[],
    }
}
fn restore(path: &str) -> bool {
    matches!(path, "snapshot_arn" | "snapshot_identifier")
}
pub(super) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    e: &[(&Value, &Value)],
) {
    let results: Vec<_> = refs
        .iter()
        .map(|r| {
            label(&nodes[r.target].resource_type, &r.attribute)?;
            let (v, m) = e[r.target];
            let value = |p| super::nested_references::value(v, m, p);
            let proof = || {
                let actual = value(&r.attribute)?;
                if if matches!(r.attribute.as_str(), "iam_roles" | "iam_role_arns") {
                    !actual.is_array()
                } else {
                    !actual.is_string()
                } {
                    return None;
                }
                if r.attribute == "kms_key_id" && value("encrypted")?.as_str() != Some("true") {
                    return None;
                }
                if r.attribute == "master_password_secret_kms_key_id"
                    && value("manage_master_password")?.as_bool() != Some(true)
                {
                    return None;
                }
                let mut account = None;
                if restore(&r.attribute) {
                    let other = if r.attribute == "snapshot_arn" {
                        "snapshot_identifier"
                    } else {
                        "snapshot_arn"
                    };
                    if !value(other)?.is_null() {
                        return None;
                    }
                    let owner = value("owner_account")?;
                    if !owner.is_null() {
                        account = Some(AwsAccountScope::new(owner.as_str()?)?);
                    }
                    let qualifier = value("snapshot_cluster_identifier")?;
                    if !qualifier.is_null()
                        && !r.sources.iter().all(|&s| {
                            super::service_evidence::known(e, s, "cluster_identifier")
                                == qualifier.as_str()
                        })
                    {
                        return None;
                    }
                    if !r.sources.iter().all(|&s| {
                        account.as_ref().is_none_or(|a| {
                            super::service_evidence::known(e, s, "owner_account")
                                .and_then(AwsAccountScope::new)
                                .as_ref()
                                == Some(a)
                        })
                    }) {
                        return None;
                    }
                }
                super::nested_references::verify(nodes, refs, e, r, fields, |_, _| true)?;
                Some(account)
            };
            Some(proof())
        })
        .collect();
    for (r, result) in refs.iter_mut().zip(results) {
        if let Some(proof) = result {
            r.service_reference_verified = proof.is_some();
            r.service_reference_owner_account = proof.flatten();
        }
    }
    super::network::annotate_subnet_group(nodes, refs, e, CLUSTER, "cluster_subnet_group_name");
}

pub(super) fn bindings(raw: &Input, r: &AttributeReference) -> bool {
    if label(&raw.nodes[r.target].resource_type, &r.attribute).is_none() {
        return true;
    }
    r.sources.iter().all(|&s| {
        let l = &raw.nodes[r.target];
        let right = &raw.nodes[s];
        let mut binding = l.provider_configuration.clone();
        if restore(&r.attribute) {
            if let Some(account) = &r.service_reference_owner_account {
                let Some(b) = binding.as_mut() else {
                    return false;
                };
                if b.account_conflict {
                    return false;
                }
                b.account = Some(account.clone());
            }
        }
        super::eventbridge::bindings(
            binding.as_ref(),
            right.provider_configuration.as_ref(),
            l.entity.scope.as_ref().and_then(|s| s.region.as_ref()),
            right.entity.scope.as_ref().and_then(|s| s.region.as_ref()),
            right.resource_type == "aws_iam_role",
        )
    })
}
