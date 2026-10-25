#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SyntheticComponent {
    pub entity: super::ArchitectureEntity,
    pub id: String,
    pub kind: String,
    pub label: String,
    pub label_reference: Option<ComponentLabelReference>,
    pub members: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ComponentLabelReference {
    pub prefix: &'static str,
    pub source: super::TerraformEntityId,
    pub suffix: String,
}
