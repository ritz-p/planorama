use super::configured_links::{self, Evidence, Rule};
use crate::model::{AttributeReference, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;

fn rule(kind: &str, path: &str) -> Option<Rule> {
    Some(match (kind, path) {
        (
            "aws_ec2_fleet",
            "launch_template_config.launch_template_specification.launch_template_id",
        ) => Rule::new(
            "Fleet launch template",
            &[("aws_launch_template", &["id"])],
            false,
            &["launch_template_config"],
        ),
        (
            "aws_ec2_fleet",
            "launch_template_config.launch_template_specification.launch_template_name",
        ) => Rule::new(
            "Fleet launch template",
            &[("aws_launch_template", &["name"])],
            false,
            &["launch_template_config"],
        ),
        ("aws_ec2_fleet", "launch_template_config.override.subnet_id") => Rule::new(
            "Launch subnet",
            &[("aws_subnet", &["id"])],
            false,
            &["launch_template_config", "launch_template_config.override"],
        ),
        ("aws_spot_fleet_request", "launch_template_config.launch_template_specification.id") => {
            Rule::new(
                "Fleet launch template",
                &[("aws_launch_template", &["id"])],
                false,
                &["launch_template_config"],
            )
        }
        ("aws_spot_fleet_request", "launch_template_config.launch_template_specification.name") => {
            Rule::new(
                "Fleet launch template",
                &[("aws_launch_template", &["name"])],
                false,
                &["launch_template_config"],
            )
        }
        ("aws_spot_fleet_request", "launch_template_config.overrides.subnet_id") => Rule::new(
            "Launch subnet",
            &[("aws_subnet", &["id"])],
            false,
            &["launch_template_config", "launch_template_config.overrides"],
        ),
        ("aws_spot_fleet_request", "iam_fleet_role") => {
            Rule::new("Spot Fleet role", &[("aws_iam_role", &["arn"])], false, &[])
        }
        ("aws_spot_fleet_request", "load_balancers") => Rule::new(
            "Classic load balancers",
            &[("aws_elb", &["name"])],
            true,
            &[],
        ),
        ("aws_spot_fleet_request", "target_group_arns") => Rule::new(
            "Load balancer target groups",
            &[("aws_lb_target_group", &["arn"])],
            true,
            &[],
        ),
        ("aws_spot_fleet_request", "launch_specification.ami") => Rule::new(
            "Launch image",
            &[
                ("aws_ami", &["id"]),
                ("aws_ami_copy", &["id"]),
                ("aws_ami_from_instance", &["id"]),
            ],
            false,
            &["launch_specification"],
        ),
        ("aws_spot_fleet_request", "launch_specification.subnet_id") => Rule::new(
            "Launch subnet",
            &[("aws_subnet", &["id"])],
            false,
            &["launch_specification"],
        ),
        ("aws_spot_fleet_request", "launch_specification.vpc_security_group_ids") => Rule::new(
            "Launch security groups",
            &[("aws_security_group", &["id"])],
            true,
            &["launch_specification"],
        ),
        ("aws_spot_fleet_request", "launch_specification.key_name") => Rule::new(
            "Launch key pair",
            &[("aws_key_pair", &["key_name"])],
            false,
            &["launch_specification"],
        ),
        ("aws_spot_fleet_request", "launch_specification.iam_instance_profile_arn") => Rule::new(
            "Launch instance profile",
            &[("aws_iam_instance_profile", &["arn"])],
            false,
            &["launch_specification"],
        ),
        ("aws_spot_fleet_request", "launch_specification.iam_instance_profile") => Rule::new(
            "Launch instance profile",
            &[("aws_iam_instance_profile", &["name"])],
            false,
            &["launch_specification"],
        ),
        ("aws_spot_fleet_request", "launch_specification.placement_group") => Rule::new(
            "Launch placement group",
            &[("aws_placement_group", &["name"])],
            false,
            &["launch_specification"],
        ),
        ("aws_spot_instance_request", "ami") => Rule::new(
            "Requested image",
            &[
                ("aws_ami", &["id"]),
                ("aws_ami_copy", &["id"]),
                ("aws_ami_from_instance", &["id"]),
            ],
            false,
            &[],
        ),
        ("aws_spot_instance_request", "subnet_id") => {
            Rule::new("Requested subnet", &[("aws_subnet", &["id"])], false, &[])
        }
        ("aws_spot_instance_request", "key_name") => Rule::new(
            "Requested key pair",
            &[("aws_key_pair", &["key_name"])],
            false,
            &[],
        ),
        ("aws_spot_instance_request", "iam_instance_profile") => Rule::new(
            "Requested instance profile",
            &[("aws_iam_instance_profile", &["name"])],
            false,
            &[],
        ),
        ("aws_spot_instance_request", "vpc_security_group_ids") => Rule::new(
            "Requested security groups",
            &[("aws_security_group", &["id"])],
            true,
            &[],
        ),
        ("aws_spot_instance_request", "launch_template.id") => Rule::new(
            "Requested template",
            &[("aws_launch_template", &["id"])],
            false,
            &[],
        ),
        ("aws_spot_instance_request", "launch_template.name") => Rule::new(
            "Requested template",
            &[("aws_launch_template", &["name"])],
            false,
            &[],
        ),
        ("aws_spot_datafeed_subscription", "bucket") => Rule::new(
            "Spot datafeed bucket",
            &[("aws_s3_bucket", &["bucket", "id"])],
            false,
            &[],
        ),
        _ => return None,
    })
}
pub(crate) fn resource(kind: &str) -> bool {
    matches!(
        kind,
        "aws_ec2_fleet"
            | "aws_spot_fleet_request"
            | "aws_spot_instance_request"
            | "aws_spot_datafeed_subscription"
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
}
pub(super) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    values: &[(&Value, &Value)],
) {
    configured_links::annotate(nodes, refs, values, rule, qualifies)
}
fn qualifies(e: &Evidence<'_>, r: &AttributeReference, _source: usize, path: &str) -> bool {
    if e.nodes[r.target].resource_type == "aws_spot_fleet_request" {
        if path.starts_with("launch_specification.")
            && !e.absent(r.target, "launch_template_config")
            || path.starts_with("launch_template_config.")
                && !e.absent(r.target, "launch_specification")
        {
            return false;
        }
    }
    e.exclusive_leaf(
        r.target,
        path,
        &[
            ("launch_template_id", "launch_template_name"),
            ("id", "name"),
            ("iam_instance_profile", "iam_instance_profile_arn"),
        ],
    )
}
#[cfg(test)]
#[path = "../../../tests/unit/semantic/ec2_fleets.rs"]
mod tests;
