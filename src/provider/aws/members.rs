use crate::model::{EntityMode, Graph};
use crate::semantic::Input;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/members.rs"]
mod tests;

pub(super) fn present(raw: &Input, graph: &mut Graph) {
    for (index, node) in raw.nodes.iter().enumerate() {
        if node.resource_type != "aws_rds_cluster_instance"
            || !node.provider.is_aws()
            || node.mode != EntityMode::Managed
            || node.deposed_key.is_some()
        {
            continue;
        }
        let mut references = raw
            .attributes
            .iter()
            .filter(|r| r.target == index && r.attribute == "cluster_identifier");
        let Some(reference) = references.next() else {
            continue;
        };
        if references.next().is_some()
            || !reference.complete
            || !reference.instances_explicit
            || !reference.issues.is_empty()
        {
            continue;
        }
        let [owner] = reference.sources.as_slice() else {
            continue;
        };
        let parent = &raw.nodes[*owner];
        if parent.resource_type == "aws_rds_cluster"
            && parent.provider.is_aws()
            && parent.mode == EntityMode::Managed
            && parent.deposed_key.is_none()
            && parent.entity.scope.is_some()
            && parent.entity.scope == node.entity.scope
        {
            graph.nodes[index].member_owner = parent.entity.provenance.first().cloned();
        }
    }
}
