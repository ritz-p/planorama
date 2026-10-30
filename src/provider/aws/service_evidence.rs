use crate::model::{Action, AttributeReference, EntityMode, TerraformEntity};
use serde_json::Value;

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
