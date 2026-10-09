use super::address::{contextualize, matches_instance, static_address};
use crate::model::{DiagnosticReason, Resolution, TerraformEntity};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn expression_object(fields: &serde_json::Map<String, Value>) -> bool {
    if !fields.contains_key("references") && !fields.contains_key("constant_value") {
        return false;
    }
    if let Some(values) = fields.get("references").and_then(Value::as_array) {
        // Empty arrays need sibling evidence to distinguish a block from an
        // empty reference expression. Literal metadata is not such evidence.
        if values.is_empty() {
            return !fields.iter().any(|(name, value)| {
                name != "references"
                    && name != "constant_value"
                    && (value.is_object() || value.is_array())
            });
        }
        // Reference metadata contains strings, whereas child blocks contain objects.
        return !values.iter().all(Value::is_object);
    }
    // A direct block contains expression objects as fields. A lone
    // constant_value remains a literal: its object contents are ambiguous.
    !(fields
        .values()
        .all(|value| value.is_object() || value.is_array())
        && fields.iter().any(|(name, value)| {
            name != "constant_value" && (value.is_object() || value.is_array())
        }))
}

pub(super) fn references(value: &Value, found: &mut BTreeSet<String>) {
    match value {
        Value::Object(object) => {
            if !expression_object(object) {
                for value in object.values() {
                    references(value, found);
                }
                return;
            }
            if let Some(refs) = object.get("references").and_then(Value::as_array) {
                // Terraform emits traversal prefixes alongside the full reference.
                // Keep the most specific traversals within this expression.
                let refs: Vec<_> = refs.iter().filter_map(Value::as_str).collect();
                found.extend(
                    refs.iter()
                        .filter(|&&reference| {
                            !refs.iter().any(|&other| {
                                other != reference
                                    && other.strip_prefix(reference).is_some_and(|suffix| {
                                        suffix.starts_with('.') || suffix.starts_with('[')
                                    })
                            })
                        })
                        .map(|&reference| reference.to_owned()),
                );
            }
            for (key, value) in object {
                if key != "constant_value" && key != "references" {
                    references(value, found);
                }
            }
        }
        Value::Array(values) => {
            for value in values {
                // Arrays in configuration expressions contain block field maps.
                // Their field names are provider attributes, not expression metadata.
                if let Value::Object(fields) = value {
                    for expression in fields.values() {
                        references(expression, found);
                    }
                } else {
                    references(value, found);
                }
            }
        }
        _ => {}
    }
}

pub(super) fn qualify(scope: &str, reference: &str) -> String {
    match scope {
        "" => reference.into(),
        _ => format!("{scope}.{reference}"),
    }
}

pub(super) fn qualify_reference(scope: &str, reference: &str) -> String {
    if reference.starts_with("module.") {
        return format!(
            "{}{}",
            super::address::MODULE_REFERENCE,
            qualify(scope, reference)
        );
    }
    // Iteration context has no resource identity. Keep its original root so
    // it cannot collide with a module output named count or each. Explicit
    // module traversals still receive normal lexical qualification.
    if builtin_value(reference)
        || (!reference.starts_with("module.") && super::address::meta_reference(reference))
    {
        reference.into()
    } else {
        qualify(scope, reference)
    }
}

fn builtin_value(reference: &str) -> bool {
    matches!(
        reference,
        "path.module" | "path.root" | "path.cwd" | "terraform.workspace"
    )
}

pub(super) fn collect_config(
    module: &Value,
    scope: &str,
    inherited: &BTreeSet<String>,
    symbols: &mut BTreeMap<String, BTreeSet<String>>,
) {
    if let Some(variables) = module["variables"].as_object() {
        for name in variables.keys() {
            // Preserve supplied module-input aliases: they may carry actual
            // resource provenance. Declarations/defaults alone carry none.
            symbols
                .entry(qualify(scope, &format!("var.{name}")))
                .or_default();
        }
    }
    if let Some(locals) = module["locals"].as_object() {
        for (name, expression) in locals {
            let mut refs = BTreeSet::new();
            references(expression, &mut refs);
            symbols.insert(
                qualify(scope, &format!("local.{name}")),
                refs.into_iter()
                    .map(|r| qualify_reference(scope, &r))
                    .collect(),
            );
        }
    }
    for resource in module["resources"].as_array().into_iter().flatten() {
        if let Some(address) = resource["address"].as_str() {
            let mut refs = BTreeSet::new();
            references(&resource["expressions"], &mut refs);
            references(&resource["count_expression"], &mut refs);
            references(&resource["for_each_expression"], &mut refs);
            refs.extend(
                resource["depends_on"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_str)
                    .map(str::to_owned),
            );
            let mut qualified: BTreeSet<_> = refs
                .into_iter()
                .map(|r| qualify_reference(scope, &r))
                .collect();
            qualified.extend(inherited.iter().cloned());
            symbols.insert(qualify(scope, address), qualified);
        }
    }
    if let Some(outputs) = module["outputs"].as_object() {
        for (name, output) in outputs {
            let mut refs = BTreeSet::new();
            references(output, &mut refs);
            symbols.insert(
                qualify(scope, &format!("output.{name}")),
                refs.into_iter()
                    .map(|r| qualify_reference(scope, &r))
                    .collect(),
            );
        }
    }
    if let Some(calls) = module["module_calls"].as_object() {
        for (name, call) in calls {
            let child = qualify(scope, &format!("module.{name}"));
            if let Some(inputs) = call["expressions"].as_object() {
                for (name, expression) in inputs {
                    let mut refs = BTreeSet::new();
                    references(expression, &mut refs);
                    symbols.insert(
                        format!("{child}.var.{name}"),
                        refs.into_iter()
                            .map(|r| qualify_reference(scope, &r))
                            .collect(),
                    );
                }
            }
            let mut call_refs = BTreeSet::new();
            references(&call["count_expression"], &mut call_refs);
            references(&call["for_each_expression"], &mut call_refs);
            call_refs.extend(
                call["depends_on"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_str)
                    .map(str::to_owned),
            );
            let mut dependencies = inherited.clone();
            dependencies.extend(call_refs.into_iter().map(|r| qualify_reference(scope, &r)));
            collect_config(&call["module"], &child, &dependencies, symbols);
            if let Some(outputs) = call["module"]["outputs"].as_object() {
                for name in outputs.keys() {
                    symbols.insert(
                        format!("{child}.{name}"),
                        BTreeSet::from([format!("{child}.output.{name}")]),
                    );
                }
            }
        }
    }
}

