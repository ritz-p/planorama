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

#[allow(
    dead_code,
    reason = "Provider classifiers will populate these architectural roles"
)]
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
    pub edges: Vec<(usize, usize)>,
}
