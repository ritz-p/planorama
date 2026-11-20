use super::configured_links::{self, Evidence, Rule};
use crate::model::{AttributeReference, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;

fn rule(kind: &str, path: &str) -> Option<Rule> {
    Some(match (kind, path) {
        ("aws_wafv2_web_acl_logging_configuration", "resource_arn") => Rule::new(
            "Logged web ACL",
            &[("aws_wafv2_web_acl", &["arn"])],
            false,
            &[],
        )
        .in_control_plane_account(),
        ("aws_wafv2_web_acl_logging_configuration", "log_destination_configs") => Rule::new(
            "Web ACL log destination",
            &[
                ("aws_kinesis_firehose_delivery_stream", &["arn"]),
                ("aws_cloudwatch_log_group", &["arn"]),
                ("aws_s3_bucket", &["arn"]),
            ],
            true,
            &[],
        )
        .decoded(log_destination)
        .in_control_plane_account(),
        _ => return None,
    })
}
pub(crate) fn resource(kind: &str) -> bool {
    matches!(kind, "aws_wafv2_web_acl_logging_configuration")
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
fn qualifies(e: &Evidence<'_>, r: &AttributeReference, source: usize, _path: &str) -> bool {
    let Some(scope) = e.nodes[r.target].waf_scope else {
        return false;
    };
    if r.attribute == "resource_arn" {
        return e.nodes[source].waf_scope == Some(scope);
    }
    let Some(arn) = e.text(source, "arn") else {
        return false;
    };
    let fields: Vec<_> = arn.splitn(6, ':').collect();
    let ["arn", partition, service, region, owner, name] = fields.as_slice() else {
        return false;
    };
    if partition.is_empty() {
        return false;
    }
    let prefix = match (e.nodes[source].resource_type.as_str(), *service) {
        ("aws_s3_bucket", "s3") => {
            return region.is_empty()
                && owner.is_empty()
                && name.starts_with("aws-waf-logs-")
                && !name.contains('/');
        }
        ("aws_cloudwatch_log_group", "logs") => "log-group:aws-waf-logs-",
        ("aws_kinesis_firehose_delivery_stream", "firehose")
            if e.absent(source, "kinesis_source_configuration")
                && e.absent(source, "msk_source_configuration") =>
        {
            "deliverystream/aws-waf-logs-"
        }
        _ => return false,
    };
    let expected = match scope {
        crate::model::WafScope::CloudFront => Some("us-east-1"),
        crate::model::WafScope::Regional => {
            super::waf::effective_region(&e.nodes[r.target]).map(|v| v.0.as_str())
        }
    };
    name.starts_with(prefix)
        && expected == Some(*region)
        && super::waf::effective_region(&e.nodes[source]).is_some_and(|r| r.0 == *region)
        && crate::model::AwsAccountScope::new(owner).is_some_and(|a| {
            e.nodes[source]
                .provider_configuration
                .as_ref()
                .and_then(|c| c.account.as_ref())
                .is_none_or(|bound| bound == &a)
        })
}
#[cfg(test)]
#[path = "../../../tests/unit/semantic/waf_destinations.rs"]
mod tests;

fn log_destination<'a>(_: &str, value: &'a Value) -> Option<std::collections::BTreeSet<&'a str>> {
    let [entry] = value.as_array()?.as_slice() else {
        return None;
    };
    let arn = entry.as_str()?;
    let base = if arn.split(':').nth(2) == Some("s3") {
        arn.split('/').next()?
    } else {
        arn
    };
    Some([base].into())
}
