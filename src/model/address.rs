use super::{EdgeChange, Node};
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
        ResourceAddress {
            terraform_address: &self.address,
            module_path: match self.module.as_str() {
                "root" => ModulePath::Root,
                module => ModulePath::Child(module),
            },
        }
    }
}

impl EdgeChange {
    pub fn local_address(&self) -> &str {
        ResourceAddress {
            terraform_address: &self.address,
            module_path: match module_of(&self.address) {
                "root" => ModulePath::Root,
                module => ModulePath::Child(module),
            },
        }
        .local()
    }
}

impl<'a> ResourceAddress<'a> {
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

pub(crate) fn module_of(address: &str) -> &str {
    let mut boundary = 0;
    let (mut depth, mut quoted, mut escaped) = (0usize, false, false);
    let mut parts = Vec::new();
    for (i, c) in address.char_indices() {
        match (escaped, quoted, c) {
            (true, _, _) => escaped = false,
            (_, true, '\\') => escaped = true,
            (_, _, '"') if depth > 0 => quoted = !quoted,
            (_, false, '[') => depth += 1,
            (_, false, ']') => depth = depth.saturating_sub(1),
            (_, false, '.') if depth == 0 => {
                parts.push((boundary, i));
                boundary = i + 1;
            }
            _ => {}
        }
    }
    parts.push((boundary, address.len()));
    let mut count = 0;
    while count + 1 < parts.len() && &address[parts[count].0..parts[count].1] == "module" {
        count += 2;
    }
    match count {
        0 => "root",
        _ => &address[..parts[count - 1].1],
    }
}
