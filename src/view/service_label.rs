pub(crate) const PADDING: usize = 10;
pub(crate) const LINE_HEIGHT: usize = 24;
pub(crate) const FONT_SIZE: usize = 18;
pub(crate) const ADVANCE: usize = 11;
pub(crate) const VERTICAL_PADDING: usize = 4;

pub(crate) fn lines(name: &str, width: usize) -> Vec<&str> {
    super::text::wrap_address(
        crate::services::short_name(name),
        (width.saturating_sub(PADDING * 2) / ADVANCE).max(1),
    )
}

pub(crate) fn width(name: &str, available: usize) -> usize {
    lines(name, available)
        .iter()
        .map(|line| super::text::width(line) * ADVANCE)
        .max()
        .unwrap_or(0)
        + PADDING * 2
}

pub(crate) fn height(name: &str, width: usize) -> usize {
    lines(name, width).len() * LINE_HEIGHT + VERTICAL_PADDING * 2 + 8
}
