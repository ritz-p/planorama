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
        0 => "root".into(),
        _ => address[..parts[count - 1].1].into(),
    }
}

#[cfg(test)]
#[path = "../../tests/unit/plan/address.rs"]
mod tests;
