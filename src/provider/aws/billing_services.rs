use super::configured_links::{self, Rule};
use crate::model::{AttributeReference, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;

fn rule(kind: &str, path: &str) -> Option<Rule> {
    Some(match (kind, path) {
        (
            "aws_bcmdataexports_export",
            "export.destination_configurations.s3_destination.s3_bucket",
        ) => Rule::new(
            "Data export bucket",
            &[("aws_s3_bucket", &["bucket", "id"])],
            false,
            &[
                "export",
                "export.destination_configurations",
                "export.destination_configurations.s3_destination",
            ],
        )
        .in_control_plane_account(),
        ("aws_cur_report_definition", "s3_bucket") => Rule::new(
            "Cost and Usage Report bucket",
            &[("aws_s3_bucket", &["bucket", "id"])],
            false,
            &[],
        )
        .in_control_plane_account(),
        _ => return None,
    })
}

pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    rule(kind, path).map(|r| r.label)
}
pub(crate) fn indexed(kind: &str, path: &str) -> bool {
    configured_links::indexed(kind, path, rule)
}
pub(super) fn bindings(raw: &Input, reference: &AttributeReference) -> bool {
    configured_links::bindings(raw, reference, rule)
}
pub(super) fn annotate(
    nodes: &[TerraformEntity],
    references: &mut [AttributeReference],
    values: &[(&Value, &Value)],
) {
    configured_links::annotate(nodes, references, values, rule, qualifies)
}

fn qualifies(
    evidence: &configured_links::Evidence<'_>,
    reference: &AttributeReference,
    endpoint: usize,
    path: &str,
) -> bool {
    let source = &evidence.nodes[reference.target];
    let target = &evidence.nodes[endpoint];
    match (source.resource_type.as_str(), reference.attribute.as_str()) {
        ("aws_bcmdataexports_export" | "aws_cur_report_definition", attribute)
            if attribute.ends_with("s3_bucket") =>
        {
            let (block, _) = path.rsplit_once('.').unwrap_or(("", path));
            let region_path = if block.is_empty() {
                "s3_region".to_owned()
            } else {
                format!("{block}.s3_region")
            };
            let scope = super::scope::scope(target);
            evidence
                .text(reference.target, &region_path)
                .zip(scope)
                .is_some_and(|(region, scope)| {
                    scope.region.as_ref().map(|region| region.0.as_str()) == Some(region)
                })
        }
        _ => true,
    }
}
