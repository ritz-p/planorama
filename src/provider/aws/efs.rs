use crate::model::{Graph, PresentationOwner};
use crate::semantic::Input;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/efs.rs"]
mod tests;

pub(super) fn present(raw: &Input, graph: &mut Graph) {
    for (index, node) in raw.nodes.iter().enumerate() {
        if node.resource_type != "aws_efs_access_point" || !node.provider.is_aws() {
            continue;
        }
        let mut candidates = raw.attributes.iter().filter(|reference| {
            reference.target == index && reference.attribute == "file_system_id"
        });
        let Some(reference) = candidates.next() else {
            continue;
        };
        if candidates.next().is_some() || !super::service_references::verified(raw, reference) {
            continue;
        }
        let [owner] = reference.sources.as_slice() else {
            continue;
        };
        if !node.presentation_scope_matches(&raw.nodes[*owner]) {
            continue;
        }
        graph.nodes[index].member_owner = Some(PresentationOwner {
            target: raw.nodes[*owner].entity.provenance[0].clone(),
            attribute: "file_system_id",
            label: "File system",
            section: "Access points",
        });
    }
}
