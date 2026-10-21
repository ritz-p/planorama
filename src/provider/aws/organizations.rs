use crate::model::{
    Action, ArchitectureEntity, AttributeReference, Diagnostic, DiagnosticReason, EntityMode,
    Graph, ReferenceKind, ReferenceTarget, ResourceReference, SyntheticComponent, TerraformEntity,
    TerraformPlan,
};
use crate::semantic::Input;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

#[cfg(test)]
#[path = "../../../tests/unit/semantic/organizations.rs"]
mod tests;

pub(super) fn resource(kind: &str) -> bool {
    matches!(
        kind,
        "aws_organizations_organization"
            | "aws_organizations_organizational_unit"
            | "aws_organizations_account"
    )
}

pub(crate) fn unsupported_data(
    provider: &crate::model::ProviderIdentity,
    kind: &str,
    mode: EntityMode,
) -> bool {
    provider.is_aws()
        && mode == EntityMode::Data
        && matches!(
            kind,
            "aws_organizations_account" | "aws_organizations_policy_attachment"
        )
}

pub(super) fn eligible(node: &TerraformEntity) -> bool {
    node.provider.is_aws()
        && node.deposed_key.is_none()
        && node.action != Action::Delete
        && (node.mode == EntityMode::Managed
            || matches!(
                node.resource_type.as_str(),
                "aws_organizations_organization"
                    | "aws_organizations_organizational_unit"
                    | "aws_organizations_policy"
            ))
}

pub(crate) fn root_index(path: &str) -> Option<usize> {
    let index = path.strip_prefix("roots[")?.strip_suffix("].id")?;
    let parsed = index.parse::<usize>().ok()?;
    (parsed.to_string() == index).then_some(parsed)
}

pub(super) fn root_value<'a>(value: &'a Value, unknown: &Value, index: usize) -> Option<&'a str> {
    if unknown == &Value::Bool(true) || unknown["roots"] == Value::Bool(true) {
        return None;
    }
    super::service_references::scalar(&value["roots"][index], &unknown["roots"][index], &["id"])
}

