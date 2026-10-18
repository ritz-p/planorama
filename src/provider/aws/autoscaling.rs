use crate::model::{
    Diagnostic, DiagnosticReason as Reason, EntityMode, Graph, ReferenceTarget, ResourceReference,
};
use crate::semantic::Input;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/autoscaling.rs"]
mod tests;

pub(super) const ATTRIBUTE: &str = "autoscaling_group_name";

pub(super) fn target(raw: &Input, index: usize) -> Result<usize, Reason> {
    let policy = &raw.nodes[index];
    let reference = raw
        .attributes
        .iter()
        .find(|reference| reference.target == index && reference.attribute == ATTRIBUTE)
        .ok_or(Reason::MissingAttribute)?;

    if reference.issues.contains(&Reason::DynamicInstanceSelection) {
        return Err(Reason::DynamicInstanceSelection);
    }
    if !reference.complete {
        return Err(if reference.sources.is_empty() {
            Reason::NoResourceReference
        } else {
            Reason::PartialResourceProvenance
        });
    }
    if reference.sources.len() != 1 {
        return Err(Reason::MultipleMatchingInstances);
    }
    if !reference.scalar_name_matches {
        return Err(Reason::UnprovenEndpointValue);
    }

    let source = reference.sources[0];
    let group = &raw.nodes[source];
    if source == index
        || !group.provider.is_aws()
        || group.resource_type != "aws_autoscaling_group"
        || group.mode != EntityMode::Managed
    {
        return Err(Reason::EndpointTypeMismatch);
    }
    if group.provider_configuration != policy.provider_configuration {
        return Err(Reason::ProviderConfigurationMismatch);
    }
    if group.deposed_key.is_some() {
        return Err(Reason::DeposedEndpoint);
    }
    Ok(source)
}

pub(super) fn diagnostic(raw: &Input, index: usize) -> Option<Diagnostic> {
    let policy = &raw.nodes[index];
    if !policy.provider.is_aws()
        || policy.resource_type != "aws_autoscaling_policy"
        || policy.mode != EntityMode::Managed
        || policy.deposed_key.is_some()
    {
        return None;
    }
    target(raw, index).err().map(|reason| Diagnostic {
        deposed_key: policy.deposed_key.clone(),
        address: policy.address.clone(),
        attribute: ATTRIBUTE.into(),
        reason,
    })
}

pub(super) fn present(raw: &Input, graph: &mut Graph) {
    for (index, policy) in raw.nodes.iter().enumerate() {
        if !policy.provider.is_aws()
            || policy.resource_type != "aws_autoscaling_policy"
            || policy.mode != EntityMode::Managed
            || policy.deposed_key.is_some()
        {
            continue;
        }

        let source = target(raw, index).ok();
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
            kind: crate::model::ReferenceKind::Configured,
            show_spatial: false,
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
