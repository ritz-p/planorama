use super::configured_links::{self, Evidence, Rule};
use crate::model::{AttributeReference, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;

fn rule(kind: &str, path: &str) -> Option<Rule> {
    Some(match (kind, path) {
        ("aws_waf_rule" | "aws_waf_rate_based_rule", "predicates.data_id") => Rule::new(
            "Rule condition",
            &[
                ("aws_waf_byte_match_set", &["id"]),
                ("aws_waf_geo_match_set", &["id"]),
                ("aws_waf_ipset", &["id"]),
                ("aws_waf_regex_match_set", &["id"]),
                ("aws_waf_size_constraint_set", &["id"]),
                ("aws_waf_sql_injection_match_set", &["id"]),
                ("aws_waf_xss_match_set", &["id"]),
            ],
            false,
            &["predicates"],
        )
        .in_control_plane_account(),
        ("aws_waf_regex_match_set", "regex_match_tuple.regex_pattern_set_id") => Rule::new(
            "Regex pattern set",
            &[("aws_waf_regex_pattern_set", &["id"])],
            false,
            &["regex_match_tuple"],
        )
        .in_control_plane_account(),
        ("aws_waf_rule_group", "activated_rule.rule_id") => Rule::new(
            "Activated rule",
            &[
                ("aws_waf_rule", &["id"]),
                ("aws_waf_rate_based_rule", &["id"]),
            ],
            false,
            &["activated_rule"],
        )
        .in_control_plane_account(),
        ("aws_waf_web_acl", "rules.rule_id") => Rule::new(
            "ACL rule",
            &[
                ("aws_waf_rule", &["id"]),
                ("aws_waf_rate_based_rule", &["id"]),
                ("aws_waf_rule_group", &["id"]),
            ],
            false,
            &["rules"],
        )
        .in_control_plane_account(),
        ("aws_waf_web_acl", "logging_configuration.log_destination") => Rule::new(
            "ACL log stream",
            &[("aws_kinesis_firehose_delivery_stream", &["arn"])],
            false,
            &[],
        )
        .in_control_plane_account(),
        ("aws_wafregional_rule" | "aws_wafregional_rate_based_rule", "predicate.data_id") => {
            Rule::new(
                "Rule condition",
                &[
                    ("aws_wafregional_byte_match_set", &["id"]),
                    ("aws_wafregional_geo_match_set", &["id"]),
                    ("aws_wafregional_ipset", &["id"]),
                    ("aws_wafregional_regex_match_set", &["id"]),
                    ("aws_wafregional_size_constraint_set", &["id"]),
                    ("aws_wafregional_sql_injection_match_set", &["id"]),
                    ("aws_wafregional_xss_match_set", &["id"]),
                ],
                false,
                &["predicate"],
            )
        }
        ("aws_wafregional_regex_match_set", "regex_match_tuple.regex_pattern_set_id") => Rule::new(
            "Regex pattern set",
            &[("aws_wafregional_regex_pattern_set", &["id"])],
            false,
            &["regex_match_tuple"],
        ),
        ("aws_wafregional_rule_group", "activated_rule.rule_id") => Rule::new(
            "Activated rule",
            &[
                ("aws_wafregional_rule", &["id"]),
                ("aws_wafregional_rate_based_rule", &["id"]),
            ],
            false,
            &["activated_rule"],
        ),
        ("aws_wafregional_web_acl", "rule.rule_id") => Rule::new(
            "ACL rule",
            &[
                ("aws_wafregional_rule", &["id"]),
                ("aws_wafregional_rate_based_rule", &["id"]),
                ("aws_wafregional_rule_group", &["id"]),
            ],
            false,
            &["rule"],
        ),
        ("aws_wafregional_web_acl", "logging_configuration.log_destination") => Rule::new(
            "ACL log stream",
            &[("aws_kinesis_firehose_delivery_stream", &["arn"])],
            false,
            &[],
        ),
        ("aws_wafregional_web_acl_association", "web_acl_id") => Rule::new(
            "Regional Classic ACL",
            &[("aws_wafregional_web_acl", &["id"])],
            false,
            &[],
        ),
        ("aws_wafregional_web_acl_association", "resource_arn") => Rule::new(
            "Protected endpoint",
            &[("aws_lb", &["arn"]), ("aws_api_gateway_stage", &["arn"])],
            false,
            &[],
        ),
        _ => return None,
    })
}
pub(crate) fn resource(kind: &str) -> bool {
    matches!(
        kind,
        "aws_waf_rule"
            | "aws_waf_rate_based_rule"
            | "aws_waf_regex_match_set"
            | "aws_waf_rule_group"
            | "aws_waf_web_acl"
            | "aws_wafregional_rule"
            | "aws_wafregional_rate_based_rule"
            | "aws_wafregional_regex_match_set"
            | "aws_wafregional_rule_group"
            | "aws_wafregional_web_acl"
            | "aws_wafregional_web_acl_association"
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
    let source_kind = super::aliases::entity_type(&e.nodes[source]);
    if r.attribute == "resource_arn" {
        return source_kind == "aws_api_gateway_stage"
            || e.text(source, "load_balancer_type") == Some("application");
    }
    if r.attribute == "logging_configuration.log_destination" {
        return e.text(source, "arn").is_some_and(|v| {
            v.splitn(6, ':')
                .last()
                .is_some_and(|name| name.starts_with("deliverystream/aws-waf-logs-"))
        }) && (!e.nodes[r.target].resource_type.starts_with("aws_waf_")
            || super::scope(&e.nodes[source])
                .and_then(|s| s.region)
                .is_some_and(|region| region.0 == "us-east-1"));
    }
    let Some((base, leaf)) = path.rsplit_once('.') else {
        return true;
    };
    let target_type = source_kind
        .strip_prefix("aws_waf_")
        .or_else(|| source_kind.strip_prefix("aws_wafregional_"))
        .unwrap_or("");
    let field = format!("{base}.type");
    match leaf {
        "data_id" => matches!(
            (e.text(r.target, &field), target_type),
            (Some("ByteMatch"), "byte_match_set")
                | (Some("GeoMatch"), "geo_match_set")
                | (Some("IPMatch"), "ipset")
                | (Some("RegexMatch"), "regex_match_set")
                | (Some("SizeConstraint"), "size_constraint_set")
                | (Some("SqlInjectionMatch"), "sql_injection_match_set")
                | (Some("XssMatch"), "xss_match_set")
        ),
        "rule_id" => match e.value(r.target, &field) {
            Some(Value::Null) => target_type == "rule",
            Some(Value::String(t)) => matches!(
                (t.as_str(), target_type),
                ("REGULAR", "rule") | ("RATE_BASED", "rate_based_rule") | ("GROUP", "rule_group")
            ),
            _ => false,
        },
        _ => true,
    }
}
#[cfg(test)]
#[path = "../../../tests/unit/semantic/waf_classic.rs"]
mod tests;
