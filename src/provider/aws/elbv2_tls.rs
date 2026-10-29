use crate::model::{Action, AttributeReference, EntityMode, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/elbv2_tls.rs"]
mod tests;

pub(super) fn label(resource: &str, attribute: &str) -> Option<&'static str> {
    match (super::aliases::canonical(resource), attribute) {
        ("aws_lb_listener" | "aws_lb_listener_certificate", "certificate_arn") => {
            Some("Certificate reference")
        }
        ("aws_lb_listener_certificate", "listener_arn") => Some("Certificate listener"),
        ("aws_lb_listener", "mutual_authentication.trust_store_arn") => {
            Some("Verify-mode trust store")
        }
        ("aws_lb_trust_store_revocation", "trust_store_arn") => Some("Revocation trust store"),
        ("aws_lb_trust_store", "ca_certificates_bundle_s3_bucket") => Some("CA bundle bucket"),
        ("aws_lb_trust_store", "ca_certificates_bundle_s3_key") => Some("CA bundle object"),
        ("aws_lb_trust_store", "ca_certificates_bundle_s3_object_version") => {
            Some("CA bundle object version")
        }
        ("aws_lb_trust_store_revocation", "revocations_s3_bucket") => {
            Some("Revocation source bucket")
        }
        ("aws_lb_trust_store_revocation", "revocations_s3_key") => Some("Revocation source object"),
        ("aws_lb_trust_store_revocation", "revocations_s3_object_version") => {
            Some("Revocation source object version")
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

fn compatible(node: &TerraformEntity, endpoint: &TerraformEntity) -> bool {
    super::eventbridge::bindings(
        node.provider_configuration.as_ref(),
        endpoint.provider_configuration.as_ref(),
        super::scope::scope(node)
            .as_ref()
            .and_then(|s| s.region.as_ref()),
        super::scope::scope(endpoint)
            .as_ref()
            .and_then(|s| s.region.as_ref()),
        endpoint.resource_type == "aws_iam_server_certificate",
    )
}

pub(super) fn bindings(raw: &Input, r: &AttributeReference) -> bool {
    let node = &raw.nodes[r.target];
    r.sources.iter().all(|&source| {
        let endpoint = &raw.nodes[source];
        super::eventbridge::bindings(
            node.provider_configuration.as_ref(),
            endpoint.provider_configuration.as_ref(),
            node.entity.scope.as_ref().and_then(|s| s.region.as_ref()),
            endpoint
                .entity
                .scope
                .as_ref()
                .and_then(|s| s.region.as_ref()),
            endpoint.resource_type == "aws_iam_server_certificate",
        )
    })
}

fn endpoint<'a>(
    nodes: &[TerraformEntity],
    refs: &'a [AttributeReference],
    target: usize,
    field: &str,
) -> Option<(&'a AttributeReference, usize)> {
    let mut found = refs
        .iter()
        .filter(|r| r.target == target && r.attribute == field);
    let r = found.next()?;
    let [source] = r.sources.as_slice() else {
        return None;
    };
    (found.next().is_none()
        && r.complete
        && r.instances_explicit
        && r.issues.is_empty()
        && *source != target
        && current(&nodes[target])
        && current(&nodes[*source])
        && compatible(&nodes[target], &nodes[*source]))
    .then_some((r, *source))
}

fn scalar<'a>(value: &'a Value, mask: &Value, path: &[&str]) -> Option<&'a str> {
    if let Some(values) = value.as_array() {
        let [value] = values.as_slice() else {
            return None;
        };
        let mask = match mask {
            Value::Null | Value::Bool(false) => &Value::Null,
            Value::Array(masks) if masks.len() == 1 => &masks[0],
            _ => return None,
        };
        return scalar(value, mask, path);
    }
    if let Some((field, rest)) = path.split_first() {
        if !matches!(mask, Value::Null | Value::Bool(false) | Value::Object(_)) {
            return None;
        }
        return scalar(&value[*field], &mask[*field], rest);
    }
    matches!(mask, Value::Null | Value::Bool(false))
        .then(|| value.as_str().filter(|s| !s.is_empty()))
        .flatten()
}

fn matches(
    evidence: &[(&Value, &Value)],
    r: &AttributeReference,
    source: usize,
    fields: &[&str],
) -> bool {
    let (value, mask) = evidence[r.target];
    let actual = scalar(value, mask, &r.attribute.split('.').collect::<Vec<_>>());
    fields.iter().any(|field| {
        r.source_attributes.len() == 1
            && r.source_attributes.contains(*field)
            && actual.is_some()
            && actual == super::service_evidence::known(evidence, source, field)
    })
}

