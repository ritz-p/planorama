use crate::model::Node;
use std::fmt::Write;

pub(super) fn resource(node: &Node) -> String {
    let mut id = String::from("resource-");
    for byte in node.address.bytes() {
        write!(id, "{byte:02x}").unwrap();
    }
    match &node.deposed_key {
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
