use super::{EdgeChange, Node, TerraformEntityId};
use std::borrow::Cow;

#[cfg(test)]
#[path = "../../tests/unit/model/address.rs"]
mod tests;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ModulePath<'a> {
    Root,
    Child(&'a str),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResourceAddress<'a> {
    terraform_address: &'a str,
    module_path: ModulePath<'a>,
}

impl Node {
    pub fn resource_address(&self) -> ResourceAddress<'_> {
        ResourceAddress::new(&self.address, &self.module)
    }
}

impl TerraformEntityId {
    pub(crate) fn resource_address(&self) -> ResourceAddress<'_> {
        ResourceAddress::new(&self.address, module_of(&self.address))
    }
}

impl EdgeChange {
    pub(crate) fn resource_address(&self) -> ResourceAddress<'_> {
        ResourceAddress::new(&self.address, module_of(&self.address))
    }

    pub fn local_address(&self) -> &str {
        self.resource_address().local()
    }
}

impl<'a> ResourceAddress<'a> {
    pub(crate) fn new(terraform_address: &'a str, module: &'a str) -> Self {
        Self {
            terraform_address,
            module_path: match module {
                "root" => ModulePath::Root,
                child => ModulePath::Child(child),
            },
        }
    }

    pub(crate) fn formatted(self, qualified: bool) -> Cow<'a, str> {
        if qualified {
            self.qualified()
        } else {
            Cow::Borrowed(self.terraform())
        }
    }

    pub fn terraform(self) -> &'a str {
        self.terraform_address
    }

    pub fn qualified(self) -> Cow<'a, str> {
        match self.module_path {
            ModulePath::Root => Cow::Owned(format!("module.root.{}", self.terraform_address)),
            ModulePath::Child(_) => Cow::Borrowed(self.terraform_address),
        }
    }

    pub fn local(self) -> &'a str {
        match self.module_path {
            ModulePath::Root => self.terraform_address,
            ModulePath::Child(module) => self
                .terraform_address
                .strip_prefix(module)
                .and_then(|suffix| suffix.strip_prefix('.'))
                .unwrap_or(self.terraform_address),
        }
    }
}

pub(crate) fn address_parts(address: &str) -> impl Iterator<Item = &str> {
    let (mut depth, mut quoted, mut escaped) = (0usize, false, false);
    address.split(move |c| {
        match (escaped, quoted, c) {
            (true, _, _) => escaped = false,
            (_, true, '\\') => escaped = true,
            (_, _, '"') if depth > 0 => quoted = !quoted,
            (_, false, '[') => depth += 1,
            (_, false, ']') => depth = depth.saturating_sub(1),
            (_, false, '.') if depth == 0 => return true,
            _ => {}
        }
        false
    })
}

pub(crate) fn module_of(address: &str) -> &str {
    let mut parts = address_parts(address);
    let mut end = 0;
    while parts.next() == Some("module") {
        let Some(child) = parts.next() else { break };
        if end > 0 {
            end += 1;
        }
        end += "module.".len() + child.len();
    }
    if end == 0 { "root" } else { &address[..end] }
}
