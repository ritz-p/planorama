use super::configured_links::{self, Evidence, Rule};
use crate::model::{AttributeReference, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;

fn rule(kind: &str, path: &str) -> Option<Rule> {
    let (label, outputs): (_, &'static [(&'static str, &'static [&'static str])]) =
        match (kind, path) {
            (
                "aws_wafv2_web_acl_rule" | "aws_wafv2_web_acl_rule_group_association",
                "web_acl_arn",
            ) => ("Parent web ACL", &[("aws_wafv2_web_acl", &["arn"])]),
            ("aws_wafv2_web_acl_rule_group_association", "rule_group_reference.arn") => {
                ("Customer rule group", &[("aws_wafv2_rule_group", &["arn"])])
            }
            ("aws_wafv2_web_acl_rule", "statement.rule_group_reference_statement.arn") => (
                "Rule group reference",
                &[("aws_wafv2_rule_group", &["arn"])],
            ),
            _ => {
                let mut statement = match kind {
                    "aws_wafv2_rule_group" | "aws_wafv2_web_acl" => {
                        path.strip_prefix("rule.statement.")?
                    }
                    "aws_wafv2_web_acl_rule" => path.strip_prefix("statement.")?,
                    _ => return None,
                };
                while let Some(rest) = [
                    "and_statement.statement.",
                    "or_statement.statement.",
                    "not_statement.statement.",
                    "rate_based_statement.scope_down_statement.",
                    "managed_rule_group_statement.scope_down_statement.",
                ]
                .iter()
                .find_map(|p| statement.strip_prefix(p))
                {
                    statement = rest;
                }
                match statement {
                    "ip_set_reference_statement.arn" => {
                        ("IP set reference", &[("aws_wafv2_ip_set", &["arn"])])
                    }
                    "regex_pattern_set_reference_statement.arn" => (
                        "Regex pattern set reference",
                        &[("aws_wafv2_regex_pattern_set", &["arn"])],
                    ),
                    _ => return None,
                }
            }
        };
    Some(
        Rule::new(label, outputs, false, &[])
            .statement_blocks()
            .in_control_plane_account(),
    )
}
pub(crate) fn resource(kind: &str) -> bool {
    matches!(
        kind,
        "aws_wafv2_web_acl_rule"
            | "aws_wafv2_web_acl_rule_group_association"
            | "aws_wafv2_web_acl"
            | "aws_wafv2_rule_group"
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
fn qualifies(e: &Evidence<'_>, r: &AttributeReference, source: usize, _path: &str) -> bool {
    let left = &e.nodes[r.target];
    let right = &e.nodes[source];
    left.waf_scope.is_some()
        && left.waf_scope == right.waf_scope
        && (left.waf_scope != Some(crate::model::WafScope::Regional)
            || super::waf::compatible(left, right, true))
        && (r.attribute != "rule_group_reference.arn" || e.absent(r.target, "managed_rule_group"))
}
#[cfg(test)]
#[path = "../../../tests/unit/semantic/waf_statements.rs"]
mod tests;
