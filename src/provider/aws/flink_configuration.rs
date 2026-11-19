use super::configured_links::{self, Evidence, Rule};
use crate::model::{AttributeReference, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;

fn rule(kind: &str, path: &str) -> Option<Rule> {
    Some(match (kind, path) {
        ("aws_kinesisanalyticsv2_application", "service_execution_role") => Rule::new(
            "Application execution role",
            &[("aws_iam_role", &["arn"])],
            false,
            &[],
        ),
        (
            "aws_kinesisanalyticsv2_application",
            "application_configuration.vpc_configuration.subnet_ids",
        ) => Rule::new("Application subnets", &[("aws_subnet", &["id"])], true, &[]),
        (
            "aws_kinesisanalyticsv2_application",
            "application_configuration.vpc_configuration.security_group_ids",
        ) => Rule::new(
            "Application security groups",
            &[("aws_security_group", &["id"])],
            true,
            &[],
        ),
        (
            "aws_kinesisanalyticsv2_application",
            "application_configuration.application_code_configuration.code_content.s3_content_location.bucket_arn",
        ) => Rule::new(
            "Application code bucket",
            &[("aws_s3_bucket", &["arn"])],
            false,
            &[],
        ),
        ("aws_kinesisanalyticsv2_application_snapshot", "application_name") => Rule::new(
            "Flink application",
            &[("aws_kinesisanalyticsv2_application", &["name"])],
            false,
            &[],
        ),
        _ => return None,
    })
}
pub(crate) fn resource(kind: &str) -> bool {
    matches!(
        kind,
        "aws_kinesisanalyticsv2_application" | "aws_kinesisanalyticsv2_application_snapshot"
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
    let app = if r.attribute == "application_name" {
        source
    } else {
        r.target
    };
    if !matches!(
        e.text(app, "runtime_environment"),
        Some(
            "FLINK-1_6"
                | "FLINK-1_8"
                | "FLINK-1_11"
                | "FLINK-1_13"
                | "FLINK-1_15"
                | "FLINK-1_18"
                | "FLINK-1_19"
                | "FLINK-1_20"
                | "FLINK-2_2"
                | "FLINK-2_3"
        )
    ) {
        return false;
    }
    match r.attribute.as_str() {
        "application_configuration.vpc_configuration.subnet_ids"
        | "application_configuration.vpc_configuration.security_group_ids" => {
            configured_links::network_context(
                e,
                r.target,
                &[
                    (
                        "application_configuration.vpc_configuration.subnet_ids",
                        "aws_subnet",
                    ),
                    (
                        "application_configuration.vpc_configuration.security_group_ids",
                        "aws_security_group",
                    ),
                ],
            )
            .is_some()
        }
        "application_configuration.application_code_configuration.code_content.s3_content_location.bucket_arn" => {
            e.text(
                r.target,
                "application_configuration.0.application_code_configuration.0.code_content_type",
            ) == Some("ZIPFILE")
        }
        _ => true,
    }
}
#[cfg(test)]
#[path = "../../../tests/unit/semantic/flink_configuration.rs"]
mod tests;

pub(super) fn present(raw: &Input, graph: &mut crate::model::Graph) {
    for reference in &raw.attributes {
        let target = reference.target;
        if raw.nodes[target].resource_type == "aws_kinesisanalyticsv2_application"
            && rule("aws_kinesisanalyticsv2_application", &reference.attribute).is_some()
            && super::service_references::verified(raw, reference)
        {
            graph.nodes[target].placement = crate::model::Placement::Spatial;
        }
    }
    super::lambda_core::present_network(
        raw,
        graph,
        "aws_kinesisanalyticsv2_application",
        (
            "application_configuration.vpc_configuration.subnet_ids",
            "application_configuration.vpc_configuration.security_group_ids",
        ),
        bindings,
    );
}
