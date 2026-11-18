use crate::model::{AttributeReference, TerraformEntity};
use serde_json::Value;

const KIND: &str = "aws_backup_report_plan";
const BUCKET: &str = "report_delivery_channel.s3_bucket_name";
const FRAMEWORKS: &str = "report_setting.framework_arns";

pub(crate) fn resource(kind: &str) -> bool {
    kind == KIND
}

pub(crate) fn indexed(kind: &str, path: &str) -> bool {
    resource(kind)
        && matches!(
            path,
            "report_delivery_channel.0.s3_bucket_name" | "report_setting.0.framework_arns"
        )
}

pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    if !resource(kind) {
        return None;
    }
    match path {
        BUCKET => Some("Backup report delivery bucket"),
        FRAMEWORKS => Some("Backup report frameworks"),
        _ => None,
    }
}

fn fields(_: &str, path: &str, source: &str) -> &'static [&'static str] {
    match (path, source) {
        (BUCKET, "aws_s3_bucket") => &["id", "bucket"],
        (FRAMEWORKS, "aws_backup_framework") => &["arn"],
        _ => &[],
    }
}

fn compatible(left: &TerraformEntity, right: &TerraformEntity) -> bool {
    let l = super::scope::scope(left);
    let r = super::scope::scope(right);
    super::eventbridge::bindings(
        left.provider_configuration.as_ref(),
        right.provider_configuration.as_ref(),
        l.as_ref().and_then(|s| s.region.as_ref()),
        r.as_ref().and_then(|s| s.region.as_ref()),
        false,
    )
}

pub(super) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    evidence: &[(&Value, &Value)],
) {
    let results: Vec<_> = refs
        .iter()
        .map(|r| {
            label(&nodes[r.target].resource_type, &r.attribute)?;
            let (v, m) = evidence[r.target];
            let block = r.attribute.split('.').next().unwrap();
            Some(
                super::nested_references::singleton(v, m, block)
                    && super::nested_references::verify(
                        nodes,
                        refs,
                        evidence,
                        r,
                        fields,
                        |s, _| compatible(&nodes[r.target], &nodes[s]),
                    )
                    .is_some(),
            )
        })
        .collect();
    for (r, result) in refs.iter_mut().zip(results) {
        if let Some(valid) = result {
            r.service_reference_verified = valid;
            if valid {
                r.complete = true;
            }
        }
    }
}

pub(super) fn bindings(raw: &crate::semantic::Input, r: &AttributeReference) -> bool {
    label(&raw.nodes[r.target].resource_type, &r.attribute).is_none()
        || r.sources.iter().all(|&s| {
            let l = &raw.nodes[r.target];
            let right = &raw.nodes[s];
            super::eventbridge::bindings(
                l.provider_configuration.as_ref(),
                right.provider_configuration.as_ref(),
                l.entity.scope.as_ref().and_then(|s| s.region.as_ref()),
                right.entity.scope.as_ref().and_then(|s| s.region.as_ref()),
                false,
            )
        })
}

#[cfg(test)]
#[path = "../../../tests/unit/semantic/backup_reports.rs"]
mod tests;
