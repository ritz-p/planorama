use crate::model::{Diagnostic, DiagnosticReason as Reason, EntityMode, TerraformGraph};
use std::collections::BTreeSet;

/// Explain conservative inference using the same attribute resolutions as transforms.
/// Only addresses, attribute names and fixed category text can reach the output.
pub fn collect(raw: &TerraformGraph) -> Vec<Diagnostic> {
    let mut found = BTreeSet::new();
    for reference in &raw.attributes {
        let node = &raw.nodes[reference.target];
        let plural = node.provider.is_aws()
            && matches!(
                (node.resource_type.as_str(), reference.attribute.as_str()),
                ("aws_lb", "subnets") | ("aws_ecs_service", "network_configuration.subnets")
            );
        for &reason in &reference.issues {
            // Collection-valued subnet relationships intentionally resolve to
            // several sources. Their endpoint types are checked below.
            if reason == Reason::MultipleMatchingInstances && plural && reference.complete {
                continue;
            }
            found.insert(Diagnostic {
                address: node.address.clone(),
                attribute: reference.attribute.clone(),
                reason,
            });
        }
    }
    for (target, node) in raw.nodes.iter().enumerate() {
        if node.mode != EntityMode::Managed || !node.provider.is_aws() {
            continue;
        }
        let expected: &[(&str, &str, bool, bool)] = match node.resource_type.as_str() {
            "aws_subnet" => &[("vpc_id", "aws_vpc", false, true)],
            "aws_instance" => &[("subnet_id", "aws_subnet", false, true)],
            "aws_lb" => &[("subnets", "aws_subnet", true, false)],
            "aws_ecs_service" => &[("network_configuration.subnets", "aws_subnet", true, false)],
            "aws_route_table_association" => &[
                ("subnet_id", "aws_subnet", false, false),
                ("route_table_id", "aws_route_table", false, false),
            ],
            _ => &[],
        };
        let mut endpoints_valid = !expected.is_empty();
        for &(attribute, resource_type, plural, containment) in expected {
            let reference = raw
                .attributes
                .iter()
                .find(|r| r.target == target && r.attribute == attribute);
            let reason = match reference {
                None => Some(Reason::MissingAttribute),
                Some(r) if !r.complete || r.sources.is_empty() => {
                    // Detailed resolution failures have already been emitted.
                    if r.issues.is_empty() {
                        Some(Reason::NoResourceReference)
                    } else {
                        endpoints_valid = false;
                        None
                    }
                }
                Some(r) if !plural && r.sources.len() > 1 => Some(if containment {
                    Reason::AmbiguousContainmentParent
                } else {
                    Reason::MultipleMatchingInstances
                }),
                Some(r)
                    if r.sources.iter().any(|&source| {
                        source == target
                            || raw.nodes[source].resource_type != resource_type
                            || !raw.nodes[source].provider.is_aws()
                    }) =>
                {
                    Some(Reason::EndpointTypeMismatch)
                }
                _ => None,
            };
            if let Some(reason) = reason {
                endpoints_valid = false;
                found.insert(Diagnostic {
                    address: node.address.clone(),
                    attribute: attribute.into(),
                    reason,
                });
            }
        }
        if endpoints_valid && node.resource_type == "aws_route_table_association" {
            let incoming: BTreeSet<_> = raw
                .attributes
                .iter()
                .filter(|r| {
                    r.target == target
                        && matches!(r.attribute.as_str(), "subnet_id" | "route_table_id")
                })
                .flat_map(|r| r.sources.iter().copied())
                .collect();
            let incident: Vec<_> = raw
                .edges
                .iter()
                .filter(|e| e.from == target || e.to == target)
                .collect();
            if incident.len() != 2
                || incident
                    .iter()
                    .any(|e| e.to != target || !incoming.contains(&e.from))
            {
                found.insert(Diagnostic {
                    address: node.address.clone(),
                    attribute: "subnet_id,route_table_id".into(),
                    reason: Reason::AdditionalRelationships,
                });
            }
        }
    }
    found.into_iter().collect()
}
