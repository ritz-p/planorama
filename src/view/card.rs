use crate::model::Node;

pub(crate) struct Metrics {
    pub inset: usize,
    pub line_limit: usize,
    pub import_height: usize,
    pub previous_y: usize,
    pub previous_line_limit: usize,
    pub references_y: usize,
    pub routes_y: usize,
    pub header_height: usize,
    pub height: usize,
}

pub(crate) fn width(node: &Node) -> usize {
    super::references::width(node).max(node.routes.reserved_width)
}

pub(crate) fn import_height(node: &Node) -> usize {
    if node.metadata.import.is_some() {
        24
    } else {
        0
    }
}

pub(crate) fn measure(node: &Node) -> Metrics {
    let inset = 12
        + if crate::icons::for_node(node).is_some() {
            crate::icons::SIZE + crate::icons::GAP
        } else {
            0
        };
    let width = super::references::width(node);
    let line_limit = (width.saturating_sub(inset + 12) / 8).max(1);
    let address = node.resource_address();
    let lines = super::text::wrap_address(address.terraform(), line_limit)
        .len()
        .max(super::text::wrap_address(&address.qualified(), line_limit).len());
    let import_height = import_height(node);
    let extra = lines.saturating_sub(2) * 20 + import_height;
    let previous_line_limit = (width - 24) / 8;
    let previous_height = node.previous_address.as_deref().map_or(0, |previous| {
        super::text::wrap_address(&super::text::moved_from(previous), previous_line_limit).len()
            * 18
    });
    let header_height =
        crate::layout::NODE_HEIGHT + extra + previous_height + super::references::height(node);
    Metrics {
        inset,
        line_limit,
        import_height,
        previous_y: 86 + extra,
        previous_line_limit,
        references_y: 86 + extra + previous_height,
        routes_y: header_height - 2,
        header_height,
        height: header_height + node.routes.reserved_height,
    }
}
