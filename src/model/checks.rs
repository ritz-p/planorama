/// One Terraform checkable object's aggregate and instance results.
/// Messages and evaluated expression values are deliberately not retained.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct CheckResult {
    /// None means unavailable; unfamiliar status strings are preserved.
    pub status: Option<String>,
    pub instances: Vec<CheckInstance>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct CheckInstance {
    pub status: Option<String>,
    /// Exact, unambiguous resource address, independent of node indices.
    pub resource: Option<String>,
}
