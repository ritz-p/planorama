use crate::layout::Layout;
use std::fmt::Write;

pub(super) fn render(layout: &Layout<'_>) -> String {
    let mut svg = String::new();
    for panel in &layout.scopes {
        let bounds = panel.bounds;
        let provider = super::escape(&panel.scope.provider);
        let region = panel.scope.region.as_ref().map(|r| r.0.as_str());
        let scope = region
            .map(|r| format!("data-region=\"{}\"", super::escape(r)))
            .unwrap_or_else(|| "data-global=\"true\"".into());
        let label = format!(
            "{provider} {}",
            region.map(super::escape).unwrap_or_else(|| "Global".into())
        );
        writeln!(svg, r##"<g data-deployment-scope="true" data-scope-provider="{provider}" {scope}><rect x="{}" y="{}" width="{}" height="{}" rx="14" fill="#f8fafc" stroke="#94a3b8" stroke-dasharray="9 5"/><text x="{}" y="{}" font-size="15" font-weight="700" fill="#334155">{label}</text></g>"##, bounds.origin.x, bounds.origin.y, bounds.width, bounds.height, bounds.origin.x + 16, bounds.origin.y + 25).unwrap();
    }
    svg
}
