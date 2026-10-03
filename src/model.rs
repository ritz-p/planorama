#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Unchanged,
    Create,
    Update,
    Delete,
    Replace,
    Read,
    Other,
}

#[derive(Debug)]
pub struct Node {
    pub address: String,
    pub resource_type: String,
    pub module: String,
    pub action: Action,
    pub mode: EntityMode,
    #[allow(dead_code, reason = "Role-specific rendering is not implemented yet")]
    pub role: ResourceRole,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EntityMode {
    Managed,
    Data,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ResourceRole {
    Container,
    Node,
    Connector,
    Association,
    Policy,
    Controller,
    #[default]
    Unknown,
}

pub struct Graph {
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
}

#[allow(
    dead_code,
    reason = "Semantic transformations will introduce non-dependency edge kinds"
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum EdgeKind {
    Dependency,
    Association,
    Connection,
    Containment,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Edge {
    pub from: usize,
    pub to: usize,
    pub kind: EdgeKind,
}

impl Edge {
    pub fn endpoints(self) -> (usize, usize) {
        (self.from, self.to)
    }
}

impl From<(usize, usize)> for Edge {
    fn from((from, to): (usize, usize)) -> Self {
        Self {
            from,
            to,
            kind: EdgeKind::Dependency,
        }
    }
}
