/// A logical architecture entity, not a Terraform resource or spatial container.
/// Provider-specific inference supplies the kind/label; the model is provider-neutral.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SyntheticComponent {
    pub id: String,
    pub kind: String,
    pub label: String,
    /// Exact source addresses. Original nodes retain actions and all metadata.
    pub members: Vec<String>,
}
