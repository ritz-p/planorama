use super::configured_links::{self, Evidence, Rule};
use crate::model::{AttributeReference, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;

fn rule(kind: &str, path: &str) -> Option<Rule> {
    Some(match (kind, path) {
        ("aws_apprunner_service", "auto_scaling_configuration_arn") => Rule::new(
            "Auto scaling configuration",
            &[("aws_apprunner_auto_scaling_configuration_version", &["arn"])],
            false,
            &[],
        ),
        ("aws_apprunner_service", "instance_configuration.instance_role_arn") => {
            Rule::new("Instance role", &[("aws_iam_role", &["arn"])], false, &[])
        }
        (
            "aws_apprunner_service",
            "source_configuration.authentication_configuration.access_role_arn",
        ) => Rule::new(
            "Repository access role",
            &[("aws_iam_role", &["arn"])],
            false,
            &[],
        ),
        (
            "aws_apprunner_service",
            "source_configuration.authentication_configuration.connection_arn",
        ) => Rule::new(
            "Source connection",
            &[("aws_apprunner_connection", &["arn"])],
            false,
            &[],
        ),
        (
            "aws_apprunner_service",
            "observability_configuration.observability_configuration_arn",
        ) => Rule::new(
            "Observability configuration",
            &[("aws_apprunner_observability_configuration", &["arn"])],
            false,
            &[],
        ),
        (
            "aws_apprunner_default_auto_scaling_configuration_version",
            "auto_scaling_configuration_arn",
        ) => Rule::new(
            "Default scaling configuration",
            &[("aws_apprunner_auto_scaling_configuration_version", &["arn"])],
            false,
            &[],
        ),
        ("aws_apprunner_deployment" | "aws_apprunner_custom_domain_association", "service_arn") => {
            Rule::new(
                "App Runner service",
                &[("aws_apprunner_service", &["arn"])],
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
        "aws_apprunner_service"
            | "aws_apprunner_default_auto_scaling_configuration_version"
            | "aws_apprunner_deployment"
            | "aws_apprunner_custom_domain_association"
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
fn qualifies(e: &Evidence<'_>, r: &AttributeReference, _source: usize, _path: &str) -> bool {
    let absent = |path| {
        e.value(r.target, path)
            .is_some_and(|value| value.is_null() || value.as_array().is_some_and(Vec::is_empty))
    };
    let (values, mask) = e.values[r.target];
    let singleton = |path| super::nested_references::singleton(values, mask, path);
    match r.attribute.as_str() {
        "observability_configuration.observability_configuration_arn" => {
            e.value(
                r.target,
                "observability_configuration.0.observability_enabled",
            )
            .and_then(Value::as_bool)
                == Some(true)
        }
        "source_configuration.authentication_configuration.access_role_arn" => {
            absent("source_configuration.0.code_repository")
                && singleton("source_configuration.0.image_repository")
                && e.text(
                    r.target,
                    "source_configuration.0.image_repository.0.image_repository_type",
                ) == Some("ECR")
                && absent("source_configuration.0.authentication_configuration.0.connection_arn")
        }
        "source_configuration.authentication_configuration.connection_arn" => {
            absent("source_configuration.0.image_repository")
                && singleton("source_configuration.0.code_repository")
                && absent("source_configuration.0.authentication_configuration.0.access_role_arn")
        }
        _ => true,
    }
}
#[cfg(test)]
#[path = "../../../tests/unit/semantic/apprunner_configuration.rs"]
mod tests;
