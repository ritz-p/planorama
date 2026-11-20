use super::configured_links::{self, Evidence, Rule};
use crate::model::{AttributeReference, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;

fn rule(kind: &str, path: &str) -> Option<Rule> {
    Some(match (kind, path) {
        (
            "aws_securityhub_automation_rule_v2",
            "action.external_integration_configuration.connector_arn",
        ) => Rule::new(
            "Security Hub connector",
            &[("aws_securityhub_connector_v2", &["arn"])],
            false,
            &["action", "action.external_integration_configuration"],
        ),
        ("aws_securityhub_connector_v2", "kms_key_arn") => Rule::new(
            "Connector encryption key",
            &[("aws_kms_key", &["arn"])],
            false,
            &[],
        ),
        ("aws_securityhub_connector_v2", "connector_provider.service_now.secret_arn") => Rule::new(
            "ServiceNow secret",
            &[("aws_secretsmanager_secret", &["arn"])],
            false,
            &["connector_provider", "connector_provider.service_now"],
        ),
        _ => return None,
    })
}
pub(crate) fn resource(kind: &str) -> bool {
    matches!(
        kind,
        "aws_securityhub_automation_rule_v2" | "aws_securityhub_connector_v2"
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
    let _ = source;
    r.attribute != "action.external_integration_configuration.connector_arn"
        || path.split('.').nth(1).is_some_and(|index| {
            e.text(r.target, &format!("action.{index}.type")) == Some("EXTERNAL_INTEGRATION")
        })
}
#[cfg(test)]
#[path = "../../../tests/unit/semantic/securityhub_integrations.rs"]
mod tests;
