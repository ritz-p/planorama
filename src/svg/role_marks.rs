use crate::services::VisualRole;

pub(super) fn render(role: VisualRole, x: usize, y: usize) -> String {
    let (name, shape) = match role {
        VisualRole::Execution => (
            "execution",
            r#"<rect x="1" y="1" width="20" height="16" rx="4"/>"#,
        ),
        VisualRole::DataStore => (
            "data-store",
            r#"<path d="M1 4 C1 0 21 0 21 4 V14 C21 18 1 18 1 14 Z"/><path d="M1 4 C1 8 21 8 21 4" fill="none"/>"#,
        ),
        VisualRole::Network => ("network", r#"<path d="M5 1 H17 L21 9 L17 17 H5 L1 9 Z"/>"#),
    };
    format!(
        r##"<g data-role-shape="{name}" transform="translate({x} {y})" role="img" aria-label="Visual role: {name}" pointer-events="none" fill="#f8fafc" stroke="#475569" stroke-width="1"><title>Visual role: {name}</title>{shape}</g>"##
    )
}

#[cfg(test)]
#[path = "../../tests/unit/svg/role_marks.rs"]
mod tests;
