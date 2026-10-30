#[cfg(test)]
#[path = "../../../tests/unit/semantic/cloudfront.rs"]
mod tests;

pub(super) fn label(attribute: &str) -> Option<&'static str> {
    match attribute {
        "origin.domain_name" => Some("Origin reference"),
        "origin.origin_access_control_id" => Some("Origin access control reference"),
        "origin.s3_origin_config.origin_access_identity" => {
            Some("Origin access identity reference")
        }
        "default_cache_behavior.cache_policy_id" | "ordered_cache_behavior.cache_policy_id" => {
            Some("Cache policy reference")
        }
        "default_cache_behavior.origin_request_policy_id"
        | "ordered_cache_behavior.origin_request_policy_id" => {
            Some("Origin request policy reference")
        }
        "default_cache_behavior.response_headers_policy_id"
        | "ordered_cache_behavior.response_headers_policy_id" => {
            Some("Response headers policy reference")
        }
        "origin.vpc_origin_config.vpc_origin_id" => Some("VPC origin reference"),
        "origin.custom_origin_config.origin_mtls_config.client_certificate_arn"
        | "viewer_certificate.iam_certificate_id" => Some("Certificate reference"),
        "continuous_deployment_policy_id" => Some("Continuous deployment policy reference"),
        "anycast_ip_list_id" => Some("Anycast IP list reference"),
        "connection_function_association.id" => Some("Connection function reference"),
        "viewer_mtls_config.trust_store_config.trust_store_id" => Some("Trust store reference"),
        "logging_config.bucket" => Some("Logging bucket reference"),
        _ => behavior(attribute).and_then(|field| match field {
            "function_association.function_arn" => Some("Function reference"),
            "lambda_function_association.lambda_arn" => Some("Lambda function reference"),
            "trusted_key_groups" => Some("Key group reference"),
            "field_level_encryption_id" => Some("Field encryption reference"),
            "realtime_log_config_arn" => Some("Realtime log reference"),
            _ => None,
        }),
    }
}

pub(super) fn source_fields(attribute: &str, source: &str) -> &'static [&'static str] {
    match (attribute, source) {
        ("origin.domain_name", "aws_s3_bucket") => {
            &["bucket_regional_domain_name", "bucket_domain_name"]
        }
        ("origin.domain_name", "aws_lb") => &["dns_name"],
        ("origin.origin_access_control_id", "aws_cloudfront_origin_access_control") => &["id"],
        (
            "origin.s3_origin_config.origin_access_identity",
            "aws_cloudfront_origin_access_identity",
        ) => &["cloudfront_access_identity_path"],
        (
            "default_cache_behavior.cache_policy_id" | "ordered_cache_behavior.cache_policy_id",
            "aws_cloudfront_cache_policy",
        )
        | (
            "default_cache_behavior.origin_request_policy_id"
            | "ordered_cache_behavior.origin_request_policy_id",
            "aws_cloudfront_origin_request_policy",
        )
        | (
            "default_cache_behavior.response_headers_policy_id"
            | "ordered_cache_behavior.response_headers_policy_id",
            "aws_cloudfront_response_headers_policy",
        ) => &["id"],
        ("origin.vpc_origin_config.vpc_origin_id", "aws_cloudfront_vpc_origin")
        | ("continuous_deployment_policy_id", "aws_cloudfront_continuous_deployment_policy")
        | ("anycast_ip_list_id", "aws_cloudfront_anycast_ip_list")
        | ("connection_function_association.id", "aws_cloudfront_connection_function")
        | ("viewer_mtls_config.trust_store_config.trust_store_id", "aws_cloudfront_trust_store")
        | ("viewer_certificate.iam_certificate_id", "aws_iam_server_certificate") => &["id"],
        (
            "origin.custom_origin_config.origin_mtls_config.client_certificate_arn",
            "aws_acm_certificate",
        ) => &["arn"],
        ("logging_config.bucket", "aws_s3_bucket") => &["bucket_domain_name"],
        _ => match (behavior(attribute), source) {
            (Some("function_association.function_arn"), "aws_cloudfront_function")
            | (Some("realtime_log_config_arn"), "aws_cloudfront_realtime_log_config") => &["arn"],
            (Some("lambda_function_association.lambda_arn"), "aws_lambda_function") => {
                &["qualified_arn"]
            }
            (Some("trusted_key_groups"), "aws_cloudfront_key_group")
            | (Some("field_level_encryption_id"), "aws_cloudfront_field_level_encryption_config") => {
                &["id"]
            }
            _ => &[],
        },
    }
}

