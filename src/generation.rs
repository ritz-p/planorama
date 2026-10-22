use crate::{html, layout::Layout, plan, semantic, svg, view};

#[derive(Clone, Copy, Default, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OutputFormat {
    #[default]
    Svg,
    Html,
}

#[derive(Default, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Options {
    pub spatial_types: Vec<String>,
    pub format: OutputFormat,
    pub address_format: svg::AddressFormat,
    pub icon_mode: crate::icons::IconMode,
    pub hide_relationship_legend: bool,
    pub changed_only: bool,
    pub focus: Option<String>,
    pub focus_depth: Option<usize>,
    pub modules: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Error {
    InputTooLarge,
    InvalidJson,
    UnsupportedInput,
    InvalidOptions,
    SelectionFailed,
    GenerationFailed,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::InputTooLarge => "Input exceeds the 32 MiB limit.",
            Self::InvalidJson => "Input is not valid plan JSON.",
            Self::UnsupportedInput => "Expected one terraform show -json saved plan (format 1.x).",
            Self::InvalidOptions => "Unsupported or invalid generation options.",
            Self::SelectionFailed => "The requested selection could not be resolved.",
            Self::GenerationFailed => "Diagram generation failed.",
        })
    }
}

impl std::error::Error for Error {}

#[derive(serde::Serialize)]
pub struct Generated {
    pub content: String,
    pub diagnostics: Vec<String>,
}

pub const MAX_INPUT_BYTES: usize = 32 * 1024 * 1024;

pub fn generate(json: &str, options: &Options) -> Result<Generated, Error> {
    if json.len() > MAX_INPUT_BYTES {
        return Err(Error::InputTooLarge);
    }
    let filter = view::Options {
        spatial_types: options.spatial_types.clone(),
        changed_only: options.changed_only,
        focus: options.focus.clone(),
        depth: options.focus_depth,
        modules: options.modules.clone(),
        icon_mode: options.icon_mode,
        ..Default::default()
    };
    filter.validate().map_err(|_| Error::InvalidOptions)?;
    let raw = parse_input(json)?;
    let diagnostics = semantic::diagnostics::collect(&raw)
        .into_iter()
        .map(|diagnostic| diagnostic.reason.description().to_owned())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    let graph = project(&raw, &filter).map_err(|_| Error::SelectionFailed)?;
    let layout = Layout::new(&graph);
    let content = render(
        &graph,
        &layout,
        options.format,
        svg::Options {
            address_format: options.address_format,
            hide_relationship_legend: options.hide_relationship_legend,
        },
        None,
        false,
    );
    Ok(Generated {
        content,
        diagnostics,
    })
}

pub(crate) fn parse_input(json: &str) -> Result<crate::model::TerraformPlan, Error> {
    if json.len() > MAX_INPUT_BYTES {
        return Err(Error::InputTooLarge);
    }
    plan::parse(json).map_err(|error| {
        if error.starts_with("invalid JSON:") {
            Error::InvalidJson
        } else {
            Error::UnsupportedInput
        }
    })
}

pub(crate) fn project(
    raw: &crate::model::TerraformPlan,
    filter: &view::Options,
) -> Result<view::ViewGraph, String> {
    view::single(&semantic::transform(raw), filter)
}

pub(crate) fn render(
    graph: &view::ViewGraph,
    layout: &Layout<'_>,
    format: OutputFormat,
    options: svg::Options,
    report: Option<&mut serde_json::Value>,
    html_diagnostics: bool,
) -> String {
    let _timing = crate::timing::span("composition_serialization", &[]);
    match (format, report) {
        (OutputFormat::Html, Some(report)) => {
            html::render_report(graph, layout, options, report, html_diagnostics)
        }
        (OutputFormat::Svg, Some(report)) => {
            svg::render_report(graph, layout, options, svg::Content::Standard, Some(report))
        }
        (OutputFormat::Html, None) => html::render(graph, layout, options),
        (OutputFormat::Svg, None)
            if matches!(options.address_format, svg::AddressFormat::Qualified)
                && !options.hide_relationship_legend =>
        {
            svg::render(graph, layout)
        }
        (OutputFormat::Svg, None) => svg::render_with_format(graph, layout, options),
    }
}

#[cfg(test)]
#[path = "../tests/unit/generation.rs"]
mod tests;
