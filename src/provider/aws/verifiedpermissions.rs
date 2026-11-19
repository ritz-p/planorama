use super::configured_links::{self, Evidence, Rule};
use crate::model::{AttributeReference, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;

fn rule(kind: &str, path: &str) -> Option<Rule> {
    Some(match (kind, path) {
        (
            "aws_verifiedpermissions_identity_source"
            | "aws_verifiedpermissions_policy"
            | "aws_verifiedpermissions_policy_template"
            | "aws_verifiedpermissions_schema",
            "policy_store_id",
        ) => Rule::new(
            "Policy store",
            &[(
                "aws_verifiedpermissions_policy_store",
                &["policy_store_id", "id"],
            )],
            false,
            &[],
        ),
        (
            "aws_verifiedpermissions_identity_source",
            "configuration.cognito_user_pool_configuration.user_pool_arn",
        ) => Rule::new(
            "Cognito user pool",
            &[("aws_cognito_user_pool", &["arn"])],
            false,
            &[],
        ),
        (
            "aws_verifiedpermissions_identity_source",
            "configuration.cognito_user_pool_configuration.client_ids",
        ) => Rule::new(
            "Cognito user pool clients",
            &[("aws_cognito_user_pool_client", &["id"])],
            true,
            &[],
        ),
        ("aws_verifiedpermissions_policy", "definition.template_linked.policy_template_id") => {
            Rule::new(
                "Linked policy template",
                &[(
                    "aws_verifiedpermissions_policy_template",
                    &["policy_template_id"],
                )],
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
        "aws_verifiedpermissions_identity_source"
            | "aws_verifiedpermissions_policy"
            | "aws_verifiedpermissions_policy_template"
            | "aws_verifiedpermissions_schema"
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
    let absent = |path: &str| {
        e.value(r.target, path)
            .is_some_and(|v| v.is_null() || v.as_array().is_some_and(Vec::is_empty))
    };
    match r.attribute.as_str() {
        "configuration.cognito_user_pool_configuration.user_pool_arn" => {
            absent("configuration.0.open_id_connect_configuration")
        }
        "configuration.cognito_user_pool_configuration.client_ids" => {
            absent("configuration.0.open_id_connect_configuration")
                && e.configured_endpoint(
                    r.target,
                    "configuration.0.cognito_user_pool_configuration.0.user_pool_arn",
                    "aws_cognito_user_pool",
                    &["arn"],
                )
                .zip(e.configured_endpoint(
                    source,
                    "user_pool_id",
                    "aws_cognito_user_pool",
                    &["id"],
                ))
                .is_some_and(|(left, right)| left == right)
        }
        "definition.template_linked.policy_template_id" => {
            absent("definition.0.static") && {
                let store = |n| {
                    e.configured_endpoint(
                        n,
                        "policy_store_id",
                        "aws_verifiedpermissions_policy_store",
                        &["policy_store_id", "id"],
                    )
                };
                store(r.target)
                    .zip(store(source))
                    .is_some_and(|(left, right)| left == right)
            }
        }
        _ => true,
    }
}
#[cfg(test)]
#[path = "../../../tests/unit/semantic/verifiedpermissions.rs"]
mod tests;