fn behavior(attribute: &str) -> Option<&str> {
    attribute
        .strip_prefix("default_cache_behavior.")
        .or_else(|| attribute.strip_prefix("ordered_cache_behavior."))
}
use crate::model::{Action, AttributeReference, EntityMode, TerraformEntity};
use serde_json::Value;
use std::collections::BTreeSet;

pub(crate) fn resource(kind: &str) -> bool {
    matches!(
        kind,
        "aws_cloudfront_distribution" | "aws_cloudfront_continuous_deployment_policy"
    )
}

pub(super) fn reference_label(kind: &str, path: &str) -> Option<&'static str> {
    match kind {
        "aws_cloudfront_distribution" => label(path),
        "aws_cloudfront_continuous_deployment_policy"
            if path == "staging_distribution_dns_names.items" =>
        {
            Some("Staging distribution reference")
        }
        _ => None,
    }
}

fn normalized(path: &str) -> String {
    path.split('.')
        .filter(|s| s.parse::<usize>().is_err())
        .collect::<Vec<_>>()
        .join(".")
}

pub(crate) fn indexed(kind: &str, path: &str) -> bool {
    let normal = normalized(path);
    normal != path && reference_label(kind, &normal).is_some()
}

fn fields(kind: &str, path: &str, source: &str) -> &'static [&'static str] {
    if kind == "aws_cloudfront_continuous_deployment_policy" {
        if path == "staging_distribution_dns_names.items" && source == "aws_cloudfront_distribution"
        {
            &["domain_name"]
        } else {
            &[]
        }
    } else {
        source_fields(path, source)
    }
}

fn current(node: &TerraformEntity) -> bool {
    node.provider.is_aws()
        && node.mode == EntityMode::Managed
        && node.action != Action::Delete
        && node.deposed_key.is_none()
}

fn compatible(left: &TerraformEntity, right: &TerraformEntity) -> bool {
    let (Some(left), Some(right)) = (&left.provider_configuration, &right.provider_configuration)
    else {
        return false;
    };
    !left.account_conflict
        && !right.account_conflict
        && match (&left.account, &right.account) {
            (Some(a), Some(b)) => a == b,
            (None, None) => left == right,
            _ => false,
        }
}

type Row<'a> = (String, Vec<usize>, &'a Value);
fn rows<'a>(
    value: &'a Value,
    mask: &Value,
    path: &[&str],
    prefix: String,
    lengths: Vec<usize>,
    result: &mut Vec<Row<'a>>,
) -> Option<()> {
    if let Some(values) = value.as_array().filter(|_| !path.is_empty()) {
        if !matches!(
            prefix.rsplit('.').next(),
            Some(
                "origin"
                    | "ordered_cache_behavior"
                    | "function_association"
                    | "lambda_function_association"
            )
        ) && values.len() != 1
        {
            return None;
        }
        if !matches!(mask, Value::Null | Value::Bool(false) | Value::Array(_))
            || mask.as_array().is_some_and(|m| m.len() != values.len())
        {
            return None;
        }
        for (i, value) in values.iter().enumerate() {
            let mut lengths = lengths.clone();
            lengths.push(values.len());
            rows(
                value,
                &mask[i],
                path,
                format!("{prefix}.{i}"),
                lengths,
                result,
            )?;
        }
    } else if let Some((field, rest)) = path.split_first() {
        if !matches!(mask, Value::Null | Value::Bool(false) | Value::Object(_)) {
            return None;
        }
        if value[*field].is_null() {
            return matches!(&mask[*field], Value::Null | Value::Bool(false)).then_some(());
        }
        let prefix = if prefix.is_empty() {
            field.to_string()
        } else {
            format!("{prefix}.{field}")
        };
        rows(&value[*field], &mask[*field], rest, prefix, lengths, result)?;
    } else {
        if !matches!(mask, Value::Null | Value::Bool(false)) {
            return None;
        }
        result.push((prefix, lengths, value));
    }
    Some(())
}

