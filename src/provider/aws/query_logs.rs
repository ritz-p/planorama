use crate::model::{AttributeReference, TerraformEntity};
use serde_json::Value;

const CONFIG: &str = "aws_route53_resolver_query_log_config";
const ASSOCIATION: &str = "aws_route53_resolver_query_log_config_association";
const PUBLIC: &str = "aws_route53_query_log";

pub(super) fn label(kind: &str, field: &str) -> Option<&'static str> {
    match (kind, field) {
        (PUBLIC, "zone_id") => Some("Query logging hosted zone"),
        (PUBLIC, "cloudwatch_log_group_arn") | (CONFIG, "destination_arn") => {
            Some("Query log destination")
        }
        (ASSOCIATION, "resolver_query_log_config_id") => Some("Query log configuration"),
        (ASSOCIATION, "resource_id") => Some("Query logging VPC"),
        _ => None,
    }
}

pub(crate) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    evidence: &[(&Value, &Value)],
) {
    for r in refs {
        let node = &nodes[r.target];
        if label(&node.resource_type, &r.attribute).is_none() {
            continue;
        }
        r.service_reference_verified = false;
        r.service_reference_owner_account = None;
        r.service_reference_shared = false;
        let [source] = r.sources.as_slice() else {
            continue;
        };
        let endpoint = &nodes[*source];
        let fields: &[&str] = match (
            node.resource_type.as_str(),
            r.attribute.as_str(),
            endpoint.resource_type.as_str(),
        ) {
            (PUBLIC, "zone_id", "aws_route53_zone") => &["id", "zone_id"],
            (PUBLIC, "cloudwatch_log_group_arn", "aws_cloudwatch_log_group") => &["arn"],
            (
                CONFIG,
                "destination_arn",
                "aws_s3_bucket"
                | "aws_cloudwatch_log_group"
                | "aws_kinesis_firehose_delivery_stream",
            ) => &["arn"],
            (ASSOCIATION, "resolver_query_log_config_id", CONFIG)
            | (ASSOCIATION, "resource_id", "aws_vpc") => &["id"],
            _ => continue,
        };
        if super::route53::endpoint(nodes, r, &endpoint.resource_type, fields).is_none()
            || !super::route53::configured(node, endpoint)
        {
            continue;
        }
        let actual = super::service_evidence::known(evidence, r.target, &r.attribute);
        if actual.is_none()
            || actual
                != super::service_evidence::known(
                    evidence,
                    *source,
                    r.source_attributes.first().unwrap(),
                )
        {
            continue;
        }
        let left = super::scope::scope(node).and_then(|s| s.region);
        let right = super::scope::scope(endpoint).and_then(|s| s.region);
        if node.resource_type == PUBLIC {
            if r.attribute == "cloudwatch_log_group_arn"
                && right.as_ref().is_none_or(|r| r.0 != "us-east-1")
            {
                continue;
            }
        } else if left.is_none() || (endpoint.resource_type != "aws_s3_bucket" && left != right) {
            continue;
        }
        let config = if node.resource_type == CONFIG {
            Some(r.target)
        } else if endpoint.resource_type == CONFIG {
            Some(*source)
        } else {
            None
        };
        if let Some(index) = config {
            let (v, m) = evidence[index];
            let share = super::service_evidence::known(evidence, index, "share_status");
            r.service_reference_shared = share == Some("SHARED_BY_ME");
            let accounts_differ = node
                .provider_configuration
                .as_ref()
                .and_then(|c| c.account.as_ref())
                .zip(
                    endpoint
                        .provider_configuration
                        .as_ref()
                        .and_then(|c| c.account.as_ref()),
                )
                .is_some_and(|(a, b)| a != b);
            if accounts_differ && share != Some("SHARED_BY_ME") {
                continue;
            }
            if !super::vpc_gateways::absent(v, m, "share_status")
                && !matches!(
                    share,
                    Some("NOT_SHARED" | "SHARED_BY_ME" | "SHARED_WITH_ME")
                )
            {
                continue;
            }
            if !super::vpc_gateways::absent(v, m, "owner_id")
                || matches!(share, Some("SHARED_BY_ME" | "SHARED_WITH_ME"))
            {
                let Some(owner) = super::service_evidence::known(evidence, index, "owner_id")
                    .and_then(crate::model::AwsAccountScope::new)
                else {
                    continue;
                };
                r.service_reference_owner_account = Some(owner);
            }
        }
        r.service_reference_verified = true;
    }
}

pub(super) fn bindings(raw: &crate::semantic::Input, r: &AttributeReference) -> bool {
    let node = &raw.nodes[r.target];
    r.sources.iter().all(|&s| {
        let endpoint = &raw.nodes[s];
        let (Some(left), Some(right)) = (
            &node.provider_configuration,
            &endpoint.provider_configuration,
        ) else {
            return false;
        };
        if left.account_conflict || right.account_conflict {
            return false;
        }
        let config = if node.resource_type == CONFIG {
            Some(left)
        } else if endpoint.resource_type == CONFIG {
            Some(right)
        } else {
            None
        };
        if r.service_reference_owner_account
            .as_ref()
            .is_some_and(|owner| config.and_then(|c| c.account.as_ref()) != Some(owner))
        {
            return false;
        }
        match (&left.account, &right.account) {
            (Some(a), Some(b)) if a == b => true,
            (None, None) => left == right,
            (Some(_), Some(_)) => {
                node.resource_type == ASSOCIATION
                    && endpoint.resource_type == CONFIG
                    && r.service_reference_owner_account.is_some()
                    && r.service_reference_shared
            }
            _ => false,
        }
    })
}

#[cfg(test)]
#[path = "../../../tests/unit/semantic/query_logs.rs"]
mod tests;
