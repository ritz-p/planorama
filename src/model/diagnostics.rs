use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum DiagnosticReason {
    UnresolvedReference,
    UnverifiedMonitoring,
    MultipleMatchingInstances,
    DynamicInstanceSelection,
    AliasCycle,
    MissingAttribute,
    NoResourceReference,
    PartialResourceProvenance,
    EndpointTypeMismatch,
    ProviderConfigurationMismatch,
    ConflictingAwsAccount,
    AmbiguousContainmentParent,
    AdditionalRelationships,
    DriftDetected,
    DriftUnmatched,
    UnsupportedRouteTarget,
    AmbiguousRouteTarget,
    NonIdReference,
    UnprovenEndpointValue,
    DeposedEndpoint,
    UnresolvedRouteInformation,
}

impl DiagnosticReason {
    pub fn description(self) -> &'static str {
        match self {
            Self::ConflictingAwsAccount => {
                "conflicting AWS account evidence in provider configuration"
            }
            Self::UnverifiedMonitoring => {
                "alarm monitoring target is not verified by supported dimensions and planned values"
            }
            Self::UnresolvedRouteInformation => {
                "route information is unavailable or not safe to display"
            }
            Self::UnsupportedRouteTarget => "unsupported route target",
            Self::AmbiguousRouteTarget => "multiple route target attributes",
            Self::NonIdReference => "endpoint reference does not select resource IDs exclusively",
            Self::UnprovenEndpointValue => {
                "planned endpoint value is not proven to equal the referenced resource identity"
            }
            Self::DeposedEndpoint => "deposed resource cannot be a route endpoint",
            Self::DriftDetected => {
                "Terraform-reported resource drift (separate from planned action)"
            }
            Self::DriftUnmatched => "Terraform-reported drift could not be matched conservatively",
            Self::UnresolvedReference => "unresolved reference",
            Self::MultipleMatchingInstances => "multiple matching instances",
            Self::DynamicInstanceSelection => "dynamic instance selection",
            Self::AliasCycle => "alias/local/module resolution cycle",
            Self::MissingAttribute => "missing expected attribute",
            Self::NoResourceReference => "attribute has no resolvable resource reference",
            Self::PartialResourceProvenance => {
                "only part of the attribute resolves to resource references"
            }
            Self::EndpointTypeMismatch => "semantic endpoint type mismatch",
            Self::ProviderConfigurationMismatch => {
                "provider configuration does not match the referenced endpoint"
            }
            Self::AmbiguousContainmentParent => "ambiguous containment parent",
            Self::AdditionalRelationships => {
                "additional relationships prevent association lowering"
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Diagnostic {
    pub deposed_key: Option<String>,
    pub address: String,
    pub attribute: String,
    pub reason: DiagnosticReason,
}

#[derive(Default)]
pub(crate) struct Resolution {
    pub sources: Vec<usize>,
    pub complete: bool,
    pub ids_only: bool,
    pub instances_explicit: bool,
    pub issues: BTreeSet<DiagnosticReason>,
}
