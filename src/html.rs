use crate::{layout::Layout, model::Graph, svg, view};
use serde_json::{Value, json};
mod diagnostics;
pub(crate) mod generation;
mod semantic_diagnostics;

pub(crate) fn render(
    graph: &view::ViewGraph,
    layout: &Layout<'_>,
    format: impl Into<svg::Options>,
) -> String {
    document(
        svg::render_content(graph, layout, format, svg::Content::ValueFree),
        resources(graph, "", ""),
        provenance_providers(graph, ""),
        None,
        semantic_diagnostics::bridge(std::iter::once(("", &graph.coverage))),
    )
}

pub(crate) fn render_states(
    architecture: &view::ViewArchitecture,
    format: impl Into<svg::Options>,
) -> String {
    crate::work::complete(render_states_step(architecture, format.into()))
}

pub(crate) async fn render_states_step(
    architecture: &view::ViewArchitecture,
    format: svg::Options,
) -> String {
    let resources = architecture
        .states
        .iter()
        .flat_map(|(state, graph)| resources(graph, state.as_str(), &svg::state_prefix(state)))
        .collect();
    document(
        svg::render_states_step(architecture, format, svg::Content::ValueFree, None).await,
        resources,
        architecture
            .states
            .iter()
            .flat_map(|(state, graph)| provenance_providers(graph, state.as_str()))
            .collect(),
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
        provenance_providers(graph, ""),
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
    crate::work::complete(render_states_report_step(
        architecture,
        format.into(),
        report,
        diagnostics,
    ))
}

pub(crate) async fn render_states_report_step(
    architecture: &view::ViewArchitecture,
    format: svg::Options,
    report: &mut Value,
    diagnostics: bool,
) -> String {
    let resources = architecture
        .states
        .iter()
        .flat_map(|(state, graph)| resources(graph, state.as_str(), &svg::state_prefix(state)))
        .collect();
    document(
        svg::render_states_step(architecture, format, svg::Content::ValueFree, Some(report)).await,
        resources,
        architecture
            .states
            .iter()
            .flat_map(|(state, graph)| provenance_providers(graph, state.as_str()))
            .collect(),
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
    let containment = crate::layout::ContainmentTree::new(graph);
    graph.nodes.iter().enumerate().map(|(index, node)| json!({
        "id": format!("{prefix}{}", svg::resource_id(node)),
        "address": node.address,
        "qualified": node.resource_address().qualified(),
        "identity": node.entity.id.as_str(),
        "module": node.module,
        "state": state,
        "type": node.resource_type,
        "provider": node.provider.source(),
        "mode": match node.mode { crate::model::EntityMode::Data => "data", crate::model::EntityMode::Managed => "managed" },
        "deposed": node.deposed_key,
        "action": format!("{:?}", node.action).to_lowercase(),
        "context": containment.parents[index].into_iter().chain(crate::view::routes::owner(graph, node))
            .map(|parent| format!("{prefix}{}", svg::resource_id(&graph.nodes[parent]))).collect::<Vec<_>>(),
        "scope": node.entity.scope.as_ref().map(|scope| json!({
            "provider": scope.provider,
            "account": scope.account.as_ref().map(|account| account.as_str()),
            "region": scope.region.as_ref().map(|region| &region.0),
            "global": scope.global,
        })),
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

fn provenance_providers(graph: &Graph, state: &str) -> Vec<Value> {
    graph
        .relationships
        .iter()
        .flat_map(|relationship| &relationship.provenance)
        .filter_map(|source| {
            let crate::model::RelationshipProvenance::Resource {
                source, deployment, ..
            } = source
            else {
                return None;
            };
            Some(json!([
                state,
                source.address,
                source.deposed_key,
                deployment
                    .as_ref()
                    .and_then(|deployment| deployment.provider.source())
            ]))
        })
        .collect()
}

fn document(
    svg: String,
    resources: Vec<Value>,
    providers: Vec<Value>,
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
                    include_str!("html/generation.js"),
                    "\n",
                    include_str!("html/metadata.js"),
                    include_str!("html/export.js"),
                    include_str!("html/neighborhood.js"),
                    include_str!("html/export-presentation.js"),
                    "\n",
                    include_str!("html/navigation.js"),
                    "\n",
                    include_str!("html/highlighting.js"),
                    "\n",
                    include_str!("html/selection.js"),
                    "\n",
                    include_str!("html/selection-history.js"),
                    "\n",
                    include_str!("html/searching.js"),
                    include_str!("html/action-filtering.js"),
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
        .replace(
            "<!-- VIEWER_PROVIDER_DATA -->",
            &safe_json(&Value::Array(providers)),
        )
        .replace("<!-- VIEWER_DIAGNOSTICS -->", &diagnostics)
        .replace("<!-- VIEWER_SEMANTIC_DIAGNOSTICS -->", &semantic)
        .replace("<!-- DIAGRAM -->", &svg)
}
