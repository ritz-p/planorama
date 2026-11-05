use super::service_evidence::known;
use crate::model::{
    Action, AttributeReference, EntityMode, ProviderConfiguration, RegionScope, TerraformEntity,
};
use crate::semantic::Input;
use serde_json::Value;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/flow_logs.rs"]
mod tests;

pub(super) const MONITORED: &[&str] = &[
    "eni_id",
    "subnet_id",
    "vpc_id",
    "regional_nat_gateway_id",
    "transit_gateway_id",
    "transit_gateway_attachment_id",
];

pub(super) fn label(kind: &str, field: &str) -> Option<&'static str> {
    if kind != "aws_flow_log" {
        return None;
    }
    if MONITORED.contains(&field) {
        return Some("Monitored resource");
    }
    match field {
        "log_destination" => Some("Flow log destination"),
        "iam_role_arn" => Some("Flow log delivery role"),
        "deliver_cross_account_role" => Some("Cross-account delivery role"),
        _ => None,
    }
}

fn output(field: &str, kind: &str) -> Option<&'static str> {
    match (field, kind) {
        ("eni_id", "aws_network_interface")
        | ("subnet_id", "aws_subnet")
        | ("vpc_id", "aws_vpc")
        | ("regional_nat_gateway_id", "aws_nat_gateway")
        | ("transit_gateway_id", "aws_ec2_transit_gateway")
        | ("transit_gateway_attachment_id", "aws_ec2_transit_gateway_vpc_attachment") => Some("id"),
        (
            "log_destination",
            "aws_cloudwatch_log_group" | "aws_s3_bucket" | "aws_kinesis_firehose_delivery_stream",
        )
        | ("iam_role_arn" | "deliver_cross_account_role", "aws_iam_role") => Some("arn"),
        _ => None,
    }
}

fn binding(
    left: Option<&ProviderConfiguration>,
    right: Option<&ProviderConfiguration>,
    left_region: Option<&RegionScope>,
    right_region: Option<&RegionScope>,
    kind: &str,
    field: &str,
) -> bool {
    let global = matches!(kind, "aws_iam_role" | "aws_s3_bucket");
    if field == "deliver_cross_account_role"
        || (field == "log_destination"
            && matches!(
                kind,
                "aws_s3_bucket" | "aws_kinesis_firehose_delivery_stream"
            ))
    {
        if let (Some(a), Some(b)) = (left, right) {
            if let (Some(x), Some(y)) = (&a.account, &b.account) {
                if x != y {
                    return !a.account_conflict
                        && !b.account_conflict
                        && left_region.is_some()
                        && (global || left_region == right_region);
                }
            }
        }
        if field == "deliver_cross_account_role" {
            return false;
        }
    }
    super::eventbridge::bindings(left, right, left_region, right_region, global)
}

fn provisional_binding(node: &TerraformEntity, endpoint: &TerraformEntity, field: &str) -> bool {
    let left = node.provider_configuration.as_ref();
    let right = endpoint.provider_configuration.as_ref();
    let left_scope = super::scope::scope(node);
    let right_scope = super::scope::scope(endpoint);
    let region = left_scope.as_ref().and_then(|s| s.region.as_ref());
    let other_region = right_scope.as_ref().and_then(|s| s.region.as_ref());
    binding(
        left,
        right,
        region,
        other_region,
        &endpoint.resource_type,
        field,
    ) || matches!((left, right), (Some(a), Some(b)) if !a.account_conflict && !b.account_conflict
            && a.account.is_none() && b.account.is_none())
        && region.is_some()
        && (matches!(
            endpoint.resource_type.as_str(),
            "aws_iam_role" | "aws_s3_bucket"
        ) || region == other_region)
}

