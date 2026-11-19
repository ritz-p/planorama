use super::configured_links::{self, Evidence, Rule};
use crate::model::{AttributeReference, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;

fn rule(kind: &str, path: &str) -> Option<Rule> {
    Some(match (kind, path) {
        ("aws_ami", "ebs_block_device.snapshot_id") => Rule::new(
            "Backing snapshot",
            &[
                ("aws_ebs_snapshot", &["id"]),
                ("aws_ebs_snapshot_copy", &["id"]),
                ("aws_ebs_snapshot_import", &["id"]),
            ],
            false,
            &["ebs_block_device"],
        ),
        ("aws_ami_copy", "source_ami_id") => Rule::new(
            "Source image",
            &[
                ("aws_ami", &["id"]),
                ("aws_ami_copy", &["id"]),
                ("aws_ami_from_instance", &["id"]),
            ],
            false,
            &[],
        )
        .with_explicit_destination(),
        ("aws_ami_copy", "kms_key_id") => Rule::new(
            "Copy encryption key",
            &[("aws_kms_key", &["arn"])],
            false,
            &[],
        ),
        ("aws_ami_from_instance", "source_instance_id") => {
            Rule::new("Source instance", &[("aws_instance", &["id"])], false, &[])
        }
        ("aws_ami_launch_permission", "image_id") => Rule::new(
            "Image",
            &[
                ("aws_ami", &["id"]),
                ("aws_ami_copy", &["id"]),
                ("aws_ami_from_instance", &["id"]),
            ],
            false,
            &[],
        ),
        ("aws_ami_launch_permission", "organization_arn") => Rule::new(
            "Grantee organization",
            &[("aws_organizations_organization", &["arn"])],
            false,
            &[],
        ),
        ("aws_ami_launch_permission", "organizational_unit_arn") => Rule::new(
            "Grantee organizational unit",
            &[("aws_organizations_organizational_unit", &["arn"])],
            false,
            &[],
        ),
        ("aws_snapshot_create_volume_permission", "snapshot_id") => Rule::new(
            "Snapshot",
            &[
                ("aws_ebs_snapshot", &["id"]),
                ("aws_ebs_snapshot_copy", &["id"]),
                ("aws_ebs_snapshot_import", &["id"]),
            ],
            false,
            &[],
        ),
        _ => return None,
    })
}
pub(crate) fn resource(kind: &str) -> bool {
    matches!(
        kind,
        "aws_ami"
            | "aws_ami_copy"
            | "aws_ami_from_instance"
            | "aws_ami_launch_permission"
            | "aws_snapshot_create_volume_permission"
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
        && (raw.nodes[r.target].resource_type != "aws_ami_copy"
            || r.attribute != "source_ami_id"
            || copy_account(raw, r, "aws_ami_launch_permission", "image_id"))
}
pub(super) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    values: &[(&Value, &Value)],
) {
    configured_links::annotate(nodes, refs, values, rule, qualifies);
    for grant in refs.iter_mut().filter(|r| {
        r.service_reference_verified
            && matches!(
                (nodes[r.target].resource_type.as_str(), r.attribute.as_str()),
                ("aws_ami_launch_permission", "image_id")
                    | ("aws_snapshot_create_volume_permission", "snapshot_id")
            )
    }) {
        grant.service_reference_owner_account =
            super::service_evidence::known(values, grant.target, "account_id")
                .and_then(crate::model::AwsAccountScope::new);
    }
}
fn qualifies(e: &Evidence<'_>, r: &AttributeReference, source: usize, path: &str) -> bool {
    match r.attribute.as_str() {
        "source_ami_id" => e.source_region(r.target, "source_ami_region", source),
        "kms_key_id" => e.value(r.target, "encrypted").and_then(Value::as_bool) == Some(true),
        "organization_arn" | "organizational_unit_arn" => [
            "account_id",
            "group",
            "organization_arn",
            "organizational_unit_arn",
        ]
        .iter()
        .all(|&k| k == path || e.absent(r.target, k)),
        _ => true,
    }
}
#[cfg(test)]
#[path = "../../../tests/unit/semantic/ec2_images.rs"]
mod tests;

pub(super) fn copy_account(
    raw: &Input,
    r: &AttributeReference,
    permission: &str,
    asset: &str,
) -> bool {
    if configured_links::destination_in_same_account(raw, r) {
        return true;
    }
    let Some(recipient) = raw.nodes[r.target]
        .provider_configuration
        .as_ref()
        .and_then(|c| c.account.as_ref())
    else {
        return false;
    };
    let [source] = r.sources.as_slice() else {
        return false;
    };
    let Some(owner) = raw.nodes[*source]
        .provider_configuration
        .as_ref()
        .and_then(|c| c.account.as_ref())
    else {
        return false;
    };
    raw.attributes.iter().any(|grant| {
        raw.nodes[grant.target].resource_type == permission
            && grant.attribute == asset
            && grant.sources == [*source]
            && grant.service_reference_verified
            && grant.service_reference_owner_account.as_ref() == Some(recipient)
            && raw.nodes[grant.target]
                .provider_configuration
                .as_ref()
                .is_some_and(|p| !p.account_conflict && p.account.as_ref() == Some(owner))
            && configured_links::bindings(raw, grant, rule)
    })
}
