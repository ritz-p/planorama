pub(super) fn static_address(address: &str) -> String {
    let mut result = String::new();
    let (mut depth, mut quoted, mut escaped) = (0usize, false, false);
    for c in address.chars() {
        match (depth, escaped, quoted, c) {
            (0, _, _, '[') => depth = 1,
            (0, _, _, c) => result.push(c),
            (_, true, _, _) => escaped = false,
            (_, _, true, '\\') => escaped = true,
            (_, _, _, '"') => quoted = !quoted,
            (_, _, false, '[') => depth += 1,
            (_, _, false, ']') => depth -= 1,
            _ => {}
        }
    }
    result
}

pub(super) fn module_of(address: &str) -> String {
    crate::model::module_of(address).to_owned()
}

pub(super) fn parts(address: &str) -> Vec<&str> {
    let (mut start, mut depth, mut quoted, mut escaped) = (0, 0usize, false, false);
    let mut result = Vec::new();
    for (i, c) in address.char_indices() {
        match (escaped, quoted, c) {
            (true, _, _) => escaped = false,
            (_, true, '\\') => escaped = true,
            (_, _, '"') if depth > 0 => quoted = !quoted,
            (_, false, '[') => depth += 1,
            (_, false, ']') => depth = depth.saturating_sub(1),
            (_, false, '.') if depth == 0 => {
                result.push(&address[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    result.push(&address[start..]);
    result
}

fn name(part: &str) -> &str {
    part.split('[').next().unwrap_or(part)
}

fn known_key(part: &str) -> Option<ValueKey> {
    let index = part
        .strip_prefix(name(part))?
        .strip_prefix('[')?
        .strip_suffix(']')?;
    match serde_json::from_str::<serde_json::Value>(index).ok()? {
        serde_json::Value::String(key) => Some(ValueKey::String(key)),
        serde_json::Value::Number(key) => key.as_u64().map(ValueKey::Number),
        _ => None,
    }
}

#[derive(PartialEq)]
enum ValueKey {
    String(String),
    Number(u64),
}

pub(super) fn dynamic_selection(reference: &str, resource_address: Option<&str>) -> bool {
    dynamic_selection_with_splats(reference, resource_address, false)
}

pub(super) fn full_splat(part: &str) -> bool {
    part.strip_prefix(name(part)) == Some("[*]")
}

pub(super) fn dynamic_selection_with_splats(
    reference: &str,
    resource_address: Option<&str>,
    allow_splats: bool,
) -> bool {
    let traversal = parts(reference);
    let mut module_end = 0;
    while module_end + 1 < traversal.len() && traversal[module_end] == "module" {
        module_end += 2;
    }
    let end = resource_address.map_or(module_end, |address| parts(address).len());
    traversal.iter().take(end).any(|part| {
        part.contains('[') && known_key(part).is_none() && !(allow_splats && full_splat(part))
    })
}

pub(super) fn selection_group(reference: &str, instance: &str) -> String {
    parts(reference)
        .iter()
        .zip(parts(instance))
        .map(|(reference, instance)| {
            if full_splat(reference) {
                name(instance)
            } else {
                instance
            }
        })
        .collect::<Vec<_>>()
        .join(".")
}

pub(super) fn meta_reference(reference: &str) -> bool {
    let parts = parts(reference);
    let local = parts.as_slice();
    matches!(local, ["count", "index"] | ["each", "key"])
        || matches!(local, ["each", value, ..] if name(value) == "value")
}

pub(super) fn matches_instance(reference: &str, instance: &str) -> bool {
    parts(reference)
        .iter()
        .zip(parts(instance))
        .all(|(reference, instance)| {
            name(reference) == name(instance)
                && known_key(reference).is_none_or(|key| Some(key) == known_key(instance))
        })
}

pub(super) fn contextualize(reference: &str, context: &str) -> String {
    if let Some(reference) = reference.strip_prefix(MODULE_REFERENCE) {
        return format!("{MODULE_REFERENCE}{}", contextualize(reference, context));
    }
    let mut reference = parts(reference);
    let context = parts(context);
    let mut i = 0;
    while i + 1 < reference.len()
        && i + 1 < context.len()
        && reference[i] == "module"
        && context[i] == "module"
        && name(reference[i + 1]) == name(context[i + 1])
    {
        if reference[i + 1].contains('[') {
            if reference[i + 1] != context[i + 1] {
                break;
            }
        } else {
            reference[i + 1] = context[i + 1];
        }
        i += 2;
    }
    reference.join(".")
}

pub(super) const MODULE_REFERENCE: &str = "@module:";

#[cfg(test)]
#[path = "../../tests/unit/plan/address.rs"]
mod tests;
