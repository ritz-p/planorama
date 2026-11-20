use super::{Evidence, Rule};
use crate::model::AttributeReference;

pub(super) fn resource(kind: &str) -> bool {
    matches!(
        kind,
        "aws_codeconnections_host"
            | "aws_codestarconnections_host"
            | "aws_codeconnections_connection"
            | "aws_codestarconnections_connection"
    )
}

pub(super) fn rule(kind: &str, path: &str) -> Option<Rule> {
    Some(match (kind, path) {
        ("aws_codeconnections_connection" | "aws_codestarconnections_connection", "host_arn") => {
            Rule::new(
                "Connection host",
                &[
                    ("aws_codeconnections_host", &["arn"]),
                    ("aws_codestarconnections_host", &["arn"]),
                ],
                false,
                &[],
            )
        }
        (
            "aws_codeconnections_host" | "aws_codestarconnections_host",
            "vpc_configuration.vpc_id",
        ) => Rule::new("Connection VPC", &[("aws_vpc", &["id"])], false, &[]),
        (
            "aws_codeconnections_host" | "aws_codestarconnections_host",
            "vpc_configuration.subnet_ids",
        ) => Rule::new("Connection subnets", &[("aws_subnet", &["id"])], true, &[]),
        (
            "aws_codeconnections_host" | "aws_codestarconnections_host",
            "vpc_configuration.security_group_ids",
        ) => Rule::new(
            "Connection security groups",
            &[("aws_security_group", &["id"])],
            true,
            &[],
        ),
        _ => return None,
    })
}

pub(super) fn qualifies(e: &Evidence<'_>, r: &AttributeReference, source: usize, _: &str) -> bool {
    use crate::model::{Action, EntityMode};
    match r.attribute.as_str() {
        "host_arn" => {
            !e.has_argument(r.target, "provider_type")
                && e.text(source, "arn").is_some_and(|arn| {
                    e.nodes
                        .iter()
                        .enumerate()
                        .filter(|(i, node)| {
                            node.provider.is_aws()
                                && node.mode == EntityMode::Managed
                                && node.action != Action::Delete
                                && node.deposed_key.is_none()
                                && matches!(
                                    node.resource_type.as_str(),
                                    "aws_codeconnections_host" | "aws_codestarconnections_host"
                                )
                                && e.text(*i, "arn") == Some(arn)
                        })
                        .count()
                        == 1
                })
        }
        "vpc_configuration.subnet_ids" | "vpc_configuration.security_group_ids" => {
            let vpc =
                e.configured_endpoint(r.target, "vpc_configuration.0.vpc_id", "aws_vpc", &["id"]);
            vpc.is_some()
                && vpc
                    == crate::provider::aws::lambda_core::parent(e.nodes, e.refs, e.values, source)
        }
        _ => true,
    }
}
