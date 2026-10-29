use crate::model::{AttributeReference, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/elbv2.rs"]
mod tests;

pub(super) fn label(resource: &str, attribute: &str) -> Option<&'static str> {
    match (super::aliases::canonical(resource), attribute) {
        ("aws_lb_target_group_attachment", "target_group_arn") => Some("Attachment target group"),
        ("aws_lb_target_group_attachment", "target_id") => Some("Registered target reference"),
        _ => None,
    }
}

pub(crate) fn annotate(
    nodes: &[TerraformEntity],
    references: &mut [AttributeReference],
    evidence: &[(&Value, &Value)],
) {
    let verified: Vec<_> = references
        .iter()
        .map(|reference| {
            if label(&nodes[reference.target].resource_type, &reference.attribute).is_none() {
                return reference.service_reference_verified;
            }
            let endpoint = |attribute: &str| {
                let mut found = references
                    .iter()
                    .filter(|r| r.target == reference.target && r.attribute == attribute);
                let r = found.next()?;
                if found.next().is_some() || !super::service_evidence::eligible(nodes, r) {
                    return None;
                }
                let [source] = r.sources.as_slice() else {
                    return None;
                };
                (*source != r.target).then_some((r, *source))
            };
            let Some((group_ref, group)) = endpoint("target_group_arn") else {
                return false;
            };
            let Some((target_ref, target)) = endpoint("target_id") else {
                return false;
            };
            let known = |index, field: &str| super::service_evidence::known(evidence, index, field);
            let matches = |r: &AttributeReference, source, fields: &[&str]| {
                fields.iter().any(|field| {
                    r.source_attributes.len() == 1
                        && r.source_attributes.contains(*field)
                        && known(r.target, &r.attribute).is_some()
                        && known(r.target, &r.attribute) == known(source, field)
                })
            };
            if super::aliases::entity_type(&nodes[group]) != "aws_lb_target_group"
                || !matches(group_ref, group, &["arn", "id"])
            {
                return false;
            }
            let kind = super::aliases::entity_type(&nodes[target]);
            match (known(group, "target_type"), kind) {
                (Some("instance"), "aws_instance") => matches(target_ref, target, &["id"]),
                (Some("lambda"), "aws_lambda_function") => matches(target_ref, target, &["arn"]),
                (Some("alb"), "aws_lb") => {
                    known(target, "load_balancer_type") == Some("application")
                        && matches(target_ref, target, &["arn"])
                }
                _ => false,
            }
        })
        .collect();
    for (reference, verified) in references.iter_mut().zip(verified) {
        reference.service_reference_verified = verified;
    }
}

pub(super) fn association(raw: &Input, index: usize) -> Option<(usize, usize)> {
    let endpoint = |field| {
        let mut found = raw
            .attributes
            .iter()
            .filter(|r| r.target == index && r.attribute == field);
        let r = found.next()?;
        if found.next().is_some()
            || !r.complete
            || !r.instances_explicit
            || !r.issues.is_empty()
            || !super::service_references::verified(raw, r)
        {
            return None;
        }
        let [source] = r.sources.as_slice() else {
            return None;
        };
        Some(*source)
    };
    endpoint("target_group_arn").zip(endpoint("target_id"))
}
