use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum DiagnosticReason {
    UnresolvedReference,
    MultipleMatchingInstances,
    DynamicInstanceSelection,
    AliasCycle,
    MissingAttribute,
    NoResourceReference,
    PartialResourceProvenance,
    EndpointTypeMismatch,
    AmbiguousContainmentParent,
    AdditionalRelationships,
    DriftDetected,
    DriftUnmatched,
}

impl DiagnosticReason {
    pub fn description(self) -> &'static str {
        match self {
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
            Self::AmbiguousContainmentParent => "ambiguous containment parent",
            Self::AdditionalRelationships => {
                "additional relationships prevent association lowering"
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Diagnostic {
    pub address: String,
    pub attribute: String,
    pub reason: DiagnosticReason,
}

/// Resolution details contain categories only, never expression/attribute values.
#[derive(Default)]
pub(crate) struct Resolution {
    pub sources: Vec<usize>,
    pub complete: bool,
    pub issues: BTreeSet<DiagnosticReason>,
}
