use crate::{html, layout::Layout, plan, semantic, svg, view};
pub(crate) mod budget;
pub(crate) mod memory;
pub(crate) mod progress;

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
    #[serde(deserialize_with = "memory::deserialize")]
    pub memory_budget_mib: Option<u32>,
    pub spatial_types: Vec<String>,
    pub format: OutputFormat,
    pub address_format: svg::AddressFormat,
    pub icon_mode: crate::icons::IconMode,
    pub hide_relationship_legend: bool,
    pub omit_relationship_legend: bool,
    pub html_diagnostics: bool,
    pub html_semantic_diagnostics: bool,
    pub changed_only: bool,
    pub focus: Option<String>,
    pub focus_depth: Option<usize>,
    pub modules: Vec<String>,
    pub remote_state_mappings: Vec<crate::RemoteStateMapping>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Error {
    InputTooLarge,
    InvalidJson,
    UnsupportedInput,
    InvalidOptions,
    InvalidStateName,
    DuplicateStateName,
    EmptyInputs,
    InvalidRemoteStateMapping,
    UnknownRemoteStateMapping,
    AmbiguousRemoteStateMapping,
    SelectionFailed,
    GenerationFailed,
    GenerationClosed,
    ResourceLimit,
    MemoryLimit,
    MemoryBudgetMismatch,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::InputTooLarge => "Input exceeds the 32 MiB limit.",
            Self::InvalidJson => "Input is not valid plan JSON.",
            Self::UnsupportedInput => "Expected one terraform show -json saved plan (format 1.x).",
            Self::InvalidOptions => "Unsupported or invalid generation options.",
            Self::InvalidStateName => "State names must contain 1–64 ASCII letters, digits, '.', '_' or '-'.",
            Self::DuplicateStateName => "Each input must have a unique state name.",
            Self::EmptyInputs => "Select at least one named input (at most 64).",
            Self::InvalidRemoteStateMapping => "Expected an exact, unique Terraform remote-state data source in the consumer state.",
            Self::UnknownRemoteStateMapping => "Both mapped states must be loaded named inputs.",
            Self::AmbiguousRemoteStateMapping => "A remote-state data source cannot map to different producer states.",
            Self::SelectionFailed => "The requested selection could not be resolved.",
            Self::GenerationFailed => "Diagram generation failed.",
            Self::GenerationClosed => "Generation has completed or was cancelled.",
            Self::ResourceLimit => "The browser generation budget was exceeded. Reduce the input or select fewer modules or a narrower focus.",
            Self::MemoryLimit => "The generation memory budget was exceeded. Retry in a new Worker with a larger memory budget or smaller input.",
            Self::MemoryBudgetMismatch => "The generation memory budget must match initialization. Use a new Worker to change it.",
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
pub const MAX_BROWSER_INPUT_BYTES: usize = 2 * 1024 * 1024;
pub const MAX_BROWSER_OUTPUT_BYTES: usize = 4 * 1024 * 1024;
pub(crate) const MAX_BROWSER_OPTIONS_BYTES: usize = 64 * 1024;

pub fn generate(json: &str, options: &Options) -> Result<Generated, Error> {
    if options.memory_budget_mib.is_some() {
        return Err(Error::InvalidOptions);
    }
    crate::work::complete(generate_step(json, options))
}

async fn generate_step(json: &str, options: &Options) -> Result<Generated, Error> {
    validate_option_sizes(options)?;
    if !options.remote_state_mappings.is_empty() {
        return Err(Error::InvalidOptions);
    }
    if (options.html_diagnostics || options.html_semantic_diagnostics)
        && !matches!(options.format, OutputFormat::Html)
    {
        return Err(Error::InvalidOptions);
    }
    progress::report("parsing");
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
        html_semantic_diagnostics: options.html_semantic_diagnostics,
        ..Default::default()
    };
    filter.validate().map_err(|_| Error::InvalidOptions)?;
    crate::work::checkpoint().await;
    if crate::work::limited() {
        budget::validate(json)?;
        crate::work::checkpoint().await;
    }
    let raw = parse_input(json)?;
    crate::work::check_limit(raw.nodes.len(), 256).await;
    crate::work::check_limit(raw.edges.len(), 1024).await;
    crate::work::checkpoint().await;
    progress::report("selection");
    let diagnostics = semantic::diagnostics::collect(&raw)
        .into_iter()
        .map(|diagnostic| diagnostic.reason.description().to_owned())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    let graph = project(&raw, &filter).map_err(|_| Error::SelectionFailed)?;
    drop(raw);
    crate::work::checkpoint().await;
    crate::work::check_limit(graph.nodes.len(), 256).await;
    crate::work::check_limit(graph.edges.len(), 1024).await;
    progress::report("placement");
    let layout = Layout::new_step(&graph).await;
    crate::work::checkpoint().await;
    progress::report("output");
    let mut report = serde_json::Value::Null;
    let content = render(
        &graph,
        &layout,
        options.format,
        svg::Options {
            address_format: options.address_format,
            hide_relationship_legend: options.hide_relationship_legend,
            omit_relationship_legend: options.omit_relationship_legend,
        },
        options.html_diagnostics.then_some(&mut report),
        options.html_diagnostics,
    );
    crate::work::check_limit(content.len(), MAX_BROWSER_OUTPUT_BYTES).await;
    Ok(Generated {
        content,
        diagnostics,
    })
}

