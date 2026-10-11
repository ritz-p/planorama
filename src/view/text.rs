pub(crate) fn wrap(text: &str, limit: usize) -> Vec<&str> {
    let mut lines = Vec::new();
    let mut start = 0;
    let mut width = 0;
    for (offset, character) in text.char_indices() {
        let advance = if character.is_ascii() { 1 } else { 2 };
        if width > 0 && width + advance > limit {
            lines.push(&text[start..offset]);
            start = offset;
            width = 0;
        }
        width += advance;
    }
    lines.push(&text[start..]);
    lines
}
