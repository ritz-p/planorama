#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SyntheticComponent {
    pub entity: super::ArchitectureEntity,
    pub id: String,
    pub kind: String,
    pub label: String,
    pub members: Vec<String>,
}
