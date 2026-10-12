use crate::model::Node;

pub(crate) struct Metrics {
    pub inset: usize,
    pub line_limit: usize,
    pub references_y: usize,
    pub routes_y: usize,
    pub header_height: usize,
    pub height: usize,
}

pub(crate) fn measure(node: &Node) -> Metrics {
    let inset = 12
        + if crate::icons::for_node(node).is_some() {
            crate::icons::SIZE + crate::icons::GAP
        } else {
            0
        };
    let line_limit = (super::references::width(node).saturating_sub(inset + 12) / 8).max(1);
    let address = node.resource_address();
    let lines = super::text::wrap_address(address.terraform(), line_limit)
        .len()
        .max(super::text::wrap_address(&address.qualified(), line_limit).len());
    let extra = lines.saturating_sub(2) * 20;
    let header_height = crate::layout::NODE_HEIGHT + extra + super::references::height(node);
    Metrics {
        inset,
        line_limit,
        references_y: 86 + extra,
        routes_y: header_height - 2,
        header_height,
        height: header_height + node.routes.reserved_height,
    }
}
