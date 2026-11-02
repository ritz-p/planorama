use crate::model::{AttributeReference, TerraformEntity};
use serde_json::Value;

const ACCESS: &str = "aws_s3_access_point";
const OBJECT: &str = "aws_s3control_object_lambda_access_point";
const ENDPOINT: &str = "aws_s3outposts_endpoint";
const FUNCTION: &str =
    "configuration.transformation_configuration.content_transformation.aws_lambda.function_arn";

pub(crate) fn indexed(kind: &str, path: &str) -> bool {
    kind == OBJECT
        && path
            .split('.')
            .filter(|s| s.parse::<usize>().is_err())
            .collect::<Vec<_>>()
            .join(".")
            == FUNCTION
        && path != FUNCTION
}

fn functions(
    nodes: &[TerraformEntity],
    refs: &[AttributeReference],
    evidence: &[(&Value, &Value)],
    r: &AttributeReference,
) -> bool {
    let (v, m) = evidence[r.target];
    let Some(config) = v["configuration"].as_array().filter(|a| a.len() == 1) else {
        return false;
    };
    let Some(rows) = config[0]["transformation_configuration"]
        .as_array()
        .filter(|a| !a.is_empty())
    else {
        return false;
    };
    if !matches!(m, Value::Null | Value::Bool(false) | Value::Object(_)) {
        return false;
    }
    let cm = match &m["configuration"] {
        Value::Null | Value::Bool(false) => &Value::Null,
        Value::Array(a) if a.len() == 1 => &a[0],
        _ => return false,
    };
    if !matches!(cm, Value::Null | Value::Bool(false) | Value::Object(_)) {
        return false;
    }
    let masks = match &cm["transformation_configuration"] {
        Value::Null | Value::Bool(false) => None,
        Value::Array(a) if a.len() == rows.len() => Some(a),
        _ => return false,
    };
    let mut values = std::collections::BTreeSet::new();
    for (i, row) in rows.iter().enumerate() {
        let Some(value) = super::service_references::scalar(
            row,
            masks.map_or(&Value::Null, |a| &a[i]),
            &["content_transformation", "aws_lambda", "function_arn"],
        ) else {
            return false;
        };
        if !values.insert(value) {
            return false;
        }
    }
    let mut sources = std::collections::BTreeSet::new();
    let mut expected = std::collections::BTreeSet::new();
    for i in 0..rows.len() {
        let path = format!(
            "configuration.0.transformation_configuration.{i}.content_transformation.0.aws_lambda.0.function_arn"
        );
        let mut found = refs
            .iter()
            .filter(|a| a.target == r.target && a.attribute == path);
        let Some(a) = found.next() else { return false };
        let Some(s) = super::route53::endpoint(nodes, a, "aws_lambda_function", &["arn"]) else {
            return false;
        };
        if found.next().is_some()
            || a.indexed_block_lengths != [1, rows.len(), 1, 1]
            || !sources.insert(s)
            || !super::route53::configured(&nodes[r.target], &nodes[s])
        {
            return false;
        }
        let left = super::scope::scope(&nodes[r.target]).and_then(|s| s.region);
        if left.is_none() || left != super::scope::scope(&nodes[s]).and_then(|s| s.region) {
            return false;
        }
        let Some(arn) = scalar(evidence, s, "arn") else {
            return false;
        };
        expected.insert(arn);
    }
    r.complete
        && r.instances_explicit
        && r.issues.is_empty()
        && r.source_attributes == ["arn".to_owned()].into()
        && sources == r.sources.iter().copied().collect()
        && values == expected
}

pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    match (kind, path) {
        (ACCESS, "bucket") => Some("Access point bucket"),
        (ACCESS, "vpc_configuration.vpc_id") => Some("Access point VPC restriction"),
        ("aws_s3control_access_point_policy", "access_point_arn") => {
            Some("Access point policy owner")
        }
        ("aws_s3control_bucket", "outpost_id") | (ENDPOINT, "outpost_id") => {
            Some("Configured Outpost")
        }
        (
            "aws_s3control_bucket_policy" | "aws_s3control_bucket_lifecycle_configuration",
            "bucket",
        ) => Some("Outposts bucket owner"),
        (
            "aws_s3control_directory_bucket_access_point_scope"
            | "aws_s3control_object_lambda_access_point_policy",
            "name",
        ) => Some("Named access point owner"),
        (OBJECT, "configuration.supporting_access_point") => Some("Supporting access point"),
        (OBJECT, FUNCTION) => Some("Object transformation function"),
        (ENDPOINT, "subnet_id") => Some("Outposts endpoint subnet"),
        (ENDPOINT, "security_group_id") => Some("Outposts endpoint security group"),
        (ENDPOINT, "customer_owned_ipv4_pool") => Some("Customer owned IP pool"),
        _ => None,
    }
}

fn output(kind: &str, path: &str, target: &str) -> Option<&'static [&'static str]> {
    match (kind, path, target) {
        (ACCESS, "bucket", "aws_s3_bucket") => Some(&["id", "bucket"]),
        (ACCESS, "bucket", "aws_s3control_bucket")
        | (
            "aws_s3control_bucket_policy" | "aws_s3control_bucket_lifecycle_configuration",
            "bucket",
            "aws_s3control_bucket",
        ) => Some(&["arn", "id"]),
        (ACCESS, "vpc_configuration.vpc_id", "aws_vpc")
        | (ENDPOINT, "subnet_id", "aws_subnet")
        | (ENDPOINT, "security_group_id", "aws_security_group") => Some(&["id"]),
        ("aws_s3control_access_point_policy", "access_point_arn", ACCESS)
        | (OBJECT, "configuration.supporting_access_point", ACCESS)
        | (OBJECT, FUNCTION, "aws_lambda_function") => Some(&["arn"]),
        ("aws_s3control_object_lambda_access_point_policy", "name", OBJECT) => Some(&["name"]),
        _ => None,
    }
}

