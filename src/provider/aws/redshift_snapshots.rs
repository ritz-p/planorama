use crate::model::{AttributeReference, AwsAccountScope, RegionScope, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;
#[cfg(test)]
#[path = "../../../tests/unit/semantic/redshift_snapshots.rs"]
mod tests;
const COPY: &str = "aws_redshift_snapshot_copy";
const GRANT: &str = "aws_redshift_snapshot_copy_grant";
const ASSOCIATION: &str = "aws_redshift_snapshot_schedule_association";
pub(super) fn resource(k: &str) -> bool {
    matches!(
        k,
        COPY | GRANT
            | ASSOCIATION
            | "aws_redshift_cluster_snapshot"
            | "aws_redshiftserverless_snapshot"
    )
}
pub(super) fn owner(k: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match k {
        COPY | ASSOCIATION => Some((
            "cluster_identifier",
            "Snapshot configuration cluster",
            "Snapshot configuration",
        )),
        _ => None,
    }
}
pub(super) fn label(k: &str, p: &str) -> Option<&'static str> {
    match (k, p) {
        ("aws_redshift_cluster_snapshot", "cluster_identifier") => Some("Snapshot source cluster"),
        (COPY | ASSOCIATION, "cluster_identifier") => Some("Snapshot configuration cluster"),
        (COPY, "snapshot_copy_grant_name") => Some("Destination snapshot grant"),
        (GRANT, "kms_key_id") => Some("Snapshot grant key"),
        (ASSOCIATION, "schedule_identifier") => Some("Snapshot schedule"),
        ("aws_redshiftserverless_snapshot", "namespace_name") => Some("Snapshot source namespace"),
        _ => None,
    }
}
fn fields(k: &str, p: &str, s: &str) -> &'static [&'static str] {
    match (k, p, s) {
        (
            "aws_redshift_cluster_snapshot" | COPY | ASSOCIATION,
            "cluster_identifier",
            "aws_redshift_cluster",
        ) => &["cluster_identifier"],
        (COPY, "snapshot_copy_grant_name", GRANT) => &["snapshot_copy_grant_name"],
        (GRANT, "kms_key_id", "aws_kms_key") => &["id", "key_id", "arn"],
        (ASSOCIATION, "schedule_identifier", "aws_redshift_snapshot_schedule") => &["identifier"],
        (
            "aws_redshiftserverless_snapshot",
            "namespace_name",
            "aws_redshiftserverless_namespace",
        ) => &["namespace_name"],
        _ => &[],
    }
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
            let proof = || {
                let (v, m) = e[r.target];
                if !super::nested_references::value(v, m, &r.attribute)?.is_string() {
                    return None;
                }
                super::nested_references::verify(nodes, refs, e, r, fields, |_, _| true)?;
                let mut region = None;
                let mut account = None;
                if r.attribute == "snapshot_copy_grant_name" {
                    region = Some(RegionScope(
                        super::service_evidence::known(e, r.target, "destination_region")?.into(),
                    ));
                }
                if r.attribute == "kms_key_id" && r.source_attributes.contains("arn") {
                    let arn = super::service_evidence::known(e, r.target, "kms_key_id")?;
                    if !super::s3_exports::kms_arn("aws_kms_key", arn) {
                        return None;
                    }
                    let parts: Vec<_> = arn.splitn(6, ':').collect();
                    region = Some(RegionScope(parts[3].into()));
                    account = Some(AwsAccountScope::new(parts[4])?);
                }
                Some((region, account))
            };
            Some(proof())
        })
        .collect();
    for (r, result) in refs.iter_mut().zip(results) {
        if let Some(proof) = result {
            r.service_reference_verified = proof.is_some();
            let (region, account) = proof.unwrap_or_default();
            r.service_reference_region = region;
            r.service_reference_owner_account = account;
        }
    }
}
pub(super) fn bindings(raw: &Input, r: &AttributeReference) -> bool {
    if label(&raw.nodes[r.target].resource_type, &r.attribute).is_none() {
        return true;
    }
    let l = &raw.nodes[r.target];
    let region = l.entity.scope.as_ref().and_then(|s| s.region.as_ref());
    r.sources.iter().all(|&s| {
        let right = &raw.nodes[s];
        let source_region = right.entity.scope.as_ref().and_then(|s| s.region.as_ref());
        let expected = if r.attribute == "snapshot_copy_grant_name" {
            r.service_reference_region.as_ref()
        } else {
            region
        };
        if region.is_none() || expected.is_none() || expected != source_region {
            return false;
        }
        let ordinary = super::iam_policies::same_account(
            l.provider_configuration.as_ref(),
            right.provider_configuration.as_ref(),
        );
        if let Some(account) = &r.service_reference_owner_account {
            if r.service_reference_region.as_ref() != region {
                return false;
            }
            let Some(source) = right.provider_configuration.as_ref() else {
                return false;
            };
            if source.account_conflict || source.account.as_ref().is_some_and(|a| a != account) {
                return false;
            }
            ordinary
                || (source.account.as_ref() == Some(account)
                    && l.provider_configuration
                        .as_ref()
                        .is_some_and(|b| !b.account_conflict && b.account.is_some()))
        } else {
            ordinary
        }
    })
}
