use crate::model::{
    DiagnosticReason, EntityMode, Graph, RelationshipProvenance, TerraformEntityId, TerraformPlan,
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Counts {
    pub resources: usize,
    pub account_applicable: usize,
    pub unknown_account: usize,
    pub region_applicable: usize,
    pub unknown_region: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Resource {
    pub mode: EntityMode,
    pub account_applicable: bool,
    pub unknown_account: bool,
    pub region_applicable: bool,
    pub unknown_region: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Finding {
    pub source: TerraformEntityId,
    pub attribute: Option<String>,
    pub code: &'static str,
    pub category: &'static str,
    pub message: &'static str,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Coverage {
    pub embed_findings: bool,
    pub collected: bool,
    pub input_complete: Option<bool>,
    pub input_findings: BTreeMap<&'static str, usize>,
    pub selected_findings: BTreeMap<&'static str, usize>,
    pub input: Counts,
    pub selected: Counts,
    pub resources: BTreeMap<TerraformEntityId, Resource>,
    pub findings: Vec<Finding>,
    pub unassociated_findings: usize,
    pub focused: bool,
    pub changed_only: bool,
}

impl Counts {
    fn collect<'a>(resources: impl Iterator<Item = &'a Resource>) -> Self {
        let mut counts = Self::default();
        for resource in resources {
            counts.resources += 1;
            counts.account_applicable += usize::from(resource.account_applicable);
            counts.unknown_account += usize::from(resource.unknown_account);
            counts.region_applicable += usize::from(resource.region_applicable);
            counts.unknown_region += usize::from(resource.unknown_region);
        }
        counts
    }
}

pub fn represented(graph: &Graph) -> BTreeSet<TerraformEntityId> {
    graph
        .nodes
        .iter()
        .flat_map(|node| node.entity.provenance.iter().cloned())
        .chain(
            graph
                .relationships
                .iter()
                .flat_map(|relationship| relationship.provenance.iter())
                .filter_map(|proof| {
                    if let RelationshipProvenance::Resource { source, .. } = proof {
                        Some(source.clone())
                    } else {
                        None
                    }
                }),
        )
        .collect()
}

impl Coverage {
    pub fn collect(plan: &TerraformPlan, graph: &Graph) -> Self {
        let resources: BTreeMap<_, _> = plan
            .nodes
            .iter()
            .map(|node| {
                let scope = crate::provider::scope(node);
                let account_applicable = node.provider.is_aws() && node.mode == EntityMode::Managed;
                let region_applicable = account_applicable
                    && !scope.as_ref().is_some_and(|scope| scope.global)
                    && crate::provider::classify(&node.provider, &node.resource_type)
                        != crate::model::ResourceRole::Unknown;
                (
                    node.into(),
                    Resource {
                        mode: node.mode,
                        account_applicable,
                        unknown_account: account_applicable
                            && scope
                                .as_ref()
                                .and_then(|scope| scope.account.as_ref())
                                .is_none(),
                        region_applicable,
                        unknown_region: region_applicable
                            && scope
                                .as_ref()
                                .and_then(|scope| scope.region.as_ref())
                                .is_none(),
                    },
                )
            })
            .collect();
        let mut result = Self {
            collected: true,
            input_complete: plan.status.complete,
            input: Counts::collect(resources.values()),
            resources,
            ..Self::default()
        };
        for diagnostic in crate::semantic::diagnostics::collect_instances(plan) {
            let source = TerraformEntityId {
                address: diagnostic.address,
                deposed_key: diagnostic.deposed_key,
            };
            if result.resources.contains_key(&source)
                && diagnostic.reason != DiagnosticReason::DriftUnmatched
            {
                result.findings.push(Finding {
                    source,
                    attribute: (!diagnostic.attribute.is_empty()).then_some(diagnostic.attribute),
                    code: diagnostic.reason.code(),
                    category: diagnostic.reason.category(),
                    message: diagnostic.reason.description(),
                });
            } else {
                result.unassociated_findings += 1;
                *result
                    .input_findings
                    .entry(diagnostic.reason.category())
                    .or_default() += 1;
            }
        }
        for node in &plan.nodes {
            if crate::provider::classify(&node.provider, &node.resource_type)
                == crate::model::ResourceRole::Unknown
            {
                result.findings.push(Finding {
                    source: node.into(),
                    attribute: None,
                    code: "unsupported_classification",
                    category: "classification",
                    message: "resource type has no supported semantic classification",
                });
            }
            let resource = &result.resources[&TerraformEntityId::from(node)];
            for (unknown, code) in [
                (resource.unknown_account, "unknown_account"),
                (resource.unknown_region, "unknown_region"),
            ] {
                if unknown {
                    result.findings.push(Finding {
                        source: node.into(),
                        attribute: None,
                        code,
                        category: "scope",
                        message: "applicable deployment scope has no verified evidence",
                    });
                }
            }
        }
        result.findings.sort();
        result.findings.dedup();
        for finding in &result.findings {
            *result.input_findings.entry(finding.category).or_default() += 1;
        }
        result.retain(&represented(graph));
        result
    }

    pub fn retain(&mut self, selected: &BTreeSet<TerraformEntityId>) {
        self.resources.retain(|id, _| selected.contains(id));
        self.findings
            .retain(|finding| selected.contains(&finding.source));
        self.selected = Counts::collect(self.resources.values());
        self.selected_findings.clear();
        for finding in &self.findings {
            *self.selected_findings.entry(finding.category).or_default() += 1;
        }
    }
}

impl DiagnosticReason {
    pub fn code(self) -> &'static str {
        match self {
            Self::UnresolvedReference => "unresolved_reference",
            Self::UnverifiedMonitoring => "unverified_monitoring",
            Self::MultipleMatchingInstances => "multiple_matching_instances",
            Self::DynamicInstanceSelection => "dynamic_instance_selection",
            Self::AliasCycle => "alias_cycle",
            Self::MissingAttribute => "missing_attribute",
            Self::NoResourceReference => "no_resource_reference",
            Self::PartialResourceProvenance => "partial_resource_provenance",
            Self::EndpointTypeMismatch => "endpoint_type_mismatch",
            Self::ProviderConfigurationMismatch => "provider_configuration_mismatch",
            Self::ConflictingAwsAccount => "conflicting_aws_account",
            Self::AmbiguousContainmentParent => "ambiguous_containment_parent",
            Self::AdditionalRelationships => "additional_relationships",
            Self::DriftDetected => "drift_detected",
            Self::DriftUnmatched => "drift_unmatched",
            Self::UnsupportedRouteTarget => "unsupported_route_target",
            Self::AmbiguousRouteTarget => "ambiguous_route_target",
            Self::NonIdReference => "non_id_reference",
            Self::UnprovenEndpointValue => "unproven_endpoint_value",
            Self::DeposedEndpoint => "deposed_endpoint",
            Self::UnresolvedRouteInformation => "unresolved_route_information",
        }
    }

    pub fn category(self) -> &'static str {
        match self {
            Self::UnresolvedReference
            | Self::AliasCycle
            | Self::NoResourceReference
            | Self::PartialResourceProvenance => "unresolved",
            Self::MultipleMatchingInstances
            | Self::AmbiguousContainmentParent
            | Self::AmbiguousRouteTarget => "ambiguous",
            Self::DynamicInstanceSelection => "dynamic",
            Self::ConflictingAwsAccount => "scope",
            Self::DriftDetected | Self::DriftUnmatched => "drift",
            _ => "unavailable_endpoint",
        }
    }
}

#[cfg(test)]
#[path = "../tests/unit/coverage.rs"]
mod tests;
