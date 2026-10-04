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

#[cfg(test)]
#[path = "../../tests/unit/plan/address.rs"]
mod tests;
