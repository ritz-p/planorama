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
            || !reference.ids_only
            || !reference.scalar_id_matches
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
            && parent.entity.scope.as_ref().is_some_and(|scope| {
                scope.region.is_some()
                    && (scope.account.is_some()
                        || parent
                            .provider_configuration
                            .as_ref()
                            .is_some_and(|configuration| {
                                !configuration.account_conflict
                                    && node.provider_configuration.as_ref() == Some(configuration)
                            }))
            })
            && parent.entity.scope == node.entity.scope
        {
            graph.nodes[index].member_owner = parent.entity.provenance.first().cloned();
        }
    }
}
