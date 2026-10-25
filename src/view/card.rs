use crate::model::Node;

pub(crate) struct Metrics {
    pub inset: usize,
    pub line_limit: usize,
    pub import_height: usize,
    pub service_height: usize,
    pub service_width: usize,
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

pub(crate) fn role_mark(node: &Node) -> Option<crate::services::VisualRole> {
    if node.placement != crate::model::Placement::Spatial
        || node.service_name.is_none()
        || node.member_owner.is_some()
        || node.routes.reserved_height != 0
    {
        return None;
    }
    crate::services::visual_role(node)
}

pub(crate) fn measure(node: &Node) -> Metrics {
    let inset = 12
        + if node.service_name.is_none() && crate::icons::for_node(node).is_some() {
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
    let service_width = line_limit * 8 - if role_mark(node).is_some() { 32 } else { 0 };
    let service_height = crate::services::display_name(node)
        .map_or(0, |name| super::service_label::height(name, service_width));
    let extra = lines.saturating_sub(2) * 20 + import_height + service_height;
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
        service_height,
        service_width,
        previous_y: 86 + extra,
        previous_line_limit,
        references_y: 86 + extra + previous_height,
        routes_y: header_height - 2,
        header_height,
        height: header_height + node.routes.reserved_height,
    }
}
