use crate::model::{AttributeReference, Graph, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;
#[cfg(test)]
#[path = "../../../tests/unit/semantic/imagebuilder.rs"]
mod tests;
const COMPONENT: &str = "aws_imagebuilder_component";
const WORKFLOW: &str = "aws_imagebuilder_workflow";
const RECIPE: &str = "aws_imagebuilder_image_recipe";
const CONTAINER: &str = "aws_imagebuilder_container_recipe";
const IMAGE: &str = "aws_imagebuilder_image";
const PIPELINE: &str = "aws_imagebuilder_image_pipeline";
const INFRA: &str = "aws_imagebuilder_infrastructure_configuration";
const MANY: &[&str] = &[
    "component",
    "workflow",
    "block_device_mapping",
    "instance_configuration.block_device_mapping",
];
pub(crate) fn resource(k: &str) -> bool {
    matches!(
        k,
        COMPONENT | WORKFLOW | RECIPE | CONTAINER | IMAGE | PIPELINE | INFRA
    )
}
pub(super) fn label(k: &str, p: &str) -> Option<&'static str> {
    match (k, p) {
        (COMPONENT | WORKFLOW | CONTAINER, "kms_key_id") => Some("Build encryption key"),
        (RECIPE | CONTAINER, "component.component_arn") => Some("Recipe components"),
        (RECIPE | CONTAINER, "parent_image") => Some("Recipe base image"),
        (CONTAINER, "target_repository.repository_name") => Some("Target container repository"),
        (RECIPE, "block_device_mapping.ebs.kms_key_id")
        | (CONTAINER, "instance_configuration.block_device_mapping.ebs.kms_key_id") => {
            Some("Build volume encryption key")
        }
        (RECIPE, "block_device_mapping.ebs.snapshot_id")
        | (CONTAINER, "instance_configuration.block_device_mapping.ebs.snapshot_id") => {
            Some("Build volume snapshot")
        }
        (IMAGE | PIPELINE, "image_recipe_arn" | "container_recipe_arn") => Some("Build recipe"),
        (IMAGE | PIPELINE, "infrastructure_configuration_arn") => Some("Build infrastructure"),
        (IMAGE | PIPELINE, "distribution_configuration_arn") => Some("Build distribution"),
        (IMAGE | PIPELINE, "workflow.workflow_arn") => Some("Build workflows"),
        (IMAGE | PIPELINE, "execution_role") => Some("Workflow execution role"),
        (INFRA, "subnet_id") => Some("Build subnet"),
        (INFRA, "security_group_ids") => Some("Build security groups"),
        (INFRA, "instance_profile_name") => Some("Build instance profile"),
        (INFRA, "sns_topic_arn") => Some("Build notification topic"),
        (INFRA, "logging.s3_logs.s3_bucket_name") => Some("Build log bucket"),
        _ => None,
    }
}
pub(crate) fn indexed(k: &str, p: &str) -> bool {
    p.split('.').any(|s| s.parse::<usize>().is_ok())
        && label(
            k,
            &p.split('.')
                .filter(|s| s.parse::<usize>().is_err())
                .collect::<Vec<_>>()
                .join("."),
        )
        .is_some()
}
fn fields(k: &str, p: &str, s: &str) -> &'static [&'static str] {
    match (k, p, s) {
        (COMPONENT | WORKFLOW | CONTAINER, "kms_key_id", "aws_kms_key")
        | (RECIPE, "block_device_mapping.ebs.kms_key_id", "aws_kms_key")
        | (
            CONTAINER,
            "instance_configuration.block_device_mapping.ebs.kms_key_id",
            "aws_kms_key",
        ) => &["arn"],
        (RECIPE | CONTAINER, "component.component_arn", COMPONENT)
        | (RECIPE | CONTAINER, "parent_image", IMAGE)
        | (IMAGE | PIPELINE, "image_recipe_arn", RECIPE)
        | (IMAGE | PIPELINE, "container_recipe_arn", CONTAINER)
        | (IMAGE | PIPELINE, "infrastructure_configuration_arn", INFRA)
        | (
            IMAGE | PIPELINE,
            "distribution_configuration_arn",
            "aws_imagebuilder_distribution_configuration",
        )
        | (IMAGE | PIPELINE, "workflow.workflow_arn", WORKFLOW) => &["arn", "id"],
        (RECIPE, "parent_image", "aws_ami")
        | (RECIPE, "block_device_mapping.ebs.snapshot_id", "aws_ebs_snapshot")
        | (
            CONTAINER,
            "instance_configuration.block_device_mapping.ebs.snapshot_id",
            "aws_ebs_snapshot",
        )
        | (INFRA, "subnet_id", "aws_subnet")
        | (INFRA, "security_group_ids", "aws_security_group") => &["id"],
        (CONTAINER, "target_repository.repository_name", "aws_ecr_repository")
        | (INFRA, "instance_profile_name", "aws_iam_instance_profile") => &["name"],
        (IMAGE | PIPELINE, "execution_role", "aws_iam_role")
        | (INFRA, "sns_topic_arn", "aws_sns_topic") => &["arn"],
        (INFRA, "logging.s3_logs.s3_bucket_name", "aws_s3_bucket") => &["bucket", "id"],
        _ => &[],
    }
}
fn shape(v: &Value, path: &str, prefix: &str) -> bool {
    let (field, rest) = path
        .split_once('.')
        .map_or((path, None), |(a, b)| (a, Some(b)));
    let v = &v[field];
    let Some(rest) = rest else {
        return if field == "security_group_ids" {
            v.is_array()
        } else {
            v.is_string()
        };
    };
    let prefix = if prefix.is_empty() {
        field.into()
    } else {
        format!("{prefix}.{field}")
    };
    v.as_array().is_some_and(|rows| {
        !rows.is_empty()
            && (MANY.contains(&prefix.as_str()) || rows.len() == 1)
            && rows.iter().all(|v| shape(v, rest, &prefix))
    })
}
fn exact_versioned_arn(kind: &str, arn: &str) -> bool {
    let parts: Vec<_> = arn.splitn(6, ':').collect();
    let ["arn", partition, "imagebuilder", region, account, resource] = parts.as_slice() else {
        return false;
    };
    if partition.is_empty()
        || region.is_empty()
        || crate::model::AwsAccountScope::new(account).is_none()
    {
        return false;
    }
    let p: Vec<_> = resource.split('/').collect();
    let (prefix, version, build) = match kind {
        COMPONENT => ("component", 2, true),
        WORKFLOW => ("workflow", 3, true),
        IMAGE => ("image", 2, true),
        RECIPE => ("image-recipe", 2, false),
        CONTAINER => ("container-recipe", 2, false),
        _ => return false,
    };
    p.len() == version + 1 + usize::from(build)
        && p[0] == prefix
        && !p[version - 1].is_empty()
        && (kind != WORKFLOW || matches!(p[1], "build" | "test"))
        && p[version].split('.').count() == 3
        && p[version]
            .split('.')
            .all(|s| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()))
        && (!build
            || (!p[version + 1].is_empty() && p[version + 1].bytes().all(|b| b.is_ascii_digit())))
}
fn proof(
    nodes: &[TerraformEntity],
    refs: &[AttributeReference],
    e: &[(&Value, &Value)],
    r: &AttributeReference,
) -> Option<()> {
    let k = nodes[r.target].resource_type.as_str();
    let (v, m) = e[r.target];
    if !shape(v, &r.attribute, "") {
        return None;
    }
    if matches!(
        r.attribute.as_str(),
        "image_recipe_arn" | "container_recipe_arn"
    ) {
        let other = if r.attribute == "image_recipe_arn" {
            "container_recipe_arn"
        } else {
            "image_recipe_arn"
        };
        if !super::nested_references::value(v, m, other)?.is_null() {
            return None;
        }
    }
    if k == CONTAINER
        && super::nested_references::value(v, m, "container_type")?.as_str() != Some("DOCKER")
    {
        return None;
    }
    super::nested_references::verify_collections(
        nodes,
        refs,
        e,
        r,
        fields,
        |s, path| {
            let valid = || {
                let actual = super::nested_references::value(v, m, path)?.as_str();
                let source = nodes[s].resource_type.as_str();
                if matches!(source, COMPONENT | WORKFLOW | IMAGE | RECIPE | CONTAINER)
                    && !exact_versioned_arn(source, actual?)
                {
                    return None;
                }
                if r.attribute == "target_repository.repository_name"
                    && super::nested_references::value(v, m, "target_repository.0.service")?
                        .as_str()
                        != Some("ECR")
                {
                    return None;
                }
                if r.attribute.contains(".ebs.") {
                    let (block, leaf) = path.split_once(".ebs.")?;
                    if !matches!(
                        super::nested_references::value(v, m, &format!("{block}.no_device"))?,
                        Value::Null | Value::Bool(false)
                    ) {
                        return None;
                    }
                    if leaf.ends_with("kms_key_id")
                        && super::nested_references::value(
                            v,
                            m,
                            &format!("{block}.ebs.0.encrypted"),
                        )?
                        .as_str()
                            != Some("true")
                    {
                        return None;
                    }
                }
                Some(())
            };
            valid().is_some()
        },
        MANY,
    )
}
pub(super) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    e: &[(&Value, &Value)],
) {
    let results: Vec<_> = refs
        .iter()
        .map(|r| {
            label(&nodes[r.target].resource_type, &r.attribute)
                .map(|_| proof(nodes, refs, e, r).is_some())
        })
        .collect();
    for (r, result) in refs.iter_mut().zip(results) {
        if let Some(ok) = result {
            r.service_reference_verified = ok;
            if ok {
                r.complete = true;
            }
        }
    }
    let contexts: Vec<_> = refs
        .iter()
        .map(|r| {
            if nodes[r.target].resource_type != INFRA
                || !matches!(r.attribute.as_str(), "subnet_id" | "security_group_ids")
            {
                return None;
            }
            let context = || {
                let subnet = super::lambda_core::unique(refs, r.target, "subnet_id")?;
                let groups = super::lambda_core::unique(refs, r.target, "security_group_ids")?;
                if !subnet.service_reference_verified || !groups.service_reference_verified {
                    return None;
                }
                let parents = subnet
                    .sources
                    .iter()
                    .chain(&groups.sources)
                    .map(|&s| super::lambda_core::parent(nodes, refs, e, s))
                    .collect::<Option<std::collections::BTreeSet<_>>>()?;
                (parents.len() == 1).then(|| *parents.first().unwrap())
            };
            Some(context().into_iter().collect::<Vec<_>>())
        })
        .collect();
    for (r, c) in refs.iter_mut().zip(contexts) {
        if let Some(c) = c {
            r.endpoint_network_context = c;
        }
    }
}
pub(super) fn bindings(raw: &Input, r: &AttributeReference) -> bool {
    if label(&raw.nodes[r.target].resource_type, &r.attribute).is_none() {
        return true;
    }
    r.sources
        .iter()
        .chain(&r.endpoint_network_context)
        .all(|&s| {
            let l = &raw.nodes[r.target];
            let right = &raw.nodes[s];
            super::eventbridge::bindings(
                l.provider_configuration.as_ref(),
                right.provider_configuration.as_ref(),
                l.entity.scope.as_ref().and_then(|s| s.region.as_ref()),
                right.entity.scope.as_ref().and_then(|s| s.region.as_ref()),
                matches!(
                    right.resource_type.as_str(),
                    "aws_iam_role" | "aws_iam_instance_profile"
                ),
            )
        })
}
pub(super) fn present(raw: &Input, g: &mut Graph) {
    super::lambda_core::present_network(
        raw,
        g,
        INFRA,
        ("subnet_id", "security_group_ids"),
        bindings,
    );
}
