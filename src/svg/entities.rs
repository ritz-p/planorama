use crate::model::architecture::{EntityMode, Node};

pub(super) fn border(mode: EntityMode) -> &'static str {
    match mode {
        EntityMode::Data => " data-mode=\"data\" stroke-dasharray=\"6 4\"",
        EntityMode::Managed => " data-mode=\"managed\"",
    }
}

pub(super) fn badge(mode: EntityMode, x: usize, y: usize) -> String {
    match mode {
        EntityMode::Data => format!(
            r##"<g data-external="true"><title>External resource: read by Terraform, not managed by this configuration</title><rect x="{}" y="{}" width="68" height="18" rx="4" fill="#e2e8f0"/><text x="{}" y="{}" font-size="10" fill="#334155">external</text></g>"##,
            x,
            y + 7,
            x + 6,
            y + 20
        ),
        EntityMode::Managed => String::new(),
    }
}

pub(super) fn import_badge(node: &Node, x: usize, y: usize) -> String {
    if node.metadata.import.is_none() {
        return String::new();
    }
    format!(
        r##"<g data-import-badge="true"><title>Imported resource</title><rect x="{x}" y="{y}" width="52" height="18" rx="4" fill="#ede9fe" stroke="#8b5cf6"/><text x="{}" y="{}" font-size="10" fill="#5b21b6">import</text></g>"##,
        x + 6,
        y + 13
    )
}
