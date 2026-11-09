use crate::model::{AttributeReference, TerraformEntity};
use serde_json::Value;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/traffic_mirror.rs"]
mod tests;

fn rule(kind: &str, path: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match (kind, path) {
        (
            "aws_ec2_traffic_mirror_filter_rule" | "aws_ec2_traffic_mirror_session",
            "traffic_mirror_filter_id",
        ) => Some((
            "aws_ec2_traffic_mirror_filter",
            "id",
            "Traffic mirror filter",
        )),
        ("aws_ec2_traffic_mirror_session", "traffic_mirror_target_id") => Some((
            "aws_ec2_traffic_mirror_target",
            "id",
            "Traffic mirror target",
        )),
        (
            "aws_ec2_traffic_mirror_session" | "aws_ec2_traffic_mirror_target",
            "network_interface_id",
        ) => Some(("aws_network_interface", "id", "Mirror network interface")),
        ("aws_ec2_traffic_mirror_target", "network_load_balancer_arn") => {
            Some(("aws_lb", "arn", "Mirror network load balancer"))
        }
        ("aws_ec2_traffic_mirror_target", "gateway_load_balancer_endpoint_id") => {
            Some(("aws_vpc_endpoint", "id", "Mirror gateway endpoint"))
        }
        _ => None,
    }
}

pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    rule(kind, path).map(|r| r.2)
}

pub(crate) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    evidence: &[(&Value, &Value)],
) {
    super::service_evidence::annotate_scalar(
        nodes,
        refs,
        evidence,
        label,
        |kind, path, source, field| {
            rule(kind, path).is_some_and(|r| {
                (r.0 == source || (r.0 == "aws_lb" && source == "aws_alb")) && r.1 == field
            })
        },
    );
    for r in refs {
        let kind = nodes[r.target].resource_type.as_str();
        if label(kind, &r.attribute).is_none() || !r.service_reference_verified {
            continue;
        }
        let known = |index, field| super::service_evidence::known(evidence, index, field);
        let (value, mask) = evidence[r.target];
        if kind == "aws_ec2_traffic_mirror_filter_rule" {
            r.service_reference_verified &=
                matches!(
                    known(r.target, "traffic_direction"),
                    Some("ingress" | "egress")
                ) && matches!(known(r.target, "rule_action"), Some("accept" | "reject"))
                    && value["rule_number"].as_i64().is_some()
                    && matches!(&mask["rule_number"], Value::Null | Value::Bool(false));
        }
        if kind == "aws_ec2_traffic_mirror_target" {
            r.service_reference_verified &= [
                "network_interface_id",
                "network_load_balancer_arn",
                "gateway_load_balancer_endpoint_id",
            ]
            .into_iter()
            .filter(|field| *field != r.attribute)
            .all(|field| {
                (value[field].is_null() || value[field].as_str() == Some(""))
                    && matches!(&mask[field], Value::Null | Value::Bool(false))
            });
            r.service_reference_verified &= match r.attribute.as_str() {
                "network_load_balancer_arn" => {
                    known(r.sources[0], "load_balancer_type") == Some("network")
                }
                "gateway_load_balancer_endpoint_id" => {
                    known(r.sources[0], "vpc_endpoint_type") == Some("GatewayLoadBalancer")
                }
                _ => true,
            };
        }
    }
}
