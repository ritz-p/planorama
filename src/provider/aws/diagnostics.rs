use crate::model::{Diagnostic, DiagnosticReason as Reason, EntityMode};
use crate::semantic::Input;
use std::collections::BTreeSet;

/// Explain conservative inference using the same attribute resolutions as transforms.
/// Only addresses, attribute names and fixed category text can reach the output.
pub fn collect(plan: &crate::model::TerraformPlan) -> Vec<Diagnostic> {
    let input = Input::new(plan);
    let raw = &input;
    let mut found = BTreeSet::new();
    for output in &raw.outputs {
        for &reason in &output.issues {
            found.insert(Diagnostic {
                address: format!("output.{}", output.name),
                attribute: "value".into(),
                reason,
            });
        }
    }
    for drift in &raw.drift {
        found.insert(Diagnostic {
            address: drift.address.clone(),
            attribute: "resource_drift".into(),
            reason: if drift.matched {
                Reason::DriftDetected
            } else {
                Reason::DriftUnmatched
            },
        });
    }
    // Reuse actual inference so ancestry handling stays aligned with rendering.
    let contained = super::containment::infer(raw);
    for (reference, dependency_metadata) in raw
        .attributes
        .iter()
        .map(|r| (r, false))
        .chain(raw.graph_references.iter().map(|r| (r, true)))
    {
        let node = &raw.nodes[reference.target];
        let plural = node.provider.is_aws()
            && matches!(
                (node.resource_type.as_str(), reference.attribute.as_str()),
                ("aws_lb", "subnets")
                    | ("aws_db_subnet_group", "subnet_ids")
                    | (
                        "aws_ecs_service",
                        "network_configuration" | "network_configuration.subnets"
                    )
            );
        for &reason in &reference.issues {
            // Network collections (including the aggregate ECS block) and
            // dependency metadata legitimately resolve to multiple instances.
            // Semantic endpoint types are checked separately below.
            let whole_dependency = dependency_metadata;
            if reason == Reason::MultipleMatchingInstances && (plural || whole_dependency) {
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
        if node.mode != EntityMode::Managed || !node.provider.is_aws() || node.deposed_key.is_some()
        {
            continue;
        }
        let direct = super::containment::direct_rule(&node.resource_type)
            .filter(|(attribute, _)| {
                node.resource_type != "aws_lb_target_group"
                    || raw
                        .attributes
                        .iter()
                        .any(|r| r.target == target && r.attribute == *attribute)
            })
            .map(|(attribute, parent)| [(attribute, parent, false, true)]);
        if node.resource_type == "aws_route" {
            match super::routes::endpoints(raw, target) {
                Err(diagnostic) => {
                    found.insert(diagnostic);
                }
                Ok((from, to)) => {
                    let incident: Vec<_> = raw
                        .edges
                        .iter()
                        .filter(|e| e.from == target || e.to == target)
                        .collect();
                    if !super::associations::can_lower(target, from, to, &incident) {
                        found.insert(Diagnostic {
                            address: node.address.clone(),
                            attribute: "route_table_id,route_target".into(),
                            reason: Reason::AdditionalRelationships,
                        });
                    }
                }
            }
        }
        // Security-group attachment attributes are optional; absent attributes
        // do not imply missing provenance or an invalid Terraform configuration.
        if let Some(attribute) = super::security_groups::attribute(&node.resource_type) {
            if let Some(reference) = raw
                .attributes
                .iter()
                .find(|r| r.target == target && r.attribute == attribute)
            {
                if let Some(reason) = super::security_groups::failure(raw, reference) {
                    if reference.issues.is_empty() || reason == Reason::EndpointTypeMismatch {
                        found.insert(Diagnostic {
                            address: node.address.clone(),
                            attribute: attribute.into(),
                            reason,
                        });
                    }
                }
            }
        }
        let association = super::associations::rule(&node.resource_type)
            .map(|rule| rule.map(|(attribute, endpoint)| (attribute, endpoint, false, false)));
        let expected: &[(&str, &str, bool, bool)] = if let Some(rule) = &direct {
            rule
        } else if let Some(rule) = &association {
            rule
        } else {
            match node.resource_type.as_str() {
                "aws_lb" => &[("subnets", "aws_subnet", true, false)],
                "aws_db_subnet_group" => &[("subnet_ids", "aws_subnet", true, false)],
                "aws_db_instance" | "aws_rds_cluster" => {
                    &[("db_subnet_group_name", "aws_db_subnet_group", false, true)]
                }
                "aws_ecs_service" => {
                    &[("network_configuration.subnets", "aws_subnet", true, false)]
                }
                _ => &[],
            }
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
                        Some(if r.sources.is_empty() {
                            Reason::NoResourceReference
                        } else {
                            Reason::PartialResourceProvenance
                        })
                    } else {
                        endpoints_valid = false;
                        None
                    }
                }
                Some(r)
                    if r.sources.iter().any(|&source| {
                        source == target
                            || raw.nodes[source].resource_type != resource_type
                            || !raw.nodes[source].provider.is_aws()
                    }) =>
                {
                    Some(Reason::EndpointTypeMismatch)
                }
                Some(r) if !plural && r.sources.len() > 1 => Some(if containment {
                    Reason::AmbiguousContainmentParent
                } else {
                    Reason::MultipleMatchingInstances
                }),
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
        if endpoints_valid
            && (matches!(node.resource_type.as_str(), "aws_lb" | "aws_ecs_service")
                || (matches!(
                    node.resource_type.as_str(),
                    "aws_db_instance" | "aws_rds_cluster"
                ) && super::containment::resolved_subnets(raw, target).is_some()))
            && !contained
                .edges
                .iter()
                .any(|edge| edge.to == target && edge.kind == crate::model::EdgeKind::Containment)
        {
            found.insert(Diagnostic {
                address: node.address.clone(),
                attribute: expected[0].0.into(),
                reason: Reason::AmbiguousContainmentParent,
            });
        }
        if endpoints_valid && association.is_some() {
            let incoming: BTreeSet<_> = raw
                .attributes
                .iter()
                .filter(|r| r.target == target && expected.iter().any(|rule| rule.0 == r.attribute))
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
                    attribute: expected
                        .iter()
                        .map(|rule| rule.0)
                        .collect::<Vec<_>>()
                        .join(","),
                    reason: Reason::AdditionalRelationships,
                });
            }
        }
    }
    found.into_iter().collect()
}
