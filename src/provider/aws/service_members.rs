use crate::model::{Graph, PresentationOwner};
use crate::semantic::Input;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/kms.rs"]
mod kms_tests;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/secrets_manager.rs"]
mod secrets_manager_tests;

pub(super) fn present(raw: &Input, graph: &mut Graph) {
    for (index, node) in raw.nodes.iter().enumerate() {
        if !node.provider.is_aws() {
            continue;
        }
        let (attribute, label, section) = match node.resource_type.as_str() {
            "aws_kms_alias" => ("target_key_id", "Alias key", "Aliases"),
            "aws_secretsmanager_secret_version" => ("secret_id", "Version secret", "Versions"),
            _ => continue,
        };
        let mut candidates = raw
            .attributes
            .iter()
            .filter(|reference| reference.target == index && reference.attribute == attribute);
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
            attribute,
            label,
            section,
        });
    }
}
