use super::configured_links::{self, Evidence, Rule};
use crate::model::{AttributeReference, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;

fn rule(kind: &str, path: &str) -> Option<Rule> {
    Some(match (kind, path) {
        ("aws_route53recoveryreadiness_readiness_check", "resource_set_name") => Rule::new(
            "Checked resource set",
            &[(
                "aws_route53recoveryreadiness_resource_set",
                &["resource_set_name"],
            )],
            false,
            &[],
        )
        .in_control_plane_account(),
        (
            "aws_route53recoveryreadiness_cell" | "aws_route53recoveryreadiness_recovery_group",
            "cells",
        ) => Rule::new(
            "Readiness cells",
            &[("aws_route53recoveryreadiness_cell", &["arn"])],
            true,
            &[],
        )
        .in_control_plane_account(),
        ("aws_route53recoveryreadiness_resource_set", "resources.resource_arn") => Rule::new(
            "Readiness resource",
            &[
                ("aws_cloudwatch_metric_alarm", &["arn"]),
                ("aws_cloudwatch_composite_alarm", &["arn"]),
                ("aws_api_gateway_stage", &["arn"]),
                ("aws_apigatewayv2_stage", &["arn"]),
                ("aws_dynamodb_table", &["arn"]),
                ("aws_customer_gateway", &["arn"]),
                ("aws_ebs_volume", &["arn"]),
                ("aws_elb", &["arn"]),
                ("aws_lb", &["arn"]),
                ("aws_lambda_function", &["arn"]),
                ("aws_msk_cluster", &["arn"]),
                ("aws_rds_cluster", &["arn"]),
                ("aws_route53_health_check", &["arn"]),
                ("aws_sqs_queue", &["arn"]),
                ("aws_sns_topic", &["arn"]),
                ("aws_sns_topic_subscription", &["arn"]),
                ("aws_vpc", &["arn"]),
                ("aws_vpn_connection", &["arn"]),
                ("aws_vpn_gateway", &["arn"]),
            ],
            false,
            &["resources"],
        )
        .in_control_plane_account(),
        ("aws_route53recoveryreadiness_resource_set", "resources.readiness_scopes") => Rule::new(
            "Readiness scopes",
            &[
                ("aws_route53recoveryreadiness_cell", &["arn"]),
                ("aws_route53recoveryreadiness_recovery_group", &["arn"]),
            ],
            true,
            &["resources"],
        )
        .in_control_plane_account(),
        (
            "aws_route53recoveryreadiness_resource_set",
            "resources.dns_target_resource.hosted_zone_arn",
        ) => Rule::new(
            "DNS hosted zone",
            &[("aws_route53_zone", &["zone_id", "id"])],
            false,
            &["resources"],
        )
        .decoded(hosted_zone_id)
        .in_control_plane_account(),
        (
            "aws_route53recoveryreadiness_resource_set",
            "resources.dns_target_resource.target_resource.nlb_resource.arn",
        ) => Rule::new(
            "DNS load balancer",
            &[("aws_lb", &["arn"])],
            false,
            &["resources"],
        )
        .in_control_plane_account(),
        _ => return None,
    })
}
pub(crate) fn resource(kind: &str) -> bool {
    matches!(
        kind,
        "aws_route53recoveryreadiness_readiness_check"
            | "aws_route53recoveryreadiness_cell"
            | "aws_route53recoveryreadiness_recovery_group"
            | "aws_route53recoveryreadiness_resource_set"
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
fn qualifies(e: &Evidence<'_>, r: &AttributeReference, source: usize, path: &str) -> bool {
    let kind = super::aliases::entity_type(&e.nodes[source]);
    match r.attribute.as_str() {
        "resources.resource_arn" => {
            if kind == "aws_lb"
                && !matches!(
                    e.text(source, "load_balancer_type"),
                    Some("application" | "network")
                )
            {
                return false;
            }
            e.text(r.target, "resource_set_type").is_some_and(|t| {
                matches!(
                    (kind, t),
                    ("aws_cloudwatch_metric_alarm", "AWS::CloudWatch::Alarm")
                        | ("aws_cloudwatch_composite_alarm", "AWS::CloudWatch::Alarm")
                        | ("aws_api_gateway_stage", "AWS::ApiGateway::Stage")
                        | ("aws_apigatewayv2_stage", "AWS::ApiGatewayV2::Stage")
                        | ("aws_dynamodb_table", "AWS::DynamoDB::Table")
                        | ("aws_customer_gateway", "AWS::EC2::CustomerGateway")
                        | ("aws_ebs_volume", "AWS::EC2::Volume")
                        | ("aws_elb", "AWS::ElasticLoadBalancing::LoadBalancer")
                        | ("aws_lb", "AWS::ElasticLoadBalancingV2::LoadBalancer")
                        | ("aws_lambda_function", "AWS::Lambda::Function")
                        | ("aws_msk_cluster", "AWS::MSK::Cluster")
                        | ("aws_rds_cluster", "AWS::RDS::DBCluster")
                        | ("aws_route53_health_check", "AWS::Route53::HealthCheck")
                        | ("aws_sqs_queue", "AWS::SQS::Queue")
                        | ("aws_sns_topic", "AWS::SNS::Topic")
                        | ("aws_sns_topic_subscription", "AWS::SNS::Subscription")
                        | ("aws_vpc", "AWS::EC2::VPC")
                        | ("aws_vpn_connection", "AWS::EC2::VPNConnection")
                        | ("aws_vpn_gateway", "AWS::EC2::VPNGateway")
                )
            }) && path
                .rsplit_once('.')
                .is_some_and(|(base, _)| e.absent(r.target, &format!("{base}.dns_target_resource")))
        }
        "resources.dns_target_resource.hosted_zone_arn"
        | "resources.dns_target_resource.target_resource.nlb_resource.arn" => {
            let Some((base, _)) = path.split_once(".dns_target_resource.") else {
                return false;
            };
            e.text(r.target, "resource_set_type")
                == Some("AWS::Route53RecoveryReadiness::DNSTargetResource")
                && e.absent(r.target, &format!("{base}.resource_arn"))
                && if kind == "aws_lb" {
                    e.text(source, "load_balancer_type") == Some("network")
                } else {
                    e.text(r.target, path)
                        .and_then(|v| v.split(':').nth(4))
                        .zip(
                            e.nodes[source]
                                .provider_configuration
                                .as_ref()
                                .and_then(|p| p.account.as_ref()),
                        )
                        .is_some_and(|(a, b)| a == b.as_str())
                }
        }
        _ => true,
    }
}
#[cfg(test)]
#[path = "../../../tests/unit/semantic/arc_readiness.rs"]
mod tests;

fn hosted_zone_id<'a>(_: &str, value: &'a Value) -> Option<std::collections::BTreeSet<&'a str>> {
    let fields: Vec<_> = value.as_str()?.splitn(6, ':').collect();
    let ["arn", partition, "route53", "", account, resource] = fields.as_slice() else {
        return None;
    };
    if partition.is_empty() || crate::model::AwsAccountScope::new(account).is_none() {
        return None;
    }
    let id = resource.strip_prefix("hostedzone/")?;
    (!id.is_empty() && !id.contains('/')).then(|| [id].into())
}
