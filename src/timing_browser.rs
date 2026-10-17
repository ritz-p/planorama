pub(crate) struct Span;

pub(crate) fn span(_: &'static str, _: &[(&'static str, usize)]) -> Span {
    Span
}
