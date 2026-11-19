use super::configured_links::{self, Evidence, Rule};
use crate::model::{AttributeReference, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;

fn rule(kind: &str, path: &str) -> Option<Rule> {
    Some(match (kind, path) {
        ("aws_batch_compute_environment", "service_role") => Rule::new(
            "Batch service role",
            &[("aws_iam_role", &["arn"])],
            false,
            &[],
        ),
        ("aws_batch_compute_environment", "eks_configuration.eks_cluster_arn") => {
            Rule::new("EKS cluster", &[("aws_eks_cluster", &["arn"])], false, &[])
        }
        ("aws_batch_compute_environment", "compute_resources.instance_role") => Rule::new(
            "EC2 instance profile",
            &[("aws_iam_instance_profile", &["arn"])],
            false,
            &[],
        ),
        ("aws_batch_compute_environment", "compute_resources.spot_iam_fleet_role") => {
            Rule::new("Spot fleet role", &[("aws_iam_role", &["arn"])], false, &[])
        }
        (
            "aws_batch_compute_environment",
            "compute_resources.launch_template.launch_template_id",
        ) => Rule::new(
            "Launch template",
            &[("aws_launch_template", &["id"])],
            false,
            &[],
        ),
        (
            "aws_batch_compute_environment",
            "compute_resources.launch_template.launch_template_name",
        ) => Rule::new(
            "Launch template",
            &[("aws_launch_template", &["name"])],
            false,
            &[],
        ),
        (
            "aws_batch_compute_environment",
            "compute_resources.ec2_configuration.image_id_override",
        ) => Rule::new(
            "Compute image override",
            &[
                ("aws_ami", &["id"]),
                ("aws_ami_copy", &["id"]),
                ("aws_ami_from_instance", &["id"]),
            ],
            false,
            &["compute_resources.ec2_configuration"],
        ),
        ("aws_batch_compute_environment", "compute_resources.image_id") => Rule::new(
            "Compute image",
            &[
                ("aws_ami", &["id"]),
                ("aws_ami_copy", &["id"]),
                ("aws_ami_from_instance", &["id"]),
            ],
            false,
            &[],
        ),
        _ => return None,
    })
}
pub(crate) fn resource(kind: &str) -> bool {
    matches!(kind, "aws_batch_compute_environment")
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
fn qualifies(e: &Evidence<'_>, r: &AttributeReference, _source: usize, path: &str) -> bool {
    let mode = e.text(r.target, "type");
    let capacity = e.text(r.target, "compute_resources.0.type");
    let ec2 = matches!(capacity, Some("EC2" | "SPOT"));
    let unmanaged = mode == Some("UNMANAGED")
        && e.value(r.target, "compute_resources")
            .is_some_and(|v| v.is_null() || v.as_array().is_some_and(Vec::is_empty));
    match r.attribute.as_str() {
        "service_role" => {
            unmanaged
                || mode == Some("MANAGED")
                    && matches!(capacity, Some("EC2" | "SPOT" | "FARGATE" | "FARGATE_SPOT"))
        }
        "eks_configuration.eks_cluster_arn" => unmanaged || mode == Some("MANAGED") && ec2,
        _ if mode != Some("MANAGED") || !ec2 => false,
        "compute_resources.spot_iam_fleet_role" => capacity == Some("SPOT"),
        "compute_resources.launch_template.launch_template_id"
        | "compute_resources.launch_template.launch_template_name" => {
            let Some((parent, field)) = path.rsplit_once('.') else {
                return false;
            };
            let other = if field == "launch_template_id" {
                "launch_template_name"
            } else {
                "launch_template_id"
            };
            e.value(r.target, &format!("{parent}.{other}"))
                .is_some_and(|v| v.is_null() || v.as_str() == Some(""))
        }
        "compute_resources.image_id" => e
            .value(r.target, "compute_resources.0.ec2_configuration")
            .is_some_and(|v| {
                v.is_null()
                    || v.as_array().is_some_and(|items| {
                        items.iter().enumerate().all(|(i, _)| {
                            e.value(
                                r.target,
                                &format!(
                                    "compute_resources.0.ec2_configuration.{i}.image_id_override"
                                ),
                            )
                            .is_some_and(|id| id.is_null() || id.as_str() == Some(""))
                        })
                    })
            }),
        _ => true,
    }
}
#[cfg(test)]
#[path = "../../../tests/unit/semantic/batch_configuration.rs"]
mod tests;
