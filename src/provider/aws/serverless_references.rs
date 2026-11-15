use crate::model::{AttributeReference, TerraformEntity};
use serde_json::Value;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/serverless_references.rs"]
mod tests;

pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    match (kind, path) {
        ("aws_opensearchserverless_collection", "collection_group_name") => {
            Some("Collection group")
        }
        ("aws_opensearchserverless_collection", "encryption_config.kms_key_arn") => {
            Some("Collection encryption key")
        }
        (
            "aws_opensearchserverless_security_config",
            "iam_identity_center_options.instance_arn",
        ) => Some("Identity Center instance"),
        _ => None,
    }
}

fn singleton<'a>(value: &'a Value, mask: &Value, block: &str) -> Option<(&'a Value, Value)> {
    let [value] = value.get(block)?.as_array()?.as_slice() else {
        return None;
    };
    if !matches!(mask, Value::Null | Value::Bool(false) | Value::Object(_)) {
        return None;
    }
    let mask = match &mask[block] {
        Value::Null | Value::Bool(false) => Value::Null,
        Value::Array(v) if v.len() == 1 => v[0].clone(),
        _ => return None,
    };
    value.is_object().then_some((value, mask))
}

fn verify(
    nodes: &[TerraformEntity],
    r: &AttributeReference,
    evidence: &[(&Value, &Value)],
) -> bool {
    let (value, mask) = evidence[r.target];
    let identity = r.attribute == "iam_identity_center_options.instance_arn";
    if r.sources.len() != 1 || r.source_attributes.len() != 1 {
        return false;
    }
    let source = r.sources[0];
    if identity {
        if super::service_evidence::known(evidence, r.target, "type") != Some("iamidentitycenter") {
            return false;
        }
        let Some((block, mask)) = singleton(value, mask, "iam_identity_center_options") else {
            return false;
        };
        let Some(actual) = super::streaming::scalar(block, &mask, "instance_arn") else {
            return false;
        };
        return super::identity_center_instance::verify(nodes, r, evidence, actual);
    }
    if !super::service_evidence::eligible(nodes, r) {
        return false;
    }
    let (kind, field, actual) = if r.attribute == "collection_group_name" {
        (
            "aws_opensearchserverless_collection_group",
            "name",
            super::service_evidence::known(evidence, r.target, "collection_group_name"),
        )
    } else {
        let Some((block, mask)) = singleton(value, mask, "encryption_config") else {
            return false;
        };
        if !matches!(&mask["aws_owned_key"], Value::Null | Value::Bool(false))
            || !matches!(&block["aws_owned_key"], Value::Null | Value::Bool(false))
        {
            return false;
        }
        let actual = super::streaming::scalar(block, &mask, "kms_key_arn");
        return nodes[source].resource_type == "aws_kms_key"
            && r.source_attributes.contains("arn")
            && actual.is_some_and(|a| {
                a.contains(":kms:")
                    && a.split(':').nth(5).is_some_and(|s| s.starts_with("key/"))
                    && super::service_evidence::known(evidence, source, "arn") == Some(a)
            });
    };
    nodes[source].resource_type == kind
        && r.source_attributes.contains(field)
        && actual
            .is_some_and(|a| super::service_evidence::known(evidence, source, field) == Some(a))
}

pub(super) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    evidence: &[(&Value, &Value)],
) {
    let valid: Vec<_> = refs
        .iter()
        .map(|r| {
            label(&nodes[r.target].resource_type, &r.attribute).map(|_| {
                super::lambda_core::unique(refs, r.target, &r.attribute).is_some()
                    && verify(nodes, r, evidence)
                    && super::service_references::cluster_binding_matches(
                        nodes[r.target].provider_configuration.as_ref(),
                        nodes[r.sources[0]].provider_configuration.as_ref(),
                        super::scope::scope(&nodes[r.target])
                            .as_ref()
                            .and_then(|s| s.region.as_ref()),
                        super::scope::scope(&nodes[r.sources[0]])
                            .as_ref()
                            .and_then(|s| s.region.as_ref()),
                    )
            })
        })
        .collect();
    for (r, valid) in refs.iter_mut().zip(valid) {
        if let Some(valid) = valid {
            r.service_reference_verified = valid;
        }
    }
}
