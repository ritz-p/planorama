use crate::model::Node;

pub(crate) const LINE_HEIGHT: usize = 18;

pub(crate) fn type_name(node: &Node, has_label: bool) -> Option<&str> {
    if has_label
        && matches!(
            crate::services::display_label(node),
            Some(crate::services::DisplayLabel::Resource(_))
        )
    {
        None
    } else {
        Some(&node.resource_type)
    }
}

pub(crate) fn text(node: &Node, include_import: bool, has_label: bool) -> String {
    let operation = node.metadata.state_removal.map_or_else(
        || format!("{:?}", node.action).to_lowercase(),
        |removal| removal.label().into(),
    );
    let import = if include_import && node.metadata.import.is_some() {
        "import / "
    } else {
        ""
    };
    match type_name(node, has_label) {
        Some(name) => format!("{name} · {import}{operation}"),
        None => format!("{import}{operation}"),
    }
}

pub(crate) fn lines(text: &str, limit: usize) -> Vec<&str> {
    super::text::wrap_address(text, limit)
}
