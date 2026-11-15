use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum DiagnosticReason {
    Cloudflare(CloudflareFailure),
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
            Self::Cloudflare(reason) => reason.description(),
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum CloudflareFailure {
    SensitiveValue,
    UnknownValue,
    InvalidValue,
    IncompleteCollection,
    IncompleteProvenance,
    InactiveEndpoint,
    UnsupportedContract,
    ScopeUnavailable,
    ScopeMismatch,
    AmbiguousEndpoint,
    OutputMismatch,
    IdentityMismatch,
    OwnerMismatch,
}

impl CloudflareFailure {
    pub fn code(self) -> &'static str {
        match self {
            Self::SensitiveValue => "cloudflare_sensitive_value",
            Self::UnknownValue => "cloudflare_unknown_value",
            Self::InvalidValue => "cloudflare_invalid_value",
            Self::IncompleteCollection => "cloudflare_incomplete_collection",
            Self::IncompleteProvenance => "cloudflare_incomplete_provenance",
            Self::InactiveEndpoint => "cloudflare_inactive_endpoint",
            Self::UnsupportedContract => "cloudflare_unsupported_contract",
            Self::ScopeUnavailable => "cloudflare_scope_unavailable",
            Self::ScopeMismatch => "cloudflare_scope_mismatch",
            Self::AmbiguousEndpoint => "cloudflare_ambiguous_endpoint",
            Self::OutputMismatch => "cloudflare_output_mismatch",
            Self::IdentityMismatch => "cloudflare_identity_mismatch",
            Self::OwnerMismatch => "cloudflare_owner_mismatch",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::SensitiveValue => "Cloudflare reference requires a sensitive value",
            Self::UnknownValue => "Cloudflare reference requires a value unknown until apply",
            Self::InvalidValue => {
                "Cloudflare reference has a missing or invalid typed value or mask"
            }
            Self::IncompleteCollection => {
                "Cloudflare reference collection is incomplete or has duplicate identities"
            }
            Self::IncompleteProvenance => {
                "Cloudflare reference requires complete explicit Terraform provenance"
            }
            Self::InactiveEndpoint => {
                "Cloudflare reference requires current, non-deleted endpoints"
            }
            Self::UnsupportedContract => {
                "Cloudflare reference uses an unsupported type, mode or configuration contract"
            }
            Self::ScopeUnavailable => {
                "Cloudflare reference has no proven, unambiguous administration scope"
            }
            Self::ScopeMismatch => {
                "Cloudflare reference endpoints have incompatible administration scopes"
            }
            Self::AmbiguousEndpoint => "Cloudflare reference matches multiple current endpoints",
            Self::OutputMismatch => "Cloudflare reference selects an unsupported endpoint output",
            Self::IdentityMismatch => {
                "Cloudflare reference values or identity aliases do not match the endpoint"
            }
            Self::OwnerMismatch => {
                "Cloudflare deployment version does not have the same proven Worker"
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
