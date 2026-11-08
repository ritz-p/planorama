use crate::model::{AttributeReference, TerraformEntity};
use serde_json::Value;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/secondary_network.rs"]
mod tests;

pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    (kind == "aws_ec2_secondary_subnet" && path == "secondary_network_id")
        .then_some("Secondary network")
}

pub(crate) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    evidence: &[(&Value, &Value)],
) {
    super::service_evidence::annotate_scalar(nodes, refs, evidence, label, |_, _, kind, field| {
        kind == "aws_ec2_secondary_network" && field == "secondary_network_id"
    });
    for r in refs {
        if label(&nodes[r.target].resource_type, &r.attribute).is_none() {
            continue;
        }
        r.service_reference_verified &= r.sources.len() == 1
            && r.sources.iter().all(|&source| {
                source != r.target
                    && super::service_evidence::known(evidence, source, "network_type")
                        == Some("rdma")
            });
    }
}
