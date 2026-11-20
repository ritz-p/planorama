use super::configured_links::{self, Evidence, Rule};
use crate::model::{AttributeReference, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;

fn rule(kind: &str, path: &str) -> Option<Rule> {
    Some(match (kind, path) {
        ("aws_config_config_rule", "source.source_identifier") => Rule::new(
            "Config evaluator",
            &[("aws_lambda_function", &["arn"])],
            false,
            &[],
        ),
        ("aws_config_organization_custom_rule", "lambda_function_arn") => Rule::new(
            "Organization rule evaluator",
            &[("aws_lambda_function", &["arn"])],
            false,
            &[],
        ),
        ("aws_config_remediation_configuration", "config_rule_name") => Rule::new(
            "Remediated Config rule",
            &[("aws_config_config_rule", &["name"])],
            false,
            &[],
        ),
        ("aws_config_remediation_configuration", "target_id") => Rule::new(
            "Remediation document",
            &[("aws_ssm_document", &["name"])],
            false,
            &[],
        ),
        _ => return None,
    })
}
pub(crate) fn resource(kind: &str) -> bool {
    matches!(
        kind,
        "aws_config_config_rule"
            | "aws_config_organization_custom_rule"
            | "aws_config_remediation_configuration"
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
    let _ = (source, path);
    match r.attribute.as_str() {
        "source.source_identifier" => e.text(r.target, "source.0.owner") == Some("CUSTOM_LAMBDA"),
        "target_id" => e.text(r.target, "target_type") == Some("SSM_DOCUMENT"),
        _ => true,
    }
}
#[cfg(test)]
#[path = "../../../tests/unit/semantic/config_rules.rs"]
mod tests;
