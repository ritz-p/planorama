use crate::model::{DiagnosticReason as Reason, Node, StateOutput};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

#[cfg(test)]
#[path = "../../tests/unit/plan/outputs.rs"]
mod tests;

pub(super) fn collect(
    module: &Value,
    nodes: &[Node],
    symbols: &BTreeMap<String, BTreeSet<String>>,
) -> Vec<StateOutput> {
    let instances = super::references::instance_map(nodes);
    let mut aliases = symbols.clone();
    remove_resource_bindings(module, "", &mut aliases);
    module["outputs"]
        .as_object()
        .into_iter()
        .flatten()
        .map(|(name, output)| {
            let expression = &output["expression"];
            let mut refs = BTreeSet::new();
            super::references::references(expression, &mut refs);
            let refs = refs
                .into_iter()
                .map(|r| super::references::qualify_reference("", &r))
                .collect();
            let mut resolved =
                super::references::resolve_output_sources(&refs, &instances, &aliases);
            let shape = expression["references"]
                .as_array()
                .is_some_and(|refs| !refs.is_empty() && refs.iter().all(Value::is_string))
                && expression.get("constant_value").is_none();
            let ambiguous = resolved.issues.contains(&Reason::DynamicInstanceSelection)
                || resolved.issues.contains(&Reason::MultipleMatchingInstances);
            let complete = shape && resolved.complete && !ambiguous;
            if !complete && resolved.issues.is_empty() {
                resolved.issues.insert(if resolved.sources.is_empty() {
                    Reason::NoResourceReference
                } else {
                    Reason::PartialResourceProvenance
                });
            }
            StateOutput {
                name: name.clone(),
                sources: resolved
                    .sources
                    .into_iter()
                    .map(|i| nodes[i].address.clone())
                    .collect(),
                complete,
                issues: resolved.issues,
            }
        })
        .collect()
}

fn remove_resource_bindings(
    module: &Value,
    scope: &str,
    aliases: &mut BTreeMap<String, BTreeSet<String>>,
) {
    for resource in module["resources"].as_array().into_iter().flatten() {
        if let Some(address) = resource["address"].as_str() {
            aliases.remove(&super::references::qualify(scope, address));
        }
    }
    if let Some(calls) = module["module_calls"].as_object() {
        for (name, call) in calls {
            remove_resource_bindings(
                &call["module"],
                &super::references::qualify(scope, &format!("module.{name}")),
                aliases,
            );
        }
    }
}
