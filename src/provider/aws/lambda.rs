use crate::model::{Action, AttributeReference, EntityMode, TerraformEntity};
use serde_json::Value;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/lambda.rs"]
mod tests;

pub(super) fn label(resource: &str, attribute: &str) -> Option<&'static str> {
    match (resource, attribute) {
        ("aws_lambda_alias" | "aws_lambda_function_url", "function_name") => {
            Some("Lambda function")
        }
        ("aws_lambda_alias", "function_version") => Some("Published function version"),
        ("aws_lambda_function", "layers") => Some("Layer versions"),
        ("aws_lambda_function_url", "qualifier") => Some("Function URL alias"),
        ("aws_lambda_layer_version_permission", "layer_name" | "version_number") => {
            Some("Permission layer version")
        }
        _ => None,
    }
}

fn current(node: &TerraformEntity) -> bool {
    node.provider.is_aws()
        && node.mode == EntityMode::Managed
        && node.action != Action::Delete
        && node.deposed_key.is_none()
}

fn eligible(nodes: &[TerraformEntity], reference: &AttributeReference) -> bool {
    let node = &nodes[reference.target];
    current(node)
        && reference.complete
        && reference.instances_explicit
        && reference.issues.is_empty()
        && !reference.sources.is_empty()
        && reference.sources.iter().all(|&source| {
            let endpoint = &nodes[source];
            current(endpoint)
                && super::service_references::cluster_binding_matches(
                    node.provider_configuration.as_ref(),
                    endpoint.provider_configuration.as_ref(),
                    super::scope::scope(node)
                        .as_ref()
                        .and_then(|s| s.region.as_ref()),
                    super::scope::scope(endpoint)
                        .as_ref()
                        .and_then(|s| s.region.as_ref()),
                )
        })
}

fn known<'a>(evidence: &[(&'a Value, &Value)], index: usize, key: &str) -> Option<&'a str> {
    let (value, unknown) = evidence[index];
    if !matches!(unknown, Value::Null | Value::Bool(false) | Value::Object(_))
        || !matches!(&unknown[key], Value::Null | Value::Bool(false))
    {
        return None;
    }
    value[key].as_str().filter(|value| !value.is_empty())
}

fn basic(
    nodes: &[TerraformEntity],
    reference: &AttributeReference,
    evidence: &[(&Value, &Value)],
) -> bool {
    if label(&nodes[reference.target].resource_type, &reference.attribute).is_none() {
        return false;
    }
    if !eligible(nodes, reference) || reference.source_attributes.len() != 1 {
        return false;
    }
    let node = &nodes[reference.target];
    if node.resource_type == "aws_lambda_function" && reference.attribute == "layers" {
        let (value, unknown) = evidence[reference.target];
        let Some(values) = value["layers"].as_array() else {
            return false;
        };
        if !matches!(unknown, Value::Null | Value::Bool(false) | Value::Object(_))
            || !reference.source_attributes.contains("arn")
        {
            return false;
        }
        let mask = &unknown["layers"];
        if !matches!(mask, Value::Null | Value::Bool(false))
            && !mask.as_array().is_some_and(|masks| {
                masks.len() == values.len()
                    && masks
                        .iter()
                        .all(|m| matches!(m, Value::Null | Value::Bool(false)))
            })
        {
            return false;
        }
        let Some(mut actual) = values.iter().map(Value::as_str).collect::<Option<Vec<_>>>() else {
            return false;
        };
        let Some(mut expected) = reference
            .sources
            .iter()
            .map(|&source| {
                (nodes[source].resource_type == "aws_lambda_layer_version")
                    .then(|| known(evidence, source, "arn"))
                    .flatten()
            })
            .collect::<Option<Vec<_>>>()
        else {
            return false;
        };
        actual.sort_unstable();
        expected.sort_unstable();
        return !actual.is_empty()
            && actual == expected
            && expected.windows(2).all(|pair| pair[0] != pair[1]);
    }
    let [source] = reference.sources.as_slice() else {
        return false;
    };
    let (resource, fields): (&str, &[&str]) =
        match (node.resource_type.as_str(), reference.attribute.as_str()) {
            ("aws_lambda_alias" | "aws_lambda_function_url", "function_name") => {
                ("aws_lambda_function", &["function_name", "arn"])
            }
            ("aws_lambda_alias", "function_version") => ("aws_lambda_function", &["version"]),
            ("aws_lambda_function_url", "qualifier") => ("aws_lambda_alias", &["name"]),
            ("aws_lambda_layer_version_permission", "layer_name") => (
                "aws_lambda_layer_version",
                &["layer_name", "layer_arn", "arn"],
            ),
            ("aws_lambda_layer_version_permission", "version_number") => {
                let (value, unknown) = evidence[reference.target];
                return nodes[*source].resource_type == "aws_lambda_layer_version"
                    && reference.source_attributes.contains("version")
                    && matches!(unknown, Value::Null | Value::Bool(false) | Value::Object(_))
                    && matches!(&unknown["version_number"], Value::Null | Value::Bool(false))
                    && value["version_number"].as_u64().is_some_and(|version| {
                        version > 0
                            && known(evidence, *source, "version")
                                == Some(version.to_string().as_str())
                    });
            }
            _ => return false,
        };
    nodes[*source].resource_type == resource
        && fields.iter().any(|field| {
            reference.source_attributes.contains(*field)
                && known(evidence, reference.target, &reference.attribute).is_some()
                && known(evidence, reference.target, &reference.attribute)
                    == known(evidence, *source, field)
        })
}

pub(super) fn annotate(
    nodes: &[TerraformEntity],
    references: &mut [AttributeReference],
    evidence: &[(&Value, &Value)],
) {
    let base: Vec<_> = references
        .iter()
        .map(|r| basic(nodes, r, evidence))
        .collect();
    let endpoint = |target: usize, attribute: &str| {
        let mut matches = references
            .iter()
            .enumerate()
            .filter(|(_, r)| r.target == target && r.attribute == attribute);
        let (index, reference) = matches.next()?;
        if matches.next().is_some() || !base[index] {
            return None;
        }
        let [source] = reference.sources.as_slice() else {
            return None;
        };
        Some(*source)
    };
    let verified: Vec<_> = references
        .iter()
        .enumerate()
        .map(|(i, r)| {
            if !base[i] {
                return false;
            }
            match (nodes[r.target].resource_type.as_str(), r.attribute.as_str()) {
                ("aws_lambda_alias", "function_version") => {
                    endpoint(r.target, "function_name") == r.sources.first().copied()
                }
                ("aws_lambda_function_url", "qualifier") => endpoint(r.target, "function_name")
                    .is_some_and(|function| {
                        endpoint(r.sources[0], "function_name") == Some(function)
                    }),
                ("aws_lambda_layer_version_permission", "layer_name") => {
                    endpoint(r.target, "version_number") == r.sources.first().copied()
                }
                ("aws_lambda_layer_version_permission", "version_number") => {
                    endpoint(r.target, "layer_name") == r.sources.first().copied()
                }
                _ => true,
            }
        })
        .collect();
    for (reference, verified) in references.iter_mut().zip(verified) {
        if label(&nodes[reference.target].resource_type, &reference.attribute).is_some() {
            reference.service_reference_verified = verified;
        }
    }
}
