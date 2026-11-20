use super::{Evidence, Rule};
use crate::model::AttributeReference;

pub(super) fn resource(kind: &str) -> bool {
    matches!(
        kind,
        "aws_devicefarm_device_pool"
            | "aws_devicefarm_network_profile"
            | "aws_devicefarm_upload"
            | "aws_devicefarm_test_grid_project"
    )
}

pub(super) fn rule(kind: &str, path: &str) -> Option<Rule> {
    Some(match (kind, path) {
        (
            "aws_devicefarm_device_pool"
            | "aws_devicefarm_network_profile"
            | "aws_devicefarm_upload",
            "project_arn",
        ) => Rule::new(
            "Device Farm project",
            &[("aws_devicefarm_project", &["arn"])],
            false,
            &[],
        ),
        ("aws_devicefarm_test_grid_project", "vpc_config.vpc_id") => {
            Rule::new("TestGrid VPC", &[("aws_vpc", &["id"])], false, &[])
        }
        ("aws_devicefarm_test_grid_project", "vpc_config.subnet_ids") => {
            Rule::new("TestGrid subnets", &[("aws_subnet", &["id"])], true, &[])
        }
        ("aws_devicefarm_test_grid_project", "vpc_config.security_group_ids") => Rule::new(
            "TestGrid security groups",
            &[("aws_security_group", &["id"])],
            true,
            &[],
        ),
        _ => return None,
    })
}

pub(super) fn qualifies(e: &Evidence<'_>, r: &AttributeReference, source: usize, _: &str) -> bool {
    if !matches!(
        r.attribute.as_str(),
        "vpc_config.subnet_ids" | "vpc_config.security_group_ids"
    ) {
        return true;
    }
    let selected = e.configured_endpoint(r.target, "vpc_config.0.vpc_id", "aws_vpc", &["id"]);
    selected.is_some()
        && selected == crate::provider::aws::lambda_core::parent(e.nodes, e.refs, e.values, source)
}
