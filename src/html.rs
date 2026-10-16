use crate::{layout::Layout, model::Graph, svg, view};
use serde_json::{Value, json};
mod diagnostics;
mod semantic_diagnostics;

pub(crate) fn render(
    graph: &view::ViewGraph,
    layout: &Layout<'_>,
    format: impl Into<svg::Options>,
) -> String {
    document(
        svg::render_content(graph, layout, format, svg::Content::ValueFree),
        resources(graph, "", ""),
        None,
        semantic_diagnostics::bridge(std::iter::once(("", &graph.coverage))),
    )
}

pub(crate) fn render_states(
    architecture: &view::ViewArchitecture,
    format: impl Into<svg::Options>,
) -> String {
    let resources = architecture
        .states
        .iter()
        .flat_map(|(state, graph)| resources(graph, state.as_str(), &svg::state_prefix(state)))
        .collect();
    document(
        svg::render_states_content(architecture, format, svg::Content::ValueFree),
        resources,
        None,
        semantic_diagnostics::bridge(
            architecture
                .states
                .iter()
                .map(|(state, graph)| (state.as_str(), &graph.coverage)),
        ),
    )
}

pub(crate) fn render_report(
    graph: &view::ViewGraph,
    layout: &Layout<'_>,
    format: impl Into<svg::Options>,
    report: &mut Value,
    diagnostics: bool,
) -> String {
    document(
        svg::render_report(graph, layout, format, svg::Content::ValueFree, Some(report)),
        resources(graph, "", ""),
        diagnostics.then(|| diagnostics::bridge(report)),
        semantic_diagnostics::bridge(std::iter::once(("", &graph.coverage))),
    )
}

pub(crate) fn render_states_report(
    architecture: &view::ViewArchitecture,
    format: impl Into<svg::Options>,
    report: &mut Value,
    diagnostics: bool,
) -> String {
    let resources = architecture
        .states
        .iter()
        .flat_map(|(state, graph)| resources(graph, state.as_str(), &svg::state_prefix(state)))
        .collect();
    document(
        svg::render_states_report(architecture, format, svg::Content::ValueFree, Some(report)),
        resources,
        diagnostics.then(|| diagnostics::bridge(report)),
        semantic_diagnostics::bridge(
            architecture
                .states
                .iter()
                .map(|(state, graph)| (state.as_str(), &graph.coverage)),
        ),
    )
}

fn resources(graph: &Graph, state: &str, prefix: &str) -> Vec<Value> {
    graph.nodes.iter().map(|node| json!({
        "id": format!("{prefix}{}", svg::resource_id(node)),
        "address": node.address,
        "qualified": node.resource_address().qualified(),
        "identity": node.entity.id.as_str(),
        "module": node.module,
        "state": state,
        "type": node.resource_type,
        "mode": match node.mode { crate::model::EntityMode::Data => "data", crate::model::EntityMode::Managed => "managed" },
        "deposed": node.deposed_key,
        "action": format!("{:?}", node.action).to_lowercase(),
    })).collect()
}

fn safe_json(value: &Value) -> String {
    value
        .to_string()
        .replace('&', "\\u0026")
        .replace('<', "\\u003c")
        .replace('>', "\\u003e")
        .replace('\u{2028}', "\\u2028")
        .replace('\u{2029}', "\\u2029")
}

fn document(
    svg: String,
    resources: Vec<Value>,
    diagnostics: Option<Value>,
    semantic: Option<Value>,
) -> String {
    let data = safe_json(&Value::Array(resources));
    let semantic = semantic
        .map(|value| {
            format!(
                "<script id=\"viewer-semantic-diagnostics\" type=\"application/json\">{}</script>",
                safe_json(&value)
            )
        })
        .unwrap_or_default();
    let diagnostics = diagnostics
        .map(|value| {
            format!(
                "<script id=\"viewer-diagnostics\" type=\"application/json\">{}</script>",
                safe_json(&value)
            )
        })
        .unwrap_or_default();
    include_str!("html/viewer.html")
        .replace("/* VIEWER_CSS */", include_str!("html/viewer.css"))
        .replace(
            "/* VIEWER_JS */",
            &include_str!("html/viewer.js").replace(
                "/* VIEWER_MODULES */",
                concat!(
                    include_str!("html/metadata.js"),
                    "\n",
                    include_str!("html/navigation.js"),
                    "\n",
                    include_str!("html/highlighting.js"),
                    "\n",
                    include_str!("html/selection.js"),
                    "\n",
                    include_str!("html/searching.js"),
                    "\n",
                    include_str!("html/lifecycle.js"),
                    "\n",
                    include_str!("html/diagnostics.js"),
                    "\n",
                    include_str!("html/semantic-diagnostics.js"),
                    "\n",
                    include_str!("html/impact-graph.js"),
                    "\n",
                    include_str!("html/impact.js"),
                ),
            ),
        )
        .replace("<!-- VIEWER_DATA -->", &data)
        .replace("<!-- VIEWER_DIAGNOSTICS -->", &diagnostics)
        .replace("<!-- VIEWER_SEMANTIC_DIAGNOSTICS -->", &semantic)
        .replace("<!-- DIAGRAM -->", &svg)
}
