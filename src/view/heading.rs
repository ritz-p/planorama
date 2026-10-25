use crate::model::Node;

pub(crate) const LINE_HEIGHT: usize = 18;

pub(crate) fn text(node: &Node, include_import: bool) -> String {
    let operation = node.metadata.state_removal.map_or_else(
        || format!("{:?}", node.action).to_lowercase(),
        |removal| removal.label().into(),
    );
    let import = if include_import && node.metadata.import.is_some() {
        "import / "
    } else {
        ""
    };
    format!("{} · {import}{operation}", node.resource_type)
}

pub(crate) fn lines(text: &str, limit: usize) -> Vec<&str> {
    super::text::wrap_address(text, limit)
}
