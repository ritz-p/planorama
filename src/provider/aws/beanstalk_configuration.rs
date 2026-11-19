use super::configured_links::{self, Evidence, Rule};
use crate::model::{AttributeReference, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;

fn rule(kind: &str, path: &str) -> Option<Rule> {
    Some(match (kind, path) {
        ("aws_elastic_beanstalk_application", "appversion_lifecycle.service_role") => Rule::new(
            "Application lifecycle role",
            &[("aws_iam_role", &["arn"])],
            false,
            &[],
        ),
        (
            "aws_elastic_beanstalk_application_version"
            | "aws_elastic_beanstalk_configuration_template"
            | "aws_elastic_beanstalk_environment",
            "application",
        ) => Rule::new(
            "Beanstalk application",
            &[("aws_elastic_beanstalk_application", &["name"])],
            false,
            &[],
        ),
        ("aws_elastic_beanstalk_application_version", "bucket") => Rule::new(
            "Version source bucket",
            &[("aws_s3_bucket", &["bucket", "id"])],
            false,
            &[],
        ),
        ("aws_elastic_beanstalk_environment", "version_label") => Rule::new(
            "Application version",
            &[("aws_elastic_beanstalk_application_version", &["name"])],
            false,
            &[],
        ),
        ("aws_elastic_beanstalk_environment", "template_name") => Rule::new(
            "Configuration template",
            &[("aws_elastic_beanstalk_configuration_template", &["name"])],
            false,
            &[],
        ),
        _ => return None,
    })
}
pub(crate) fn resource(kind: &str) -> bool {
    matches!(
        kind,
        "aws_elastic_beanstalk_application"
            | "aws_elastic_beanstalk_application_version"
            | "aws_elastic_beanstalk_configuration_template"
            | "aws_elastic_beanstalk_environment"
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
    !matches!(r.attribute.as_str(), "version_label" | "template_name")
        || e.same_endpoint(
            r.target,
            "application",
            source,
            "application",
            "aws_elastic_beanstalk_application",
            "name",
        )
}
#[cfg(test)]
#[path = "../../../tests/unit/semantic/beanstalk_configuration.rs"]
mod tests;