pub(super) fn prefix_match(reference: &str, key: &str) -> bool {
    reference == key
        || reference
            .strip_prefix(key)
            .is_some_and(|suffix| suffix.starts_with('.'))
}

pub(super) fn resolve(
    nodes: &[TerraformEntity],
    symbols: &BTreeMap<String, BTreeSet<String>>,
) -> Vec<crate::model::TerraformReference> {
    let instances = instance_map(nodes);
    let mut edges = BTreeSet::new();
    for (target, node) in nodes.iter().enumerate() {
        // Configuration describes the current object, not its deposed predecessors.
        if node.deposed_key.is_some() {
            continue;
        }
        if let Some(refs) = symbols.get(&static_address(&node.address)) {
            let refs = refs
                .iter()
                .map(|r| contextualize(r, &node.address))
                .collect();
            let resolution = resolve_sources(&refs, &instances, symbols);
            for source in resolution.sources {
                if source != target {
                    edges.insert((source, target));
                }
            }
        }
    }
    edges
        .into_iter()
        .map(crate::model::TerraformReference::from)
        .collect()
}

pub(super) fn instance_map(nodes: &[TerraformEntity]) -> BTreeMap<String, Vec<usize>> {
    nodes
        .iter()
        .enumerate()
        .filter(|(_, node)| node.deposed_key.is_none())
        .map(|(i, node)| (node.address.clone(), vec![i]))
        .collect()
}

pub(super) fn resolve_sources(
    refs: &BTreeSet<String>,
    instances: &BTreeMap<String, Vec<usize>>,
    symbols: &BTreeMap<String, BTreeSet<String>>,
) -> Resolution {
    resolve_with_policy(refs, instances, symbols, false)
}

pub(super) fn resolve_output_sources(
    refs: &BTreeSet<String>,
    instances: &BTreeMap<String, Vec<usize>>,
    aliases: &BTreeMap<String, BTreeSet<String>>,
) -> Resolution {
    resolve_with_policy(refs, instances, aliases, true)
}

