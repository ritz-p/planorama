use crate::model::{
    DiagnosticReason, EntityMode, Graph, ReferenceTarget, ResourceReference,
};
use crate::semantic::Input;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/autoscaling.rs"]
mod tests;

const ATTRIBUTE: &str = "autoscaling_group_name";

pub(super) fn present(raw: &Input, graph: &mut Graph) {
    for (index, policy) in raw.nodes.iter().enumerate() {
        if !policy.provider.is_aws()
            || policy.resource_type != "aws_autoscaling_policy"
            || policy.mode != EntityMode::Managed
            || policy.deposed_key.is_some()
        {
            continue;
        }

        let source = raw
            .attributes
            .iter()
            .find(|reference| reference.target == index && reference.attribute == ATTRIBUTE)
            .and_then(|reference| {
                if !reference.complete
                    || reference.sources.len() != 1
                    || reference
                        .issues
                        .contains(&DiagnosticReason::DynamicInstanceSelection)
                {
                    return None;
                }
                let source = reference.sources[0];
                let group = &raw.nodes[source];
                (source != index
                    && group.provider.is_aws()
                    && group.resource_type == "aws_autoscaling_group"
                    && group.mode == EntityMode::Managed
                    && group.deposed_key.is_none()
                    && group.provider_configuration == policy.provider_configuration)
                    .then_some(source)
            });

        for reference in graph.nodes[index]
            .references
            .iter_mut()
            .filter(|reference| !reference.dependency && reference.attribute == ATTRIBUTE)
        {
            reference.label = "Auto Scaling group";
            if source.is_none() {
                reference.complete = false;
            }
        }

        let Some(source) = source else {
            continue;
        };
        let inverse = ResourceReference {
            label: "Scaling policy",
            attribute: ATTRIBUTE.into(),
            targets: vec![ReferenceTarget {
                state: None,
                id: policy.entity.provenance[0].clone(),
                placement: policy.placement,
            }],
            complete: true,
            dependency: false,
            issues: Default::default(),
        };
        if !graph.nodes[source].references.contains(&inverse) {
            graph.nodes[source].references.push(inverse);
        }
    }
}
