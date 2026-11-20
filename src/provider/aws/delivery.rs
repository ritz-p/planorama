use super::configured_links::{self, Rule};
use crate::model::{AttributeReference, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;

const IAM_OWNER: &[(&str, &[&str])] = &[("aws_iam_user", &["arn"]), ("aws_iam_role", &["arn"])];

fn rule(kind: &str, path: &str) -> Option<Rule> {
    let (label, outputs): (_, &'static [(&str, &[&str])]) = match (kind, path) {
        ("aws_cloud9_environment_ec2", "subnet_id") => {
            ("Cloud9 subnet", &[("aws_subnet", &["id"])])
        }
        ("aws_cloud9_environment_ec2", "owner_arn") => ("Cloud9 owner", IAM_OWNER),
        ("aws_cloud9_environment_membership", "environment_id") => (
            "Cloud9 environment",
            &[("aws_cloud9_environment_ec2", &["id"])],
        ),
        ("aws_cloud9_environment_membership", "user_arn") => {
            ("Cloud9 member", &[("aws_iam_user", &["arn"])])
        }
        _ => return None,
    };
    Some(Rule::new(label, outputs, false, &[]))
}

pub(crate) fn resource(kind: &str) -> bool {
    matches!(
        kind,
        "aws_cloud9_environment_ec2" | "aws_cloud9_environment_membership"
    )
}

pub(crate) fn indexed(kind: &str, path: &str) -> bool {
    resource(kind) && configured_links::indexed(kind, path, rule)
}

pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    rule(kind, path).map(|r| r.label)
}

pub(super) fn bindings(raw: &Input, reference: &AttributeReference) -> bool {
    configured_links::bindings(raw, reference, rule)
}

pub(super) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    values: &[(&Value, &Value)],
) {
    configured_links::annotate(nodes, refs, values, rule, |_, _, _, _| true)
}

#[cfg(test)]
#[path = "../../../tests/unit/semantic/delivery.rs"]
mod tests;
