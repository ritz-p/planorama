#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct CheckResult {
    pub status: Option<String>,
    pub instances: Vec<CheckInstance>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct CheckInstance {
    pub status: Option<String>,
    pub resource: Option<String>,
}