fn scalar<'a>(evidence: &[(&'a Value, &Value)], index: usize, path: &str) -> Option<&'a str> {
    let (v, m) = evidence[index];
    super::service_references::scalar(v, m, &path.split('.').collect::<Vec<_>>())
}

pub(crate) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    evidence: &[(&Value, &Value)],
) {
    let function_proofs: Vec<_> = refs
        .iter()
        .map(|r| {
            nodes[r.target].resource_type == OBJECT
                && r.attribute == FUNCTION
                && functions(nodes, refs, evidence, r)
        })
        .collect();
    for (r, proof) in refs.iter_mut().zip(function_proofs) {
        let node = &nodes[r.target];
        if label(&node.resource_type, &r.attribute).is_none() {
            continue;
        }
        r.service_reference_verified = false;
        r.service_reference_owner_account = None;
        let (value, mask) = evidence[r.target];
        if !super::vpc_gateways::absent(value, mask, "account_id") {
            let Some(account) = scalar(evidence, r.target, "account_id")
                .and_then(crate::model::AwsAccountScope::new)
            else {
                continue;
            };
            r.service_reference_owner_account = Some(account);
        }
        if node.resource_type == OBJECT && r.attribute == FUNCTION {
            r.service_reference_verified = proof;
            continue;
        }
        let [s] = r.sources.as_slice() else { continue };
        let endpoint = &nodes[*s];
        let Some(fields) = output(&node.resource_type, &r.attribute, &endpoint.resource_type)
        else {
            continue;
        };
        if super::route53::endpoint(nodes, r, &endpoint.resource_type, fields).is_none()
            || !super::route53::configured(node, endpoint)
        {
            continue;
        }
        let actual = scalar(evidence, r.target, &r.attribute);
        if actual.is_none() || actual != scalar(evidence, *s, r.source_attributes.first().unwrap())
        {
            continue;
        }
        let region = super::scope::scope(node).and_then(|s| s.region);
        let other = super::scope::scope(endpoint).and_then(|s| s.region);
        if region.is_none() || (endpoint.resource_type != "aws_s3_bucket" && region != other) {
            continue;
        }
        let (v, m) = evidence[r.target];
        if node.resource_type == ACCESS
            && r.attribute == "bucket"
            && endpoint.resource_type == "aws_s3control_bucket"
            && scalar(evidence, r.target, "vpc_configuration.vpc_id").is_none()
        {
            continue;
        }
        let account_field = if node.resource_type == ACCESS && r.attribute == "bucket" {
            "bucket_account_id"
        } else {
            "account_id"
        };
        if !super::vpc_gateways::absent(v, m, account_field) {
            let Some(owner) = scalar(evidence, r.target, account_field)
                .and_then(crate::model::AwsAccountScope::new)
            else {
                continue;
            };
            if r.service_reference_owner_account
                .as_ref()
                .is_some_and(|account| account != &owner)
            {
                continue;
            }
            r.service_reference_owner_account = Some(owner);
        }
        if node.resource_type == "aws_s3control_object_lambda_access_point_policy" {
            let owner = scalar(evidence, r.target, "account_id");
            if owner.is_none() || owner != scalar(evidence, *s, "account_id") {
                continue;
            }
        }
        r.service_reference_verified = true;
    }
    let network: Vec<_> = refs
        .iter()
        .filter(|r| nodes[r.target].resource_type == ENDPOINT && r.service_reference_verified)
        .map(|r| (r.target, r.attribute.clone(), r.sources[0]))
        .collect();
    for r in refs.iter_mut().filter(|r| {
        nodes[r.target].resource_type == ENDPOINT
            && matches!(r.attribute.as_str(), "subnet_id" | "security_group_id")
    }) {
        let find = |field: &str| {
            let mut it = network
                .iter()
                .filter(|(t, f, _)| *t == r.target && f == field);
            let first = it.next()?;
            it.next().is_none().then_some(first.2)
        };
        let (Some(subnet), Some(group)) = (find("subnet_id"), find("security_group_id")) else {
            r.service_reference_verified = false;
            continue;
        };
        let vpc = scalar(evidence, subnet, "vpc_id");
        let outpost = scalar(evidence, r.target, "outpost_id");
        let arn = scalar(evidence, subnet, "outpost_arn");
        r.service_reference_verified &= vpc.is_some()
            && vpc == scalar(evidence, group, "vpc_id")
            && outpost.is_some()
            && arn
                .filter(|a| {
                    a.starts_with("arn:")
                        && a.split(':').nth(2) == Some("outposts")
                        && a.split(':').count() == 6
                })
                .and_then(|a| a.rsplit_once(":outpost/").map(|(_, id)| id))
                == outpost;
    }
}

pub(super) fn bindings(raw: &crate::semantic::Input, r: &AttributeReference) -> bool {
    super::route53::bindings(raw, r)
        && r.service_reference_owner_account
            .as_ref()
            .is_none_or(|owner| {
                r.sources.iter().all(|&s| {
                    raw.nodes[s]
                        .provider_configuration
                        .as_ref()
                        .and_then(|c| c.account.as_ref())
                        == Some(owner)
                })
            })
}

#[cfg(test)]
#[path = "../../../tests/unit/semantic/s3_access.rs"]
mod tests;