fn verify(
    nodes: &[TerraformEntity],
    refs: &[AttributeReference],
    evidence: &[(&Value, &Value)],
    r: &AttributeReference,
) -> Option<()> {
    let node = &nodes[r.target];
    if !current(node) || !r.issues.is_empty() || !r.instances_explicit {
        return None;
    }
    let (value, mask) = evidence[r.target];
    let mut leaves = Vec::new();
    rows(
        value,
        mask,
        &r.attribute.split('.').collect::<Vec<_>>(),
        String::new(),
        Vec::new(),
        &mut leaves,
    )?;
    if leaves.is_empty() {
        return None;
    }
    let mut sources = BTreeSet::new();
    for (path, lengths, actual) in leaves {
        let reference = if lengths.is_empty() {
            r
        } else {
            let mut found = refs
                .iter()
                .filter(|s| s.target == r.target && s.attribute == path);
            let reference = found.next()?;
            if found.next().is_some() || reference.indexed_block_lengths != lengths {
                return None;
            }
            reference
        };
        if !reference.complete || !reference.instances_explicit || !reference.issues.is_empty() {
            return None;
        }
        let actual: BTreeSet<_> = if let Some(values) = actual.as_array() {
            let values: BTreeSet<_> = values.iter().map(Value::as_str).collect::<Option<_>>()?;
            if values.len() != actual.as_array()?.len() {
                return None;
            }
            values
        } else {
            [actual.as_str()?].into()
        };
        if actual.is_empty() || actual.contains("") {
            return None;
        }
        let mut expected = BTreeSet::new();
        for &source in &reference.sources {
            let endpoint = &nodes[source];
            if source == r.target || !current(endpoint) || !compatible(node, endpoint) {
                return None;
            }
            if r.attribute == "origin.vpc_origin_config.vpc_origin_id" {
                let owner_path = path.replace("vpc_origin_id", "owner_account_id");
                let mut owner = value;
                let mut owner_mask = mask;
                for field in owner_path.split('.') {
                    if owner_mask == &Value::Bool(true) {
                        return None;
                    }
                    if let Ok(index) = field.parse::<usize>() {
                        owner = &owner[index];
                        owner_mask = &owner_mask[index];
                    } else {
                        owner = &owner[field];
                        owner_mask = &owner_mask[field];
                    }
                }
                if !matches!(owner_mask, Value::Null | Value::Bool(false)) {
                    return None;
                }
                if !owner.is_null() && owner.as_str() != Some("") {
                    let account = endpoint.provider_configuration.as_ref()?.account.as_ref()?;
                    if owner.as_str() != Some(account.as_str()) {
                        return None;
                    }
                }
            }
            let outputs = fields(
                &node.resource_type,
                &r.attribute,
                super::aliases::entity_type(endpoint),
            );
            let attrs = reference.source_attributes_by_instance.get(&source)?;
            let outputs_by_instance = attrs.iter().collect::<Vec<_>>();
            let [output] = outputs_by_instance.as_slice() else {
                return None;
            };
            if !outputs.contains(&output.as_str()) {
                return None;
            }
            let expected_value = super::service_evidence::known(evidence, source, output)?;
            if r.attribute == "origin.domain_name"
                && super::aliases::entity_type(endpoint) == "aws_lb"
                && super::service_evidence::known(evidence, source, "load_balancer_type")
                    != Some("application")
            {
                return None;
            }
            if !expected.insert(expected_value) {
                return None;
            }
            sources.insert(source);
        }
        if actual != expected {
            return None;
        }
    }
    if node.resource_type == "aws_cloudfront_continuous_deployment_policy" {
        let block = &value["staging_distribution_dns_names"];
        let [block] = block.as_array()?.as_slice() else {
            return None;
        };
        let quantity_mask = &mask["staging_distribution_dns_names"][0]["quantity"];
        if !matches!(quantity_mask, Value::Null | Value::Bool(false))
            || block["quantity"].as_u64()? != block["items"].as_array()?.len() as u64
        {
            return None;
        }
    }
    (sources == r.sources.iter().copied().collect()).then_some(())
}

pub(crate) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    evidence: &[(&Value, &Value)],
) {
    let results: Vec<_> = refs
        .iter()
        .map(|r| {
            if reference_label(&nodes[r.target].resource_type, &r.attribute).is_some() {
                verify(nodes, refs, evidence, r).is_some()
            } else {
                r.service_reference_verified
            }
        })
        .collect();
    for (r, verified) in refs.iter_mut().zip(results) {
        r.service_reference_verified = verified;
        if verified {
            r.complete = true;
        }
    }
}
