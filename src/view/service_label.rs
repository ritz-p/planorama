pub(crate) const PADDING: usize = 6;
pub(crate) const LINE_HEIGHT: usize = 20;

pub(crate) fn lines(name: &str, width: usize) -> Vec<&str> {
    super::text::wrap(
        crate::services::short_name(name),
        (width.saturating_sub(PADDING * 2) / 8).max(1),
    )
}

pub(crate) fn width(name: &str, available: usize) -> usize {
    (super::text::width(crate::services::short_name(name)) * 8 + PADDING * 2).min(available)
}

pub(crate) fn height(name: &str, width: usize) -> usize {
    lines(name, width).len() * LINE_HEIGHT
}
