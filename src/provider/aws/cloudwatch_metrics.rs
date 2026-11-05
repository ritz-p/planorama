use super::service_evidence::{eligible_with_global_role, known};
use crate::model::{AttributeReference, TerraformEntity};
use serde_json::Value;

pub(super) fn label(kind: &str, field: &str) -> Option<&'static str> {
    match (kind, field) {
        ("aws_cloudwatch_metric_stream", "firehose_arn") => {
            Some("Configured metric delivery stream")
        }
        ("aws_cloudwatch_metric_stream", "role_arn") => Some("Metric stream role reference"),
        ("aws_cloudwatch_contributor_managed_insight_rule", "resource_arn") => {
            Some("Configured Contributor Insights resource")
        }
        ("aws_cloudwatch_resource_metrics_configuration", "resource_arn") => {
            Some("Configured detailed metrics resource")
        }
        _ => None,
    }
}

pub(super) fn bindings(raw: &crate::semantic::Input, r: &AttributeReference) -> bool {
    let node = &raw.nodes[r.target];
    r.sources.iter().all(|&source| {
        let endpoint = &raw.nodes[source];
        super::eventbridge::bindings(
            node.provider_configuration.as_ref(),
            endpoint.provider_configuration.as_ref(),
            node.entity.scope.as_ref().and_then(|s| s.region.as_ref()),
            endpoint
                .entity
                .scope
                .as_ref()
                .and_then(|s| s.region.as_ref()),
            endpoint.resource_type == "aws_iam_role",
        )
    })
}

fn template(value: Option<&str>) -> bool {
    matches!(
        value,
        Some(
            "VpcEndpointService-ActiveConnectionsByEndpointId-v1"
                | "VpcEndpointService-BytesByEndpointId-v1"
                | "VpcEndpointService-NewConnectionsByEndpointId-v1"
                | "VpcEndpointService-RstPacketsByEndpointId-v1"
        )
    )
}

fn arn_matches(node: &TerraformEntity, value: &str, service: &str, prefix: &str) -> bool {
    let parts: Vec<_> = value.splitn(6, ':').collect();
    let ["arn", partition, actual_service, region, account, resource] = parts.as_slice() else {
        return false;
    };
    !partition.is_empty()
        && *actual_service == service
        && account.len() == 12
        && account.bytes().all(|b| b.is_ascii_digit())
        && resource.strip_prefix(prefix).is_some_and(|s| !s.is_empty())
        && if service == "iam" {
            region.is_empty()
        } else {
            super::scope::scope(node)
                .and_then(|s| s.region)
                .is_some_and(|r| r.0 == *region)
        }
        && node
            .provider_configuration
            .as_ref()
            .and_then(|c| c.account.as_ref())
            .is_none_or(|a| a.as_str() == *account)
}

pub(crate) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    evidence: &[(&Value, &Value)],
) {
    for r in refs {
        let kind = nodes[r.target].resource_type.as_str();
        if label(kind, &r.attribute).is_none() {
            continue;
        }
        r.service_reference_verified = false;
        if !eligible_with_global_role(nodes, r, true)
            || r.sources.len() != 1
            || r.source_attributes.len() != 1
            || !r.source_attributes.contains("arn")
        {
            continue;
        }
        let (target_kind, service, prefix) = match (kind, r.attribute.as_str()) {
            ("aws_cloudwatch_metric_stream", "firehose_arn") => (
                "aws_kinesis_firehose_delivery_stream",
                "firehose",
                "deliverystream/",
            ),
            ("aws_cloudwatch_metric_stream", "role_arn") => ("aws_iam_role", "iam", "role/"),
            ("aws_cloudwatch_resource_metrics_configuration", "resource_arn") => (
                "aws_elasticache_replication_group",
                "elasticache",
                "replicationgroup:",
            ),
            ("aws_cloudwatch_contributor_managed_insight_rule", "resource_arn")
                if template(known(evidence, r.target, "template_name")) =>
            {
                ("aws_vpc_endpoint_service", "ec2", "vpc-endpoint-service/")
            }
            _ => continue,
        };
        let source = r.sources[0];
        if nodes[source].resource_type != target_kind {
            continue;
        }
        r.service_reference_verified = known(evidence, source, "arn").is_some_and(|id| {
            known(evidence, r.target, &r.attribute) == Some(id)
                && arn_matches(&nodes[source], id, service, prefix)
        });
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/semantic/cloudwatch_metrics.rs"]
mod tests;
