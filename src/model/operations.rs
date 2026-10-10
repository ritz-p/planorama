#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct ChangeMetadata {
    pub import: Option<ImportMetadata>,
    pub state_removal: Option<StateRemoval>,
    pub drift: std::collections::BTreeSet<super::Action>,
    pub action_reason: Option<String>,
    pub replace_paths: Option<Vec<Vec<AttributePathStep>>>,
    pub relevant_attributes: Option<Vec<Vec<AttributePathStep>>>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum AttributePathStep {
    Attribute(String),
    Index(u64),
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct DriftRecord {
    pub relevant: bool,
    pub address: String,
    pub deposed_key: Option<String>,
    pub previous_address: Option<String>,
    pub action: super::Action,
    pub matched: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct RelevantAttribute {
    pub resource: String,
    pub path: Option<Vec<AttributePathStep>>,
    pub matched: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct ImportMetadata {
    pub has_id: bool,
    pub has_identity: bool,
    pub unknown: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum StateRemoval {
    Forget,
    CreateThenForget,
    ForgetThenCreate,
}

impl StateRemoval {
    pub fn label(self) -> &'static str {
        match self {
            Self::Forget => "forget",
            Self::CreateThenForget => "create then forget",
            Self::ForgetThenCreate => "forget then create",
        }
    }
}
