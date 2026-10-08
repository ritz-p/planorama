/// Only operation categories are retained; import IDs and identities may be sensitive.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct ChangeMetadata {
    pub import: Option<ImportMetadata>,
    pub state_removal: Option<StateRemoval>,
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