pub(crate) fn annotate(
    nodes: &mut [TerraformEntity],
    references: &mut [AttributeReference],
    evidence: &[(&Value, &Value)],
) {
    for (index, node) in nodes.iter_mut().enumerate() {
        if eligible(node) && node.resource_type == "aws_organizations_organization" {
            let (value, unknown) = evidence[index];
            node.organization_roots = value["roots"]
                .as_array()
                .into_iter()
                .flatten()
                .enumerate()
                .filter_map(|(index, _)| root_value(value, unknown, index).map(|_| index))
                .collect();
        }
    }
    for reference in references.iter_mut() {
        let node = &nodes[reference.target];
        if !matches!(
            node.resource_type.as_str(),
            "aws_organizations_organizational_unit" | "aws_organizations_account"
        ) || reference.attribute != "parent_id"
        {
            continue;
        }
        let [source] = reference.sources.as_slice() else {
            continue;
        };
        let parent = &nodes[*source];
        if !eligible(node)
            || !eligible(parent)
            || !reference.complete
            || !reference.instances_explicit
            || !reference.issues.is_empty()
        {
            continue;
        }
        let (value, unknown) = evidence[reference.target];
        let actual = super::service_references::scalar(value, unknown, &["parent_id"]);
        let (value, unknown) = evidence[*source];
        let expected = match parent.resource_type.as_str() {
            "aws_organizations_organizational_unit"
                if reference.source_attributes == BTreeSet::from(["id".into()]) =>
            {
                super::service_references::scalar(value, unknown, &["id"])
            }
            "aws_organizations_organization" => {
                let paths: Vec<_> = reference
                    .source_attributes
                    .iter()
                    .filter_map(|path| root_index(path))
                    .collect();
                match paths.as_slice() {
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
    let parents: BTreeMap<_, _> = references
        .iter()
        .filter(|r| {
            r.attribute == "parent_id"
                && r.service_reference_verified
                && resource(&nodes[r.target].resource_type)
        })
        .map(|r| (r.target, r.sources[0]))
        .collect();
    for reference in references
        .iter_mut()
        .filter(|r| r.attribute == "parent_id" && resource(&nodes[r.target].resource_type))
    {
        let mut visited = BTreeSet::new();
        let mut current = reference.target;
        while let Some(parent) = parents.get(&current) {
            if !visited.insert(current) {
                reference.service_reference_verified = false;
                reference.issues.insert(DiagnosticReason::AliasCycle);
                break;
            }
            current = *parent;
        }
    }
}

pub(crate) fn root_id(address: &str, index: usize) -> crate::model::ArchitectureId {
    crate::model::ArchitectureId::synthetic(
        "organization_root",
        &format!("{address}.roots[{index}]"),
    )
}

pub(crate) fn roots(plan: &TerraformPlan) -> Vec<SyntheticComponent> {
    plan.nodes
        .iter()
        .flat_map(|node| {
            node.organization_roots.iter().map(move |index| {
                let key = format!("{}.roots[{index}]", node.address);
                SyntheticComponent {
                    entity: ArchitectureEntity::synthetic(
                        "organization_root",
                        &key,
                        vec![node.into()],
                    ),
                    id: format!("organization_root:{key}"),
                    kind: "organization_root".into(),
                    label: format!("Derived Organization Root: {key}"),
                    members: vec![node.address.clone()],
                }
            })
        })
        .collect()
}

pub(super) fn present(raw: &Input, graph: &mut Graph) {
    for (index, node) in raw.nodes.iter().enumerate() {
        if !node.provider.is_aws()
            || unsupported_data(&node.provider, &node.resource_type, node.mode)
            || !matches!(
                node.resource_type.as_str(),
                "aws_organizations_organizational_unit" | "aws_organizations_account"
            )
        {
            continue;
        }
        let Some(target) = graph
            .nodes
            .iter_mut()
            .find(|target| target.entity.id == node.entity.id)
        else {
            continue;
        };
        let reference = raw
            .attributes
            .iter()
            .find(|r| r.target == index && r.attribute == "parent_id");
        let verified = reference.is_some_and(|r| r.service_reference_verified);
        let root = reference
            .filter(|_| verified)
            .and_then(|r| r.source_attributes.iter().find_map(|path| root_index(path)));
        if verified {
            target
                .references
                .retain(|r| r.dependency || r.attribute != "parent_id");
        }
        target.references.push(ResourceReference {
            kind: root.map_or(
                ReferenceKind::OrganizationParent,
                ReferenceKind::OrganizationRoot,
            ),
            preserve_spatial: false,
            show_spatial: true,
            label: if root.is_some() {
                "Organization Root parent"
            } else {
                "Organization parent"
            },
            attribute: "parent_id".into(),
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
                reference
                    .map(|r| r.issues.clone())
                    .unwrap_or_default()
                    .into_iter()
                    .chain([DiagnosticReason::UnprovenEndpointValue])
                    .collect()
            },
        });
    }
}

pub(super) fn diagnostics(raw: &Input) -> Vec<Diagnostic> {
    raw.nodes
        .iter()
        .enumerate()
        .filter(|(_, node)| {
            node.provider.is_aws()
                && !unsupported_data(&node.provider, &node.resource_type, node.mode)
                && matches!(
                    node.resource_type.as_str(),
                    "aws_organizations_organizational_unit" | "aws_organizations_account"
                )
        })
        .filter(|(index, _)| {
            !raw.attributes.iter().any(|r| {
                r.target == *index && r.attribute == "parent_id" && r.service_reference_verified
            })
        })
        .map(|(_, node)| Diagnostic {
            address: node.address.clone(),
            deposed_key: node.deposed_key.clone(),
            attribute: "parent_id".into(),
            reason: DiagnosticReason::UnprovenEndpointValue,
        })
        .collect()
}
