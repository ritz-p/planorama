use crate::model::{TerraformEntity, cross_state::RemoteReference};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn collect(module: &Value, nodes: &[TerraformEntity]) -> Vec<RemoteReference> {
    let mut expressions = BTreeMap::new();
    walk(module, "", &mut expressions);
    let mut result = BTreeSet::new();
    for node in nodes.iter().filter(|n| n.deposed_key.is_none()) {
        if let Some(refs) = expressions.get(&super::address::static_address(&node.address)) {
            for reference in refs {
                let reference = super::address::contextualize(reference, &node.address);
                let parts = super::address::parts(&reference);
                let mut i = 0;
                while i + 1 < parts.len() && parts[i] == "module" && !parts[i + 1].is_empty() {
                    i += 2;
                }
                if parts.get(i..i + 2) != Some(&["data", "terraform_remote_state"][..]) {
                    continue;
                }
                if parts.len() != i + 5 || parts[i + 3] != "outputs" {
                    continue;
                }
                let remote = parts[..i + 3].join(".");
                if super::address::dynamic_selection(
                    &remote,
                    Some(&super::address::static_address(&remote)),
                ) || parts[i + 4].contains(['[', ']'])
                {
                    continue;
                }
                result.insert(RemoteReference {
                    consumer: node.address.clone(),
                    remote,
                    output: parts[i + 4].into(),
                });
            }
        }
    }
    result.into_iter().collect()
}

fn walk(module: &Value, scope: &str, found: &mut BTreeMap<String, BTreeSet<String>>) {
    for resource in module["resources"].as_array().into_iter().flatten() {
        if let Some(address) = resource["address"].as_str() {
            let mut refs = BTreeSet::new();
            super::references::references(&resource["expressions"], &mut refs);
            super::references::references(&resource["count_expression"], &mut refs);
            super::references::references(&resource["for_each_expression"], &mut refs);
            found.insert(
                super::references::qualify(scope, address),
                refs.into_iter()
                    .map(|r| super::references::qualify(scope, &r))
                    .collect(),
            );
        }
    }
    for (name, call) in module["module_calls"].as_object().into_iter().flatten() {
        walk(
            &call["module"],
            &super::references::qualify(scope, &format!("module.{name}")),
            found,
        );
    }
}

#[cfg(test)]
#[test]
fn instance_expressions_are_collected_but_attribute_names_are_not_remote_roots() {
    for field in ["count_expression", "for_each_expression"] {
        let mut resource = serde_json::json!({"address":"test.app"});
        resource[field] = serde_json::json!({"references":["data.terraform_remote_state.network.outputs.subnets"]});
        resource["expressions"] = serde_json::json!({"input":{"references":["terraform_data.config.output.data.terraform_remote_state.name.outputs.value"]}});
        let input = serde_json::json!({"format_version":"1.2","resource_changes":[{"address":"test.app[0]","type":"test"}],"configuration":{"root_module":{"resources":[resource]}}});
        let raw = crate::plan::parse(&input.to_string()).unwrap();
        assert_eq!(raw.remote_references.len(), 1);
        assert_eq!(
            raw.remote_references[0].remote,
            "data.terraform_remote_state.network"
        );
        assert_eq!(raw.remote_references[0].consumer, "test.app[0]");
    }
}
