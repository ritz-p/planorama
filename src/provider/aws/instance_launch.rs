use super::configured_links::{self, Evidence, Rule};
use crate::model::{AttributeReference, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;

fn rule(kind: &str, path: &str) -> Option<Rule> {
    Some(match (kind, path) {
        ("aws_instance", "ami") => Rule::new(
            "Launch image",
            &[
                ("aws_ami", &["id"]),
                ("aws_ami_copy", &["id"]),
                ("aws_ami_from_instance", &["id"]),
            ],
            false,
            &[],
        ),
        ("aws_instance", "iam_instance_profile") => Rule::new(
            "Launch instance profile",
            &[("aws_iam_instance_profile", &["name"])],
            false,
            &[],
        ),
        ("aws_instance", "key_name") => Rule::new(
            "Launch key pair",
            &[("aws_key_pair", &["key_name"])],
            false,
            &[],
        ),
        ("aws_instance", "launch_template.id") => Rule::new(
            "Launch template",
            &[("aws_launch_template", &["id"])],
            false,
            &[],
        ),
        ("aws_instance", "launch_template.name") => Rule::new(
            "Launch template",
            &[("aws_launch_template", &["name"])],
            false,
            &[],
        ),
        ("aws_instance", "host_id") => {
            Rule::new("Dedicated host", &[("aws_ec2_host", &["id"])], false, &[])
        }
        ("aws_instance", "placement_group") => Rule::new(
            "Placement group",
            &[("aws_placement_group", &["name"])],
            false,
            &[],
        ),
        ("aws_instance", "placement_group_id") => Rule::new(
            "Placement group",
            &[("aws_placement_group", &["placement_group_id"])],
            false,
            &[],
        ),
        (
            "aws_instance",
            "capacity_reservation_specification.capacity_reservation_target.capacity_reservation_id",
        ) => Rule::new(
            "Capacity reservation",
            &[
                ("aws_ec2_capacity_reservation", &["id"]),
                ("aws_ec2_capacity_block_reservation", &["id"]),
            ],
            false,
            &[],
        ),
        ("aws_instance", "primary_network_interface.network_interface_id") => Rule::new(
            "Primary network interface",
            &[("aws_network_interface", &["id"])],
            false,
            &[],
        ),
        ("aws_instance", "network_interface.network_interface_id") => Rule::new(
            "Launch network interface",
            &[("aws_network_interface", &["id"])],
            false,
            &["network_interface"],
        ),
        ("aws_instance", "secondary_network_interface.secondary_subnet_id") => Rule::new(
            "Secondary subnet",
            &[("aws_ec2_secondary_subnet", &["secondary_subnet_id"])],
            false,
            &["secondary_network_interface"],
        ),
        ("aws_instance", "ebs_block_device.snapshot_id") => Rule::new(
            "Device snapshot",
            &[
                ("aws_ebs_snapshot", &["id"]),
                ("aws_ebs_snapshot_copy", &["id"]),
                ("aws_ebs_snapshot_import", &["id"]),
            ],
            false,
            &["ebs_block_device"],
        ),
        ("aws_instance", "ebs_block_device.kms_key_id") => Rule::new(
            "Device encryption key",
            &[("aws_kms_key", &["arn"])],
            false,
            &["ebs_block_device"],
        ),
        ("aws_instance", "root_block_device.kms_key_id") => Rule::new(
            "Root encryption key",
            &[("aws_kms_key", &["arn"])],
            false,
            &[],
        ),
        ("aws_ec2_instance_state", "instance_id") => {
            Rule::new("Managed instance", &[("aws_instance", &["id"])], false, &[])
        }
        _ => return None,
    })
}
pub(crate) fn resource(kind: &str) -> bool {
    matches!(kind, "aws_instance" | "aws_ec2_instance_state")
}
pub(crate) fn indexed(kind: &str, path: &str) -> bool {
    resource(kind) && configured_links::indexed(kind, path, rule)
}
pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    rule(kind, path).map(|r| r.label)
}
pub(super) fn bindings(raw: &Input, r: &AttributeReference) -> bool {
    configured_links::bindings(raw, r, rule)
}
pub(super) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    values: &[(&Value, &Value)],
) {
    configured_links::annotate(nodes, refs, values, rule, qualifies)
}
fn qualifies(e: &Evidence<'_>, r: &AttributeReference, _source: usize, path: &str) -> bool {
    if !e.exclusive_leaf(
        r.target,
        path,
        &[
            ("id", "name"),
            ("placement_group", "placement_group_id"),
            (
                "capacity_reservation_id",
                "capacity_reservation_resource_group_arn",
            ),
            ("host_id", "host_resource_group_arn"),
        ],
    ) {
        return false;
    }
    match r.attribute.as_str() {
        "primary_network_interface.network_interface_id" => e.absent(r.target, "network_interface"),
        "network_interface.network_interface_id" => e.absent(r.target, "primary_network_interface"),
        "root_block_device.kms_key_id" | "ebs_block_device.kms_key_id" => {
            path.rsplit_once('.').is_some_and(|(block, _)| {
                e.value(r.target, &format!("{block}.encrypted"))
                    .and_then(Value::as_bool)
                    == Some(true)
            })
        }
        _ => true,
    }
}
#[cfg(test)]
#[path = "../../../tests/unit/semantic/instance_launch.rs"]
mod tests;
