use crate::model::architecture::Node;
use std::fmt::Write;

pub(super) fn architecture(id: &crate::model::architecture::ArchitectureId) -> String {
    let mut encoded = String::from("architecture-");
    for byte in id.as_str().bytes() {
        write!(encoded, "{byte:02x}").unwrap();
    }
    encoded
}

pub(crate) fn resource(node: &Node) -> String {
    resource_parts(&node.address, node.deposed_key.as_deref())
}

pub(super) fn source(source: &crate::model::TerraformEntityId) -> String {
    resource_parts(&source.address, source.deposed_key.as_deref())
}

fn resource_parts(address: &str, deposed_key: Option<&str>) -> String {
    let mut id = String::from("resource-");
    for byte in address.bytes() {
        write!(id, "{byte:02x}").unwrap();
    }
    match deposed_key {
        None => id.push_str("-current"),
        Some(key) => {
            id.push_str("-deposed-");
            for byte in key.bytes() {
                write!(id, "{byte:02x}").unwrap();
            }
        }
    }
    id
}
