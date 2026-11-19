use super::configured_links::{self, Evidence, Rule};
use crate::model::{AttributeReference, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;

fn rule(kind: &str, path: &str) -> Option<Rule> {
    Some(match (kind, path) {
        ("aws_dlm_lifecycle_policy", "execution_role_arn") => Rule::new(
            "DLM execution role",
            &[("aws_iam_role", &["arn"])],
            false,
            &[],
        ),
        ("aws_dlm_lifecycle_policy", "policy_details.schedule.cross_region_copy_rule.cmk_arn") => {
            Rule::new(
                "Destination copy key",
                &[("aws_kms_key", &["arn"])],
                false,
                &[
                    "policy_details.schedule",
                    "policy_details.schedule.cross_region_copy_rule",
                ],
            )
            .with_explicit_destination()
        }
        (
            "aws_dlm_lifecycle_policy",
            "policy_details.action.cross_region_copy.encryption_configuration.cmk_arn",
        ) => Rule::new(
            "Destination copy key",
            &[("aws_kms_key", &["arn"])],
            false,
            &["policy_details.action.cross_region_copy"],
        )
        .with_explicit_destination(),
        (
            "aws_dlm_lifecycle_policy",
            "policy_details.schedule.create_rule.scripts.execution_handler",
        ) => Rule::new(
            "Snapshot script document",
            &[("aws_ssm_document", &["name", "arn"])],
            false,
            &["policy_details.schedule"],
        ),
        ("aws_drs_replication_configuration_template", "staging_area_subnet_id") => {
            Rule::new("Staging subnet", &[("aws_subnet", &["id"])], false, &[])
        }
        (
            "aws_drs_replication_configuration_template",
            "replication_servers_security_groups_ids",
        ) => Rule::new(
            "Staging security groups",
            &[("aws_security_group", &["id"])],
            true,
            &[],
        ),
        ("aws_drs_replication_configuration_template", "ebs_encryption_key_arn") => Rule::new(
            "Staging encryption key",
            &[("aws_kms_key", &["arn"])],
            false,
            &[],
        ),
        _ => return None,
    })
}
pub(crate) fn resource(kind: &str) -> bool {
    matches!(
        kind,
        "aws_dlm_lifecycle_policy" | "aws_drs_replication_configuration_template"
    )
}
pub(crate) fn indexed(kind: &str, path: &str) -> bool {
    resource(kind) && configured_links::indexed(kind, path, rule)
}
pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    rule(kind, path).map(|r| r.label)
}
pub(super) fn bindings(raw: &Input, r: &AttributeReference) -> bool {
    configured_links::bindings(raw, r, rule)
        && (raw.nodes[r.target].resource_type != "aws_dlm_lifecycle_policy"
            || !copy_key(&r.attribute)
            || r.service_reference_owner_account.is_some()
                && configured_links::destination_in_same_account(raw, r))
}
fn copy_key(path: &str) -> bool {
    matches!(
        path,
        "policy_details.schedule.cross_region_copy_rule.cmk_arn"
            | "policy_details.action.cross_region_copy.encryption_configuration.cmk_arn"
    )
}
pub(super) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    values: &[(&Value, &Value)],
) {
    configured_links::annotate(nodes, refs, values, rule, qualifies);
    for r in refs.iter_mut().filter(|r| {
        r.service_reference_verified
            && copy_key(&r.attribute)
            && nodes[r.target].resource_type == "aws_dlm_lifecycle_policy"
    }) {
        let accounts = r
            .sources
            .iter()
            .map(|&source| {
                let arn = super::service_evidence::known(values, source, "arn")?;
                crate::model::AwsAccountScope::new(arn.split(':').nth(4)?)
            })
            .collect::<Option<std::collections::BTreeSet<_>>>();
        r.service_reference_owner_account = accounts
            .filter(|accounts| accounts.len() == 1)
            .and_then(|mut accounts| accounts.pop_first());
    }
}
fn qualifies(e: &Evidence<'_>, r: &AttributeReference, source: usize, path: &str) -> bool {
    match r.attribute.as_str() {
        "staging_area_subnet_id" | "replication_servers_security_groups_ids" => {
            let subnet = ("staging_area_subnet_id", "aws_subnet");
            if e.value(r.target, "replication_servers_security_groups_ids")
                .is_some_and(|v| v.is_null() || v.as_array().is_some_and(Vec::is_empty))
            {
                return r.attribute == "staging_area_subnet_id"
                    && configured_links::network_context(e, r.target, &[subnet]).is_some();
            }
            configured_links::network_context(
                e,
                r.target,
                &[
                    subnet,
                    (
                        "replication_servers_security_groups_ids",
                        "aws_security_group",
                    ),
                ],
            )
            .is_some()
        }
        "ebs_encryption_key_arn" => e.text(r.target, "ebs_encryption") == Some("CUSTOM"),
        "policy_details.schedule.create_rule.scripts.execution_handler" => {
            e.text(source, "name").is_some_and(|name| {
                !matches!(
                    name,
                    "AWS_VSS_BACKUP" | "AWSSystemsManagerSAP-CreateDLMSnapshotForSAPHANA"
                )
            })
        }
        "policy_details.schedule.cross_region_copy_rule.cmk_arn"
        | "policy_details.action.cross_region_copy.encryption_configuration.cmk_arn" => {
            destination_matches(e, r, source, path)
        }
        _ => true,
    }
}
fn destination_matches(
    e: &Evidence<'_>,
    r: &AttributeReference,
    source: usize,
    path: &str,
) -> bool {
    let Some((parent, _)) = path.rsplit_once('.') else {
        return false;
    };
    let selector = match (
        r.attribute.as_str(),
        e.text(r.target, "policy_details.0.policy_type"),
    ) {
        (
            "policy_details.action.cross_region_copy.encryption_configuration.cmk_arn",
            Some("EVENT_BASED_POLICY"),
        ) => parent
            .strip_suffix(".encryption_configuration.0")
            .map(|p| format!("{p}.target")),
        ("policy_details.schedule.cross_region_copy_rule.cmk_arn", Some("IMAGE_MANAGEMENT")) => {
            Some(format!("{parent}.target_region"))
        }
        (
            "policy_details.schedule.cross_region_copy_rule.cmk_arn",
            Some("EBS_SNAPSHOT_MANAGEMENT"),
        ) => Some(format!("{parent}.target")),
        _ => None,
    };
    let Some(destination) = selector
        .as_deref()
        .and_then(|selector| e.text(r.target, selector))
    else {
        return false;
    };
    let Some(arn) = e.text(source, "arn") else {
        return false;
    };
    let mut arn = arn.splitn(6, ':');
    let [
        Some("arn"),
        Some("aws"),
        Some("kms"),
        Some(region),
        Some(account),
        Some(key),
    ] = std::array::from_fn(|_| arn.next())
    else {
        return false;
    };
    let destination_matches = if destination.contains(':') {
        let mut outpost = destination.splitn(6, ':');
        let [
            Some("arn"),
            Some("aws"),
            Some("outposts"),
            Some(outpost_region),
            Some(owner),
            Some(resource),
        ] = std::array::from_fn(|_| outpost.next())
        else {
            return false;
        };
        outpost_region == region
            && owner == account
            && resource
                .strip_prefix("outpost/")
                .is_some_and(|id| !id.is_empty())
    } else {
        destination == region
    };
    destination_matches
        && key.strip_prefix("key/").is_some_and(|key| !key.is_empty())
        && crate::model::AwsAccountScope::new(account).is_some()
        && super::scope::scope(&e.nodes[source])
            .and_then(|scope| scope.region)
            .is_some_and(|scope| scope.0 == region)
}
#[cfg(test)]
#[path = "../../../tests/unit/semantic/recovery_configuration.rs"]
mod tests;
