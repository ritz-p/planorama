use super::configured_links::{self, Evidence, Rule};
use crate::model::{AttributeReference, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;

fn rule(kind: &str, path: &str) -> Option<Rule> {
    Some(match (kind, path) {
        ("aws_mwaa_environment", "network_configuration.subnet_ids") => {
            Rule::new("Environment subnets", &[("aws_subnet", &["id"])], true, &[])
        }
        ("aws_mwaa_environment", "network_configuration.security_group_ids") => Rule::new(
            "Environment security groups",
            &[("aws_security_group", &["id"])],
            true,
            &[],
        ),
        ("aws_mwaa_environment", "execution_role_arn") => Rule::new(
            "Environment execution role",
            &[("aws_iam_role", &["arn"])],
            false,
            &[],
        ),
        ("aws_mwaa_environment", "source_bucket_arn") => Rule::new(
            "Environment artifact bucket",
            &[("aws_s3_bucket", &["arn"])],
            false,
            &[],
        ),
        ("aws_mwaa_environment", "kms_key") => Rule::new(
            "Environment encryption key",
            &[("aws_kms_key", &["arn"])],
            false,
            &[],
        ),
        _ => return None,
    })
}
pub(crate) fn resource(kind: &str) -> bool {
    matches!(kind, "aws_mwaa_environment")
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
fn qualifies(e: &Evidence<'_>, r: &AttributeReference, _source: usize, _path: &str) -> bool {
    !matches!(
        r.attribute.as_str(),
        "network_configuration.subnet_ids" | "network_configuration.security_group_ids"
    ) || configured_links::network_context(
        e,
        r.target,
        &[
            ("network_configuration.subnet_ids", "aws_subnet"),
            (
                "network_configuration.security_group_ids",
                "aws_security_group",
            ),
        ],
    )
    .is_some()
}
#[cfg(test)]
#[path = "../../../tests/unit/semantic/mwaa_configuration.rs"]
mod tests;

pub(super) fn present(raw: &Input, graph: &mut crate::model::Graph) {
    super::lambda_core::present_network(
        raw,
        graph,
        "aws_mwaa_environment",
        (
            "network_configuration.subnet_ids",
            "network_configuration.security_group_ids",
        ),
        bindings,
    );
}
