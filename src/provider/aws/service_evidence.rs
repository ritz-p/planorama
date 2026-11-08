use crate::model::{Action, AttributeReference, EntityMode, TerraformEntity};
use serde_json::Value;

pub(super) fn annotate_scalar(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    evidence: &[(&Value, &Value)],
    label: fn(&str, &str) -> Option<&'static str>,
    endpoint: fn(&str, &str, &str, &str) -> bool,
) {
    let mut counts = std::collections::BTreeMap::new();
    for r in refs.iter() {
        *counts
            .entry((r.target, r.attribute.clone()))
            .or_insert(0usize) += 1;
    }
    for r in refs {
        let kind = nodes[r.target].resource_type.as_str();
        if label(kind, &r.attribute).is_none() {
            continue;
        }
        r.service_reference_verified = false;
        if counts[&(r.target, r.attribute.clone())] != 1
            || !eligible(nodes, r)
            || r.sources.len() != 1
            || r.source_attributes.len() != 1
        {
            continue;
        }
        let source = r.sources[0];
        let field = r.source_attributes.first().unwrap().as_str();
        if endpoint(kind, &r.attribute, &nodes[source].resource_type, field) {
            r.service_reference_verified = known(evidence, r.target, &r.attribute)
                .is_some_and(|value| known(evidence, source, field) == Some(value));
        }
    }
}

fn current(node: &TerraformEntity) -> bool {
    node.provider.is_aws()
        && node.mode == EntityMode::Managed
        && node.action != Action::Delete
        && node.deposed_key.is_none()
}

pub(super) fn eligible(nodes: &[TerraformEntity], reference: &AttributeReference) -> bool {
    eligible_with_global_role(nodes, reference, false)
}

pub(super) fn eligible_with_global_role(
    nodes: &[TerraformEntity],
    reference: &AttributeReference,
    global_role: bool,
) -> bool {
    let node = &nodes[reference.target];
    current(node)
        && reference.complete
        && reference.instances_explicit
        && reference.issues.is_empty()
        && !reference.sources.is_empty()
        && reference.sources.iter().all(|&source| {
            let endpoint = &nodes[source];
            current(endpoint)
                && if global_role && endpoint.resource_type == "aws_iam_role" {
                    super::eventbridge::bindings(
                        node.provider_configuration.as_ref(),
                        endpoint.provider_configuration.as_ref(),
                        super::scope::scope(node)
                            .as_ref()
                            .and_then(|s| s.region.as_ref()),
                        super::scope::scope(endpoint)
                            .as_ref()
                            .and_then(|s| s.region.as_ref()),
                        true,
                    )
                } else {
                    super::service_references::cluster_binding_matches(
                        node.provider_configuration.as_ref(),
                        endpoint.provider_configuration.as_ref(),
                        super::scope::scope(node)
                            .as_ref()
                            .and_then(|s| s.region.as_ref()),
                        super::scope::scope(endpoint)
                            .as_ref()
                            .and_then(|s| s.region.as_ref()),
                    )
                }
        })
}

pub(super) fn known<'a>(
    evidence: &[(&'a Value, &Value)],
    index: usize,
    key: &str,
) -> Option<&'a str> {
    let (value, unknown) = evidence[index];
    if !matches!(unknown, Value::Null | Value::Bool(false) | Value::Object(_))
        || !matches!(&unknown[key], Value::Null | Value::Bool(false))
    {
        return None;
    }
    value[key].as_str().filter(|value| !value.is_empty())
}
