use crate::model::{AttributeReference, AwsAccountScope, RegionScope, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;
#[cfg(test)]
#[path = "../../../tests/unit/semantic/imagebuilder_distribution.rs"]
mod tests;
const DIST: &str = "aws_imagebuilder_distribution_configuration";
const LIFE: &str = "aws_imagebuilder_lifecycle_policy";
const KMS: &str = "distribution.ami_distribution_configuration.kms_key_id";
const ECR: &str =
    "distribution.container_distribution_configuration.target_repository.repository_name";
const LT: &str = "distribution.launch_template_configuration.launch_template_id";
const BUCKET: &str = "distribution.s3_export_configuration.s3_bucket";
const ROLE: &str = "distribution.s3_export_configuration.role_name";
const PARAMETER: &str = "distribution.ssm_parameter_configuration.parameter_name";
const NAME: &str = "resource_selection.recipe.name";
const VERSION: &str = "resource_selection.recipe.semantic_version";
const MANY: &[&str] = &[
    "distribution",
    "distribution.launch_template_configuration",
    "distribution.ssm_parameter_configuration",
    "resource_selection.recipe",
];
pub(crate) fn resource(k: &str) -> bool {
    matches!(k, DIST | LIFE)
}
fn canonical(p: &str) -> String {
    p.split('.')
        .filter(|s| s.parse::<usize>().is_err())
        .collect::<Vec<_>>()
        .join(".")
}
pub(super) fn label(k: &str, p: &str) -> Option<&'static str> {
    match (k, p) {
        (DIST, KMS) => Some("Distribution encryption keys"),
        (DIST, ECR) => Some("Distribution repositories"),
        (DIST, LT) => Some("Distribution launch templates"),
        (DIST, BUCKET) => Some("Image export buckets"),
        (DIST, ROLE) => Some("Image export roles"),
        (DIST, PARAMETER) => Some("Distribution parameters"),
        (LIFE, "execution_role") => Some("Lifecycle execution role"),
        (LIFE, NAME) => Some("Lifecycle recipes"),
        (LIFE, VERSION) => Some("Lifecycle recipe versions"),
        _ => None,
    }
}
pub(crate) fn indexed(k: &str, p: &str) -> bool {
    p.split('.').any(|s| s.parse::<usize>().is_ok()) && label(k, &canonical(p)).is_some()
}
fn fields(k: &str, p: &str, s: &str) -> &'static [&'static str] {
    match (k, p, s) {
        (DIST, KMS, "aws_kms_key") | (LIFE, "execution_role", "aws_iam_role") => &["arn"],
        (DIST, ECR, "aws_ecr_repository")
        | (DIST, ROLE, "aws_iam_role")
        | (DIST, PARAMETER, "aws_ssm_parameter") => &["name"],
        (DIST, LT, "aws_launch_template") => &["id"],
        (DIST, BUCKET, "aws_s3_bucket") => &["bucket", "id"],
        (LIFE, NAME, "aws_imagebuilder_image_recipe" | "aws_imagebuilder_container_recipe") => {
            &["name"]
        }
        (LIFE, VERSION, "aws_imagebuilder_image_recipe" | "aws_imagebuilder_container_recipe") => {
            &["version"]
        }
        _ => &[],
    }
}
fn shape(v: &Value, path: &str, prefix: &str) -> bool {
    let (field, rest) = path
        .split_once('.')
        .map_or((path, None), |(a, b)| (a, Some(b)));
    let v = &v[field];
    if v.is_null() {
        return true;
    }
    let Some(rest) = rest else {
        return v.is_string();
    };
    let prefix = if prefix.is_empty() {
        field.into()
    } else {
        format!("{prefix}.{field}")
    };
    v.as_array().is_some_and(|rows| {
        (MANY.contains(&prefix.as_str()) || rows.len() <= 1)
            && rows.iter().all(|r| shape(r, rest, &prefix))
    })
}
fn destination(v: &Value, m: &Value, path: &str) -> Option<(RegionScope, Option<AwsAccountScope>)> {
    let parts: Vec<_> = path.split('.').collect();
    let ["distribution", index, ..] = parts.as_slice() else {
        return None;
    };
    index.parse::<usize>().ok()?;
    let root = format!("distribution.{index}");
    let p = canonical(path);
    let value = |p: &str| super::nested_references::value(v, m, p);
    let region = RegionScope(value(&format!("{root}.region"))?.as_str()?.to_string());
    if region.0.is_empty() {
        return None;
    }
    let row = path.rsplit_once('.')?.0;
    let account = if p == KMS {
        let arn = value(path)?.as_str()?;
        if !super::s3_exports::kms_arn("aws_kms_key", arn) {
            return None;
        }
        let parts: Vec<_> = arn.splitn(6, ':').collect();
        if parts[3] != region.0 {
            return None;
        }
        Some(AwsAccountScope::new(parts[4])?)
    } else if matches!(p.as_str(), LT | PARAMETER) {
        let key = if p == LT {
            "account_id"
        } else {
            "ami_account_id"
        };
        let account = value(&format!("{row}.{key}"))?;
        if account.is_null() {
            None
        } else {
            let account = AwsAccountScope::new(account.as_str()?)?;
            if p == PARAMETER {
                let targets = value(&format!(
                    "{root}.ami_distribution_configuration.0.target_account_ids"
                ))?
                .as_array()?;
                let targets = targets
                    .iter()
                    .map(|v| AwsAccountScope::new(v.as_str()?))
                    .collect::<Option<std::collections::BTreeSet<_>>>()?;
                if !targets.contains(&account) {
                    return None;
                }
            }
            Some(account)
        }
    } else {
        None
    };
    if p == ECR && value(&format!("{row}.service"))?.as_str() != Some("ECR") {
        return None;
    }
    Some((region, account))
}
fn selected_recipe(
    nodes: &[TerraformEntity],
    e: &[(&Value, &Value)],
    target: usize,
    source: usize,
    path: &str,
) -> bool {
    let valid = || {
        let (v, m) = e[target];
        let value = |p: &str| super::nested_references::value(v, m, p);
        let k = match value("resource_type")?.as_str()? {
            "AMI_IMAGE" => "aws_imagebuilder_image_recipe",
            "CONTAINER_IMAGE" => "aws_imagebuilder_container_recipe",
            _ => return None,
        };
        if nodes[source].resource_type != k {
            return None;
        }
        let row = path.rsplit_once('.')?.0;
        let name = value(&format!("{row}.name"))?.as_str()?;
        let version = value(&format!("{row}.semantic_version"))?.as_str()?;
        if version.split('.').count() != 3
            || !version
                .split('.')
                .all(|s| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()))
        {
            return None;
        }
        (super::service_evidence::known(e, source, "name") == Some(name)
            && super::service_evidence::known(e, source, "version") == Some(version))
        .then_some(())
    };
    valid().is_some()
}
pub(super) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    e: &[(&Value, &Value)],
) {
    let results: Vec<_> = refs
        .iter()
        .map(|r| {
            let k = nodes[r.target].resource_type.as_str();
            let (v, m) = e[r.target];
            if k == DIST && indexed(k, &r.attribute) {
                return Some((
                    destination(v, m, &r.attribute).is_some(),
                    destination(v, m, &r.attribute),
                ));
            }
            label(k, &r.attribute)?;
            let ok = shape(v, &r.attribute, "")
                && super::nested_references::verify_collections(
                    nodes,
                    refs,
                    e,
                    r,
                    fields,
                    |s, path| {
                        if k == DIST {
                            destination(v, m, path).is_some()
                        } else if matches!(r.attribute.as_str(), NAME | VERSION) {
                            selected_recipe(nodes, e, r.target, s, path)
                        } else {
                            true
                        }
                    },
                    MANY,
                )
                .is_some();
            Some((ok, None))
        })
        .collect();
    for (r, result) in refs.iter_mut().zip(results) {
        if let Some((ok, scope)) = result {
            r.service_reference_verified = ok;
            if ok {
                r.complete = true;
            }
            if let Some((region, account)) = scope {
                r.service_reference_region = Some(region);
                r.service_reference_owner_account = account;
            }
        }
    }
}
fn scoped_binding(raw: &Input, r: &AttributeReference, region: Option<&RegionScope>) -> bool {
    let l = &raw.nodes[r.target];
    if l.entity
        .scope
        .as_ref()
        .and_then(|s| s.region.as_ref())
        .is_none()
    {
        return false;
    }
    r.sources.iter().all(|&s| {
        let right = &raw.nodes[s];
        let mut binding = l.provider_configuration.clone();
        if let Some(account) = &r.service_reference_owner_account {
            let Some(b) = binding.as_mut() else {
                return false;
            };
            if b.account_conflict {
                return false;
            }
            b.account = Some(account.clone());
        }
        super::eventbridge::bindings(
            binding.as_ref(),
            right.provider_configuration.as_ref(),
            region,
            right.entity.scope.as_ref().and_then(|s| s.region.as_ref()),
            right.resource_type == "aws_iam_role",
        )
    })
}
pub(super) fn bindings(raw: &Input, r: &AttributeReference) -> bool {
    let k = raw.nodes[r.target].resource_type.as_str();
    if label(k, &r.attribute).is_none() {
        return true;
    }
    if k == DIST {
        let leaves: Vec<_> = raw
            .attributes
            .iter()
            .filter(|x| {
                x.target == r.target
                    && indexed(k, &x.attribute)
                    && canonical(&x.attribute) == r.attribute
            })
            .collect();
        !leaves.is_empty()
            && leaves.iter().all(|x| {
                x.service_reference_verified
                    && x.service_reference_region.is_some()
                    && scoped_binding(raw, x, x.service_reference_region.as_ref())
            })
    } else {
        scoped_binding(
            raw,
            r,
            raw.nodes[r.target]
                .entity
                .scope
                .as_ref()
                .and_then(|s| s.region.as_ref()),
        )
    }
}
