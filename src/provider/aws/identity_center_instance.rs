use crate::model::{Action, AttributeReference, EntityMode, TerraformEntity};
use serde_json::Value;

pub(super) fn verify(
    nodes: &[TerraformEntity],
    r: &AttributeReference,
    evidence: &[(&Value, &Value)],
    actual: &str,
) -> bool {
    let [source] = r.sources.as_slice() else {
        return false;
    };
    let node = &nodes[r.target];
    let endpoint = &nodes[*source];
    if actual.is_empty()
        || !r.complete
        || !r.instances_explicit
        || !r.issues.is_empty()
        || r.source_attributes.len() != 1
        || !r
            .source_attributes
            .iter()
            .all(|f| f == "arns" || f == "arns[0]")
        || !node.provider.is_aws()
        || !endpoint.provider.is_aws()
        || node.mode != EntityMode::Managed
        || endpoint.mode != EntityMode::Data
        || node.action == Action::Delete
        || endpoint.action == Action::Delete
        || node.deposed_key.is_some()
        || endpoint.deposed_key.is_some()
        || endpoint.resource_type != "aws_ssoadmin_instances"
    {
        return false;
    }
    let (v, m) = evidence[*source];
    super::streaming::values(v, m, "arns", true)
        .is_some_and(|values| values.len() == 1 && values.contains(actual))
}
