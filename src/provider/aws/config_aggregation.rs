use super::configured_links::{self, Evidence, Rule};
use crate::model::{AttributeReference, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;

fn rule(kind: &str, path: &str) -> Option<Rule> {
    Some(match (kind, path) {
        ("aws_config_aggregate_authorization", "account_id") => Rule::new(
            "Authorized aggregator account",
            &[("aws_organizations_account", &["id"])],
            false,
            &[],
        )
        .with_explicit_destination(),
        ("aws_config_configuration_aggregator", "account_aggregation_source.account_ids") => {
            Rule::new(
                "Aggregation source accounts",
                &[("aws_organizations_account", &["id"])],
                true,
                &[],
            )
            .with_explicit_destination()
        }
        ("aws_config_configuration_aggregator", "organization_aggregation_source.role_arn") => {
            Rule::new(
                "Organization aggregation role",
                &[("aws_iam_role", &["arn"])],
                false,
                &[],
            )
        }
        _ => return None,
    })
}
pub(crate) fn resource(kind: &str) -> bool {
    matches!(
        kind,
        "aws_config_aggregate_authorization" | "aws_config_configuration_aggregator"
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
    if e.nodes[source].resource_type != "aws_organizations_account" {
        return true;
    }
    if e.text(source, "id")
        .and_then(crate::model::AwsAccountScope::new)
        .is_none()
    {
        return false;
    }
    if r.attribute == "account_id" {
        return e
            .text(r.target, "authorized_aws_region")
            .and_then(super::scope_selections::region)
            .is_some();
    }
    let (v, m) = e.values[r.target];
    path.rsplit_once('.').is_some_and(|(prefix, _)| {
        super::scope_selections::selected_regions(v, m, prefix).is_some()
    })
}
#[cfg(test)]
#[path = "../../../tests/unit/semantic/config_aggregation.rs"]
mod tests;
