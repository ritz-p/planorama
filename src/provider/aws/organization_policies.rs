use super::organizations::{eligible, root_index, root_value};
use super::service_references::scalar;
use crate::model::{
    AttributeReference, Diagnostic, DiagnosticReason, Graph, ReferenceKind, ReferenceTarget,
    ResourceReference, TerraformEntity, TerraformPlan,
};
use crate::semantic::Input;
use serde_json::Value;
use std::collections::BTreeSet;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/organization_policies.rs"]
mod tests;

fn policy_label(value: Option<&str>) -> &'static str {
    match value {
        Some("SERVICE_CONTROL_POLICY") => "Policy type: SERVICE_CONTROL_POLICY (SCP)",
        Some("TAG_POLICY") => "Policy type: TAG_POLICY",
        Some("BACKUP_POLICY") => "Policy type: BACKUP_POLICY",
        Some("AISERVICES_OPT_OUT_POLICY") => "Policy type: AISERVICES_OPT_OUT_POLICY",
        Some("RESOURCE_CONTROL_POLICY") => "Policy type: RESOURCE_CONTROL_POLICY",
        Some("CHATBOT_POLICY") => "Policy type: CHATBOT_POLICY",
        Some("DECLARATIVE_POLICY_EC2") => "Policy type: DECLARATIVE_POLICY_EC2",
        Some("SECURITYHUB_POLICY") => "Policy type: SECURITYHUB_POLICY",
        _ => "Policy type: unknown or unsupported",
    }
}

pub(crate) fn annotate(
    nodes: &mut [TerraformEntity],
    references: &mut [AttributeReference],
    evidence: &[(&Value, &Value)],
) {
    for (index, node) in nodes.iter_mut().enumerate() {
        if eligible(node) && node.resource_type == "aws_organizations_policy" {
            let (value, unknown) = evidence[index];
            node.organization_policy_type = Some(policy_label(scalar(value, unknown, &["type"])));
        }
    }
    for reference in references.iter_mut() {
        let node = &nodes[reference.target];
        if node.resource_type != "aws_organizations_policy_attachment" || !eligible(node) {
            continue;
        }
        let [source] = reference.sources.as_slice() else {
            continue;
        };
        let endpoint = &nodes[*source];
        if !eligible(endpoint)
            || !reference.complete
            || !reference.instances_explicit
            || !reference.issues.is_empty()
        {
            continue;
        }
        let (value, unknown) = evidence[reference.target];
        let actual = scalar(value, unknown, &[&reference.attribute]);
        let (value, unknown) = evidence[*source];
        let expected = match (
            reference.attribute.as_str(),
            endpoint.resource_type.as_str(),
        ) {
            ("policy_id", "aws_organizations_policy")
            | (
                "target_id",
                "aws_organizations_organizational_unit" | "aws_organizations_account",
            ) if reference.source_attributes == BTreeSet::from(["id".into()]) => {
                scalar(value, unknown, &["id"])
            }
            ("target_id", "aws_organizations_organization") => {
                let indices: Vec<_> = reference
                    .source_attributes
                    .iter()
                    .filter_map(|path| root_index(path))
                    .collect();
                match indices.as_slice() {
                    [index]
                        if reference.source_attributes.iter().all(|path| {
                            root_index(path) == Some(*index)
                                || path == "roots"
                                || path == &format!("roots[{index}]")
                        }) =>
                    {
                        root_value(value, unknown, *index)
                    }
                    _ => None,
                }
            }
            _ => None,
        };
        reference.service_reference_verified = actual.is_some() && actual == expected;
    }
}

pub(crate) fn facts(plan: &TerraformPlan, graph: &mut Graph) {
    for (fact, node) in plan.nodes.iter().zip(&mut graph.nodes) {
        if !fact.provider.is_aws() || fact.resource_type != "aws_organizations_policy" {
            continue;
        }
        node.references.push(ResourceReference {
            kind: ReferenceKind::Information,
            preserve_spatial: false,
            show_spatial: true,
            label: fact
                .organization_policy_type
                .unwrap_or("Policy type: unknown or unsupported"),
            attribute: "type".into(),
            targets: Vec::new(),
            complete: true,
            dependency: false,
            issues: BTreeSet::new(),
        });
    }
}

pub(super) fn present(raw: &Input, graph: &mut Graph) {
    for (index, node) in raw.nodes.iter().enumerate() {
        if !node.provider.is_aws() || node.resource_type != "aws_organizations_policy_attachment" {
            continue;
        }
        let Some(target) = graph
            .nodes
            .iter_mut()
            .find(|target| target.entity.id == node.entity.id)
        else {
            continue;
        };
        for (attribute, label) in [
            ("policy_id", "Direct attachment policy"),
            ("target_id", "Direct attachment target"),
        ] {
            let reference = raw
                .attributes
                .iter()
                .find(|r| r.target == index && r.attribute == attribute);
            let verified = reference.is_some_and(|r| r.service_reference_verified);
            let root = reference
                .filter(|_| verified)
                .and_then(|r| r.source_attributes.iter().find_map(|path| root_index(path)));
            if verified {
                target
                    .references
                    .retain(|r| r.dependency || r.attribute != attribute);
            }
            target.references.push(ResourceReference {
                kind: root.map_or(
                    ReferenceKind::PolicyAttachment,
                    ReferenceKind::OrganizationRoot,
                ),
                preserve_spatial: false,
                show_spatial: true,
                label,
                attribute: attribute.into(),
                targets: reference
                    .filter(|_| verified)
                    .into_iter()
                    .flat_map(|r| r.sources.iter())
                    .map(|source| ReferenceTarget {
                        state: None,
                        id: raw.nodes[*source].entity.provenance[0].clone(),
                        placement: raw.nodes[*source].placement,
                    })
                    .collect(),
                complete: verified,
                dependency: false,
                issues: if verified {
                    BTreeSet::new()
                } else {
                    [DiagnosticReason::UnprovenEndpointValue].into()
                },
            });
        }
    }
}

pub(super) fn diagnostics(raw: &Input) -> Vec<Diagnostic> {
    raw.nodes
        .iter()
        .enumerate()
        .filter(|(_, node)| {
            node.provider.is_aws() && node.resource_type == "aws_organizations_policy_attachment"
        })
        .flat_map(|(index, node)| {
            ["policy_id", "target_id"]
                .into_iter()
                .filter(move |attribute| {
                    !raw.attributes.iter().any(|r| {
                        r.target == index
                            && r.attribute == *attribute
                            && r.service_reference_verified
                    })
                })
                .map(move |attribute| Diagnostic {
                    address: node.address.clone(),
                    deposed_key: node.deposed_key.clone(),
                    attribute: attribute.into(),
                    reason: DiagnosticReason::UnprovenEndpointValue,
                })
        })
        .collect()
}