fn verify(
    nodes: &[TerraformEntity],
    refs: &[AttributeReference],
    r: &AttributeReference,
    evidence: &[(&Value, &Value)],
) -> bool {
    let Some((_, source)) = endpoint(nodes, refs, r.target, &r.attribute) else {
        return false;
    };
    let kind = super::aliases::entity_type(&nodes[source]);
    let resource = super::aliases::entity_type(&nodes[r.target]);
    match (resource, r.attribute.as_str()) {
        ("aws_lb_listener" | "aws_lb_listener_certificate", "certificate_arn") => {
            (kind == "aws_acm_certificate" && matches(evidence, r, source, &["arn", "id"]))
                || (kind == "aws_iam_server_certificate" && matches(evidence, r, source, &["arn"]))
                || (resource == "aws_lb_listener"
                    && kind == "aws_acm_certificate_validation"
                    && matches(evidence, r, source, &["certificate_arn"]))
        }
        ("aws_lb_listener_certificate", "listener_arn") => {
            kind == "aws_lb_listener" && matches(evidence, r, source, &["arn"])
        }
        ("aws_lb_listener", "mutual_authentication.trust_store_arn") => {
            let (value, mask) = evidence[r.target];
            kind == "aws_lb_trust_store"
                && value["mutual_authentication"]
                    .as_array()
                    .is_some_and(|v| v.len() == 1)
                && scalar(value, mask, &["mutual_authentication", "mode"]) == Some("verify")
                && matches(evidence, r, source, &["arn", "id"])
        }
        ("aws_lb_trust_store_revocation", "trust_store_arn") => {
            kind == "aws_lb_trust_store" && matches(evidence, r, source, &["arn", "id"])
        }
        ("aws_lb_trust_store" | "aws_lb_trust_store_revocation", field) => {
            let prefix = if resource == "aws_lb_trust_store" {
                "ca_certificates_bundle_s3"
            } else {
                "revocations_s3"
            };
            if field == format!("{prefix}_bucket") {
                return kind == "aws_s3_bucket" && matches(evidence, r, source, &["bucket", "id"]);
            }
            if !matches!(kind, "aws_s3_object" | "aws_s3_bucket_object") {
                return false;
            }
            let Some((bucket_ref, bucket)) =
                endpoint(nodes, refs, r.target, &format!("{prefix}_bucket"))
            else {
                return false;
            };
            if nodes[bucket].resource_type != "aws_s3_bucket"
                || !matches(evidence, bucket_ref, bucket, &["bucket", "id"])
            {
                return false;
            }
            let known = |index, key: &str| super::service_evidence::known(evidence, index, key);
            if known(source, "bucket") != known(r.target, &format!("{prefix}_bucket")) {
                return false;
            }
            let Some((key_ref, object)) = endpoint(nodes, refs, r.target, &format!("{prefix}_key"))
            else {
                return false;
            };
            if object != source || !matches(evidence, key_ref, object, &["key"]) {
                return false;
            }
            let version = format!("{prefix}_object_version");
            let (value, mask) = evidence[r.target];
            if value[&version].is_null()
                && matches!(mask, Value::Null | Value::Bool(false) | Value::Object(_))
                && matches!(&mask[&version], Value::Null | Value::Bool(false))
            {
                return field == format!("{prefix}_key");
            }
            endpoint(nodes, refs, r.target, &version).is_some_and(|(version_ref, object)| {
                object == source && matches(evidence, version_ref, object, &["version_id"])
            })
        }
        _ => false,
    }
}

pub(super) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    evidence: &[(&Value, &Value)],
) {
    let values: Vec<_> = refs
        .iter()
        .map(|r| {
            if label(&nodes[r.target].resource_type, &r.attribute).is_some() {
                verify(nodes, refs, r, evidence)
            } else {
                r.service_reference_verified
            }
        })
        .collect();
    for (r, verified) in refs.iter_mut().zip(values) {
        r.service_reference_verified = verified;
    }
}

pub(super) fn association(raw: &Input, index: usize) -> Option<(usize, usize)> {
    let get = |field| {
        let mut found = raw
            .attributes
            .iter()
            .filter(|r| r.target == index && r.attribute == field);
        let r = found.next()?;
        let [source] = r.sources.as_slice() else {
            return None;
        };
        (found.next().is_none()
            && r.complete
            && r.instances_explicit
            && r.issues.is_empty()
            && super::service_references::verified(raw, r))
        .then_some(*source)
    };
    get("listener_arn").zip(get("certificate_arn"))
}
