pub(crate) fn wrap(text: &str, limit: usize) -> Vec<&str> {
    wrap_with_boundaries(text, limit, false)
}

pub(crate) fn wrap_address(text: &str, limit: usize) -> Vec<&str> {
    wrap_with_boundaries(text, limit, true)
}

pub(crate) fn width(text: &str) -> usize {
    text.chars().map(advance).sum()
}

fn advance(character: char) -> usize {
    if character.is_ascii() { 1 } else { 2 }
}

fn wrap_with_boundaries(text: &str, limit: usize, address: bool) -> Vec<&str> {
    let mut lines = Vec::new();
    let mut start = 0;
    let mut width = 0;
    let mut boundary = None;
    let (mut depth, mut quoted, mut escaped) = (0usize, false, false);
    for (offset, character) in text.char_indices() {
        let advance = advance(character);
        while width > 0 && width + advance > limit {
            if let Some((end, preceding_width)) = boundary.take() {
                lines.push(&text[start..end]);
                start = end;
                width -= preceding_width;
            } else {
                lines.push(&text[start..offset]);
                start = offset;
                width = 0;
            }
        }
        width += advance;
        if address {
            match (escaped, quoted, character) {
                (true, _, _) => escaped = false,
                (_, true, '\\') => escaped = true,
                (_, _, '"') if depth > 0 => quoted = !quoted,
                (_, false, '[') => depth += 1,
                (_, false, ']') => depth = depth.saturating_sub(1),
                (_, false, '.' | ':') if depth == 0 && width > advance => {
                    boundary = Some((offset + character.len_utf8(), width));
                }
                (_, false, c) if depth == 0 && c.is_whitespace() && width > advance => {
                    boundary = Some((offset + character.len_utf8(), width));
                }
                _ => {}
            }
        }
    }
    lines.push(&text[start..]);
    lines
}

#[cfg(test)]
#[path = "../../tests/unit/view/text.rs"]
mod tests;