fn resolve_with_policy(
    refs: &BTreeSet<String>,
    instances: &BTreeMap<String, Vec<usize>>,
    symbols: &BTreeMap<String, BTreeSet<String>>,
    exact_sources: bool,
) -> Resolution {
    let mut pending: Vec<_> = refs.iter().cloned().map(|r| (r, false, false)).collect();
    let mut visited = BTreeSet::new();
    let mut active = BTreeSet::new();
    let mut sources = BTreeSet::new();
    let mut complete = !pending.is_empty();
    let mut ids_only = true;
    let mut issues = BTreeSet::new();
    while let Some((reference, exiting, inherited_ambiguity)) = pending.pop() {
        if exiting {
            active.remove(&reference);
            continue;
        }
        if active.contains(&reference) {
            complete = false;
            issues.insert(DiagnosticReason::AliasCycle);
            continue;
        }
        if !visited.insert((reference.clone(), inherited_ambiguity)) {
            continue;
        }
        active.insert(reference.clone());
        pending.push((reference.clone(), true, inherited_ambiguity));
        let caller_module = reference.starts_with(super::address::MODULE_REFERENCE);
        let reference = reference
            .strip_prefix(super::address::MODULE_REFERENCE)
            .unwrap_or(&reference);
        let normalized = static_address(reference);
        if builtin_value(reference) {
            complete = false;
            continue;
        }
        let output_binding = if caller_module {
            let parts = super::address::parts(&normalized);
            let mut end = 0;
            while end + 1 < parts.len() && parts[end] == "module" {
                end += 2;
            }
            (end < parts.len())
                .then(|| parts[..=end].join("."))
                .and_then(|key| symbols.get_key_value(&key))
        } else {
            None
        };
        let binding = output_binding.or_else(|| {
            symbols
                .iter()
                .filter(|(key, _)| prefix_match(&normalized, key))
                .max_by_key(|(key, _)| key.len())
        });
        let resource_key = instances
            .keys()
            .map(|key| static_address(key))
            .filter(|key| output_binding.is_none() && prefix_match(&normalized, key))
            .max_by_key(String::len);
        let metadata = super::address::meta_reference(reference);
        // Module outputs named count/each take precedence over the syntactic
        // metadata heuristic, including outputs whose values have no sources.
        let selection_key = resource_key.as_deref().or_else(|| {
            exact_sources
                .then(|| binding.map(|(key, _)| key.as_str()))
                .flatten()
        });
        let dynamic = if exact_sources {
            super::address::dynamic_selection_with_splats(reference, selection_key, true)
        } else {
            super::address::dynamic_selection(reference, selection_key)
        };
        if dynamic || (metadata && binding.is_none()) {
            issues.insert(DiagnosticReason::DynamicInstanceSelection);
        }
        // Terraform emits iteration metadata as standalone traversals. It is
        // dynamic context, not a missing resource or alias, even in modules.
        if metadata && binding.is_none() {
            complete = false;
            continue;
        }

        if let Some(key) = resource_key {
            // Terraform can split a dynamic selector from its collection traversal.
            // Even one current instance does not prove which element was selected.
            let implicit_collection = exact_sources
                && normalized == key
                && super::address::parts(reference)
                    .last()
                    .is_some_and(|part| !part.contains('['))
                && instances.keys().any(|address| {
                    static_address(address) == key
                        && super::address::parts(address)
                            .last()
                            .is_some_and(|part| part.contains('['))
                });
            if implicit_collection {
                complete = false;
                issues.insert(DiagnosticReason::DynamicInstanceSelection);
            }
            ids_only &= normalized == format!("{key}.id") && reference.ends_with(".id");
            let matching: Vec<_> = instances
                .iter()
                .filter(|(address, _)| {
                    static_address(address) == key && matches_instance(reference, address)
                })
                .collect();
            let ambiguous = matching.len() > 1
                && (!exact_sources
                    || matching
                        .iter()
                        .map(|(address, _)| super::address::selection_group(reference, address))
                        .collect::<BTreeSet<_>>()
                        .len()
                        > 1);
            if matching.is_empty() {
                complete = false;
                issues.insert(DiagnosticReason::UnresolvedReference);
            } else if ambiguous {
                issues.insert(DiagnosticReason::MultipleMatchingInstances);
            }
            if !exact_sources
                || !(dynamic || inherited_ambiguity || implicit_collection || ambiguous)
            {
                sources.extend(
                    matching
                        .into_iter()
                        .flat_map(|(_, indices)| indices.iter().copied()),
                );
            }
            continue;
        }
        match binding {
            Some((key, aliases)) if !aliases.is_empty() => {
                // Selecting a field of an alias is not proof that its underlying
                // resource ID is the selected value.
                ids_only &= normalized == *key;
                // Flattened expression references cannot prove which aggregate
                // field/index supplies a narrowed value, even for static selection.
                let key_parts = super::address::parts(key);
                let module_object = key_parts.len() % 2 == 0
                    && key_parts.chunks_exact(2).all(|pair| pair[0] == "module");
                let narrowed = exact_sources
                    && (normalized != *key
                        || (!module_object
                            && super::address::parts(reference)
                                .get(key_parts.len() - 1)
                                .is_some_and(|part| part.contains('['))));
                if narrowed {
                    complete = false;
                    issues.insert(DiagnosticReason::PartialResourceProvenance);
                }
                pending.extend(aliases.iter().map(|alias| {
                    (
                        contextualize(alias, reference),
                        false,
                        exact_sources && (dynamic || inherited_ambiguity || narrowed),
                    )
                }));
            }
            Some(_) => {
                // A defined constant alias is not missing. It contributes no
                // resource provenance, so semantic inference stays conservative
                // without producing an unresolved-reference diagnostic.
                complete = false;
            }
            None if normalized.starts_with("module.") => {
                if exact_sources {
                    complete = false;
                    issues.insert(DiagnosticReason::UnresolvedReference);
                    continue;
                }
                ids_only = false;
                // A whole-module traversal denotes all matching descendants,
                // not a choice of one endpoint. Multiplicity is expected even
                // when other traversals in the expression remain unresolved.
                let matching: Vec<_> = instances
                    .iter()
                    .filter(|(address, _)| {
                        prefix_match(&static_address(address), &normalized)
                            && matches_instance(reference, address)
                    })
                    .flat_map(|(_, indices)| indices.iter().copied())
                    .collect();
                if matching.is_empty() {
                    complete = false;
                    issues.insert(DiagnosticReason::UnresolvedReference);
                }
                sources.extend(matching);
            }
            _ => {
                complete = false;
                issues.insert(DiagnosticReason::UnresolvedReference);
            }
        }
    }
    Resolution {
        sources: sources.into_iter().collect(),
        complete,
        ids_only,
        issues,
    }
}
