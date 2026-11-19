use super::configured_links::{self, Evidence, Rule};
use crate::model::{AttributeReference, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;

fn rule(kind: &str, path: &str) -> Option<Rule> {
    Some(match (kind, path) {
        ("aws_launch_template", "image_id") => Rule::new(
            "Template image",
            &[
                ("aws_ami", &["id"]),
                ("aws_ami_copy", &["id"]),
                ("aws_ami_from_instance", &["id"]),
            ],
            false,
            &[],
        ),
        ("aws_launch_template", "key_name") => Rule::new(
            "Template key pair",
            &[("aws_key_pair", &["key_name"])],
            false,
            &[],
        ),
        ("aws_launch_template", "iam_instance_profile.name") => Rule::new(
            "Template instance profile",
            &[("aws_iam_instance_profile", &["name"])],
            false,
            &[],
        ),
        ("aws_launch_template", "iam_instance_profile.arn") => Rule::new(
            "Template instance profile",
            &[("aws_iam_instance_profile", &["arn"])],
            false,
            &[],
        ),
        ("aws_launch_template", "placement.group_id") => Rule::new(
            "Placement group",
            &[("aws_placement_group", &["placement_group_id"])],
            false,
            &[],
        ),
        ("aws_launch_template", "placement.group_name") => Rule::new(
            "Placement group",
            &[("aws_placement_group", &["name"])],
            false,
            &[],
        ),
        ("aws_launch_template", "placement.host_id") => {
            Rule::new("Dedicated host", &[("aws_ec2_host", &["id"])], false, &[])
        }
        (
            "aws_launch_template",
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
        ("aws_launch_template", "network_interfaces.subnet_id") => Rule::new(
            "Launch subnet",
            &[("aws_subnet", &["id"])],
            false,
            &["network_interfaces"],
        ),
        ("aws_launch_template", "network_interfaces.network_interface_id") => Rule::new(
            "Network interface",
            &[("aws_network_interface", &["id"])],
            false,
            &["network_interfaces"],
        ),
        ("aws_launch_template", "network_interfaces.security_groups") => Rule::new(
            "Interface security groups",
            &[("aws_security_group", &["id"])],
            true,
            &["network_interfaces"],
        ),
        ("aws_launch_template", "vpc_security_group_ids") => Rule::new(
            "Launch security groups",
            &[("aws_security_group", &["id"])],
            true,
            &[],
        ),
        ("aws_launch_template", "secondary_interfaces.secondary_subnet_id") => Rule::new(
            "Secondary subnet",
            &[("aws_ec2_secondary_subnet", &["secondary_subnet_id"])],
            false,
            &["secondary_interfaces"],
        ),
        ("aws_launch_template", "block_device_mappings.ebs.snapshot_id") => Rule::new(
            "Device snapshot",
            &[
                ("aws_ebs_snapshot", &["id"]),
                ("aws_ebs_snapshot_copy", &["id"]),
                ("aws_ebs_snapshot_import", &["id"]),
            ],
            false,
            &["block_device_mappings"],
        ),
        ("aws_launch_template", "block_device_mappings.ebs.kms_key_id") => Rule::new(
            "Device encryption key",
            &[
                ("aws_kms_key", &["arn", "key_id"]),
                ("aws_kms_alias", &["arn", "name"]),
            ],
            false,
            &["block_device_mappings"],
        ),
        ("aws_launch_template", "license_specification.license_configuration_arn") => Rule::new(
            "License configuration",
            &[("aws_licensemanager_license_configuration", &["arn"])],
            false,
            &["license_specification"],
        ),
        _ => return None,
    })
}
pub(crate) fn resource(kind: &str) -> bool {
    matches!(kind, "aws_launch_template")
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
            ("group_id", "group_name"),
            ("arn", "name"),
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
        "image_id" => e
            .text(r.target, path)
            .is_some_and(|v| !v.starts_with("resolve:ssm:")),
        "block_device_mappings.ebs.kms_key_id" => {
            path.rsplit_once('.').is_some_and(|(block, _)| {
                e.text(r.target, &format!("{block}.encrypted")) == Some("true")
            })
        }
        "network_interfaces.security_groups" => {
            e.absent(r.target, "vpc_security_group_ids")
                && e.absent(r.target, "security_group_names")
        }
        "vpc_security_group_ids" => {
            e.absent(r.target, "security_group_names")
                && (e.absent(r.target, "network_interfaces") || {
                    let (value, mask) = e.values[r.target];
                    super::nested_references::block(value, mask, "network_interfaces", usize::MAX)
                        && value["network_interfaces"].as_array().is_some_and(|items| {
                            (0..items.len()).all(|i| {
                                e.absent(
                                    r.target,
                                    &format!("network_interfaces.{i}.security_groups"),
                                )
                            })
                        })
                })
        }
        _ => true,
    }
}
#[cfg(test)]
#[path = "../../../tests/unit/semantic/template_launch.rs"]
mod tests;