type GenerationFuture =
    std::pin::Pin<Box<dyn std::future::Future<Output = Result<Generated, Error>>>>;

pub struct Generator {
    future: Option<GenerationFuture>,
    stage: std::rc::Rc<std::cell::Cell<&'static str>>,
    search_remaining: usize,
}

impl Generator {
    pub fn new(json: String, options: Options) -> Result<Self, Error> {
        Self::validate_options(&options)?;
        if !options.remote_state_mappings.is_empty() {
            return Err(Error::InvalidOptions);
        }
        if json.len() > MAX_BROWSER_INPUT_BYTES {
            return Err(Error::ResourceLimit);
        }
        Ok(Self::from_future(Box::pin(async move {
            generate_step(&json, &options).await
        })))
    }

    pub fn new_named(inputs: Vec<crate::NamedInput>, options: Options) -> Result<Self, Error> {
        Self::validate_options(&options)?;
        crate::named_inputs::validate(&inputs, MAX_BROWSER_INPUT_BYTES, Error::ResourceLimit)?;
        Ok(Self::from_future(Box::pin(async move {
            crate::named_inputs::generate_step(&inputs, &options).await
        })))
    }

    fn validate_options(options: &Options) -> Result<(), Error> {
        #[cfg(target_arch = "wasm32")]
        memory::check_options(options.memory_budget_mib)?;
        #[cfg(not(target_arch = "wasm32"))]
        if options.memory_budget_mib.is_some() {
            return Err(Error::InvalidOptions);
        }
        validate_option_sizes(options)
    }

    fn from_future(future: GenerationFuture) -> Self {
        Self {
            future: Some(future),
            stage: std::rc::Rc::new(std::cell::Cell::new("parsing")),
            search_remaining: 64 * 1024 * 1024,
        }
    }

    pub fn stage(&self) -> &'static str {
        self.stage.get()
    }

    pub fn step(&mut self, units: usize) -> Result<Option<Generated>, Error> {
        self.step_with_progress(units, |_| {})
    }

    pub fn step_with_progress(
        &mut self,
        units: usize,
        observer: impl Fn(&'static str) + 'static,
    ) -> Result<Option<Generated>, Error> {
        if units == 0 || units > 4096 {
            return Err(Error::InvalidOptions);
        }
        let future = self.future.as_mut().ok_or(Error::GenerationClosed)?;
        let stage = self.stage.clone();
        let result = progress::observe(
            std::rc::Rc::new(move |value| {
                stage.set(value);
                observer(value);
            }),
            || {
                crate::work::with_search_budget(&mut self.search_remaining, || {
                    crate::work::step(future.as_mut(), units, true)
                })
            },
        );
        match result {
            Ok(std::task::Poll::Pending) => Ok(None),
            Ok(std::task::Poll::Ready(result)) => {
                self.future = None;
                result.map(Some)
            }
            Err(error) => {
                self.future = None;
                Err(error)
            }
        }
    }

    pub fn cancel(&mut self) {
        self.future = None;
    }
}

pub(crate) fn validate_option_sizes(options: &Options) -> Result<(), Error> {
    if options.modules.len() > 256
        || options.spatial_types.len() > 256
        || options.remote_state_mappings.len() > 256
    {
        return Err(Error::ResourceLimit);
    }
    let mut remaining = MAX_BROWSER_OPTIONS_BYTES;
    for value in options
        .modules
        .iter()
        .chain(&options.spatial_types)
        .chain(options.focus.iter())
        .chain(
            options
                .remote_state_mappings
                .iter()
                .flat_map(|mapping| [&mapping.consumer, &mapping.remote, &mapping.producer]),
        )
    {
        remaining = remaining
            .checked_sub(value.len())
            .ok_or(Error::ResourceLimit)?;
    }
    for mapping in &options.remote_state_mappings {
        mapping.validate()?;
    }
    Ok(())
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
                && !options.hide_relationship_legend
                && !options.omit_relationship_legend =>
        {
            svg::render(graph, layout)
        }
        (OutputFormat::Svg, None) => svg::render_with_format(graph, layout, options),
    }
}

#[cfg(test)]
#[path = "../tests/unit/generation.rs"]
mod tests;
