use super::StateId;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct RemoteReference {
    pub consumer: String,
    pub remote: String,
    pub output: String,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Mapping {
    pub consumer: StateId,
    pub remote: String,
    pub producer: StateId,
}

impl Mapping {
    pub fn parse(value: &str) -> Result<Self, String> {
        let (left, producer) = value
            .rsplit_once('=')
            .ok_or("--remote-state requires CONSUMER:ADDRESS=PRODUCER")?;
        let (consumer, remote) = left
            .split_once(':')
            .ok_or("--remote-state requires CONSUMER:ADDRESS=PRODUCER")?;
        if remote.is_empty() || remote.chars().any(char::is_control) {
            return Err("invalid remote-state address".into());
        }
        Ok(Self {
            consumer: StateId::new(consumer)?,
            remote: remote.into(),
            producer: StateId::new(producer)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Endpoint {
    pub state: StateId,
    pub address: String,
}

/// A dependency with explicit remote-output provenance, not cross-state containment.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct CrossStateEdge {
    pub from: Endpoint,
    pub to: Endpoint,
    pub remote: String,
    pub output: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Failure {
    Unmapped,
    Ambiguous,
    MissingState,
    MissingOutput,
    IncompleteOutput,
    InvalidRemote,
}
impl Failure {
    pub fn description(self) -> &'static str {
        match self {
            Self::Unmapped => "unmapped remote state",
            Self::Ambiguous => "ambiguous remote-state mapping",
            Self::MissingState => "mapped state is not loaded",
            Self::MissingOutput => "producer output is missing",
            Self::IncompleteOutput => "producer output provenance is unresolved",
            Self::InvalidRemote => "remote-state data source is not uniquely known",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct CrossDiagnostic {
    pub consumer: Endpoint,
    pub remote: String,
    pub output: String,
    pub reason: Failure,
}
#[derive(Default, Debug, PartialEq, Eq)]
pub struct Resolution {
    pub edges: Vec<CrossStateEdge>,
    pub diagnostics: Vec<CrossDiagnostic>,
}
