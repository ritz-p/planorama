use super::{escape, shorten};
use crate::model::ResourceRole;

pub(super) fn annotation(role: ResourceRole, module: &str) -> String {
    let name = match role {
        ResourceRole::Policy => "policy",
        ResourceRole::Controller => "controller",
        _ => return String::new(),
    };
    format!("{name} · {}", escape(&shorten(module, 30)))
}

pub(super) fn attributes(role: ResourceRole) -> &'static str {
    match role {
        ResourceRole::Policy => " data-role=\"policy\"",
        ResourceRole::Controller => " data-role=\"controller\"",
        _ => "",
    }
}
