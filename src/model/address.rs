use super::Node;
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