pub(super) fn bindings(raw: &Input, r: &AttributeReference) -> bool {
    let node = &raw.nodes[r.target];
    if r.attribute == "deliver_cross_account_role" {
        let [role] = r.sources.as_slice() else {
            return false;
        };
        let mut destinations = raw
            .attributes
            .iter()
            .filter(|d| d.target == r.target && d.attribute == "log_destination");
        let Some(destination) = destinations.next() else {
            return false;
        };
        let [endpoint] = destination.sources.as_slice() else {
            return false;
        };
        if destinations.next().is_some()
            || !destination.service_reference_verified
            || !bindings(raw, destination)
            || raw.nodes[*endpoint]
                .provider_configuration
                .as_ref()
                .and_then(|c| c.account.as_ref())
                != raw.nodes[*role]
                    .provider_configuration
                    .as_ref()
                    .and_then(|c| c.account.as_ref())
        {
            return false;
        }
    }
    r.sources.iter().all(|&s| {
        let endpoint = &raw.nodes[s];
        binding(
            node.provider_configuration.as_ref(),
            endpoint.provider_configuration.as_ref(),
            node.entity.scope.as_ref().and_then(|s| s.region.as_ref()),
            endpoint
                .entity
                .scope
                .as_ref()
                .and_then(|s| s.region.as_ref()),
            &endpoint.resource_type,
            &r.attribute,
        )
    })
}

fn absent(value: &Value, unknown: &Value, field: &str) -> bool {
    matches!(unknown, Value::Null | Value::Bool(false) | Value::Object(_))
        && matches!(&unknown[field], Value::Null | Value::Bool(false))
        && (value[field].is_null() || value[field].as_str() == Some(""))
}

pub(crate) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    evidence: &[(&Value, &Value)],
) {
    let current = |n: &TerraformEntity| {
        n.provider.is_aws()
            && n.mode == EntityMode::Managed
            && n.action != Action::Delete
            && n.deposed_key.is_none()
    };
    for r in refs.iter_mut() {
        let node = &nodes[r.target];
        if label(&node.resource_type, &r.attribute).is_none() {
            continue;
        }
        r.service_reference_verified = false;
        if !current(node)
            || !r.complete
            || !r.instances_explicit
            || !r.issues.is_empty()
            || r.sources.len() != 1
            || r.source_attributes.len() != 1
        {
            continue;
        }
        let s = r.sources[0];
        let endpoint = &nodes[s];
        let Some(output) = output(&r.attribute, &endpoint.resource_type) else {
            continue;
        };
        if !current(endpoint)
            || !r.source_attributes.contains(output)
            || !provisional_binding(node, endpoint, &r.attribute)
        {
            continue;
        }
        let (value, mask) = evidence[r.target];
        if MONITORED.contains(&r.attribute.as_str()) {
            if !MONITORED
                .iter()
                .all(|f| *f == r.attribute || absent(value, mask, f))
            {
                continue;
            }
            if r.attribute == "regional_nat_gateway_id"
                && known(evidence, s, "availability_mode") != Some("regional")
            {
                continue;
            }
        } else {
            let typ = known(evidence, r.target, "log_destination_type");
            let valid = match r.attribute.as_str() {
                "log_destination" => matches!(
                    (typ, endpoint.resource_type.as_str()),
                    (Some("cloud-watch-logs"), "aws_cloudwatch_log_group")
                        | (Some("s3"), "aws_s3_bucket")
                        | (
                            Some("kinesis-data-firehose"),
                            "aws_kinesis_firehose_delivery_stream"
                        )
                ),
                "iam_role_arn" => matches!(typ, Some("cloud-watch-logs" | "kinesis-data-firehose")),
                "deliver_cross_account_role" => typ == Some("kinesis-data-firehose"),
                _ => false,
            };
            if !valid {
                continue;
            }
        }
        r.service_reference_verified = known(evidence, r.target, &r.attribute)
            .is_some_and(|v| Some(v) == known(evidence, s, output));
    }
    let destinations = refs
        .iter()
        .filter(|r| {
            nodes[r.target].resource_type == "aws_flow_log"
                && r.attribute == "log_destination"
                && r.service_reference_verified
        })
        .map(|r| (r.target, r.sources[0]))
        .collect::<Vec<_>>();
    for r in refs.iter_mut().filter(|r| {
        nodes[r.target].resource_type == "aws_flow_log"
            && r.attribute == "deliver_cross_account_role"
            && r.service_reference_verified
    }) {
        let mut matches = destinations.iter().filter(|(t, _)| *t == r.target);
        r.service_reference_verified = matches.next().is_some_and(|(_, s)| {
            nodes[*s].resource_type == "aws_kinesis_firehose_delivery_stream"
        }) && matches.next().is_none();
    }
}
