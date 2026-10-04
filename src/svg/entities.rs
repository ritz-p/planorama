use crate::model::EntityMode;

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
            x + 242,
            y + 7,
            x + 248,
            y + 20
        ),
        EntityMode::Managed => String::new(),
    }
}

pub(super) fn type_limit(mode: EntityMode) -> usize {
    match mode {
        EntityMode::Data => 18,
        EntityMode::Managed => 25,
    }
}
