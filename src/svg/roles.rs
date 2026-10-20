use super::{escape, shorten};
use crate::model::architecture::ResourceRole;

pub(super) fn explanation(role: ResourceRole) -> Option<&'static str> {
    match role {
        ResourceRole::Identity => Some(
            "Role: Identity — principals, identity collections or federation configuration; does not prove authentication, role assumption or effective permissions.",
        ),
        ResourceRole::Policy => Some(
            "Role: Policy — access/control rules or policy-like configuration; not necessarily IAM or policy enforcement.",
        ),
        ResourceRole::Controller => Some(
            "Role: Controller — manages or configures services, workloads or related resources; does not prove runtime control, traffic or relationships.",
        ),
        _ => None,
    }
}

pub(super) fn annotation(role: ResourceRole, module: &str) -> String {
    let name = match role {
        ResourceRole::Identity => "identity",
        ResourceRole::Policy => "policy",
        ResourceRole::Controller => "controller",
        _ => return String::new(),
    };
    format!("{name} · {}", escape(&shorten(module, 30)))
}

pub(super) fn attributes(role: ResourceRole) -> &'static str {
    match role {
        ResourceRole::Identity => " data-role=\"identity\"",
        ResourceRole::Policy => " data-role=\"policy\"",
        ResourceRole::Controller => " data-role=\"controller\"",
        _ => "",
    }
}
