use crate::model::{
    MultiPlan,
    cross_state::{CrossStateProvenance, Resolution},
};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;

pub(super) fn render(plans: &MultiPlan, resolution: &Resolution, anonymous: bool) -> String {
    let mut rows = BTreeMap::new();
    for (i, id) in plans.states.keys().enumerate() {
        let label = if anonymous {
            format!("State {}", i + 1)
        } else {
            id.as_str().to_owned()
        };
        rows.insert(id, (i, label));
    }
    let mut pairs = BTreeMap::<_, BTreeSet<_>>::new();
    for edge in &resolution.edges {
        let CrossStateProvenance::TerraformRemoteState { remote, output, .. } = &edge.provenance;
        pairs
            .entry((&edge.to.state, &edge.from.state))
            .or_default()
            .insert((&edge.to.address, remote, output));
    }
    let mut unresolved = BTreeMap::new();
    for diagnostic in &resolution.diagnostics {
        *unresolved
            .entry((&diagnostic.consumer.state, diagnostic.reason))
            .or_insert(0usize) += 1;
    }
    let card = rows
        .values()
        .map(|(_, label)| label.len() * 9 + 32)
        .max()
        .unwrap_or(100)
        .max(140);
    let right = card + 16;
    let width = (right + pairs.len() * 56 + 64).max(card + 400);
    let footer = 76 + rows.len() * 70;
    let height = footer + 46 + 24 * unresolved.len().max(1);
    let mut svg = format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}" font-family="sans-serif" fill="#1e293b"><title>Remote-state dependencies</title><defs><marker id="arrow" viewBox="0 0 8 8" refX="8" refY="4" markerWidth="6" markerHeight="6" orient="auto"><path d="M0 0 L8 4 L0 8 Z" fill="#475569"/></marker></defs><rect width="100%" height="100%" fill="white"/><text x="16" y="30" font-size="20">Remote-state dependencies</text><text x="16" y="52" font-size="14">Consumer → provider · distinct references</text>"##
    );
    for (i, label) in rows.values() {
        let y = 70 + i * 70;
        write!(svg, r##"<g><rect x="16" y="{y}" width="{card}" height="44" rx="6" fill="#eff6ff" stroke="#64748b"/><text x="28" y="{}" font-family="monospace" font-size="14">{label}</text></g>"##, y + 28).unwrap();
    }
    for (lane, ((consumer, provider), references)) in pairs.iter().enumerate() {
        let (a, from) = &rows[consumer];
        let (b, to) = &rows[provider];
        let start = 82 + a * 70;
        let end = 102 + b * 70;
        let x = right + (lane + 1) * 56;
        let mid = (start + end) / 2;
        let count = references.len();
        write!(svg, r##"<g><title>{from} → {to}: {count}</title><path d="M{right} {start} H{x} V{end} H{right}" fill="none" stroke="#475569" stroke-width="2" marker-end="url(#arrow)"/><text x="{x}" y="{mid}" text-anchor="middle" stroke="white" stroke-width="5" paint-order="stroke">{count}</text></g>"##).unwrap();
    }
    write!(
        svg,
        r#"<text x="16" y="{footer}" font-weight="bold">Unresolved references</text>"#
    )
    .unwrap();
    if unresolved.is_empty() {
        write!(svg, r#"<text x="16" y="{}">None</text>"#, footer + 24).unwrap();
    }
    for (i, ((state, reason), count)) in unresolved.iter().enumerate() {
        write!(
            svg,
            r#"<text x="16" y="{}">{} · {}: {count}</text>"#,
            footer + 24 * (i + 1),
            rows[state].1,
            reason.description()
        )
        .unwrap();
    }
    svg.push_str("</svg>");
    svg
}
