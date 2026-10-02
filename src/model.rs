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
}

pub struct Graph {
    pub nodes: Vec<Node>,
    pub edges: Vec<(usize, usize)>,
}
