use crate::{Error, Generated, InputInventory, Options, OutputFormat, semantic, svg, view};
use std::collections::{BTreeMap, BTreeSet};

#[cfg(test)]
#[path = "../tests/unit/named_inputs.rs"]
mod tests;

#[cfg(test)]
#[path = "../tests/unit/named_mappings.rs"]
mod mapping_tests;

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NamedInput {
    pub state: String,
    pub json: String,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct RemoteStateMapping {
    pub consumer: String,
    pub remote: String,
    pub producer: String,
}

impl RemoteStateMapping {
    pub(crate) fn validate(&self) -> Result<crate::model::cross_state::Mapping, Error> {
        if self.remote.len() > 4096 {
            return Err(Error::ResourceLimit);
        }
        crate::model::cross_state::Mapping::new(&self.consumer, &self.remote, &self.producer)
            .map_err(|_| Error::InvalidRemoteStateMapping)
    }
}

fn parse(inputs: &[NamedInput]) -> Result<crate::model::MultiPlan, Error> {
    validate(
        inputs,
        crate::generation::MAX_INPUT_BYTES,
        Error::InputTooLarge,
    )?;
    let mut states = BTreeMap::new();
    for input in inputs {
        let id = crate::model::StateId::new(&input.state).map_err(|_| Error::InvalidStateName)?;
        states.insert(id, crate::generation::parse_input(&input.json)?);
    }
    Ok(crate::model::MultiPlan { states })
}

pub(crate) fn validate(
    inputs: &[NamedInput],
    maximum: usize,
    exceeded: Error,
) -> Result<(), Error> {
    if inputs.is_empty() || inputs.len() > 64 {
        return Err(Error::EmptyInputs);
    }
    let mut remaining = maximum;
    let mut names = BTreeSet::new();
    for input in inputs {
        crate::model::StateId::new(&input.state).map_err(|_| Error::InvalidStateName)?;
        if !names.insert(&input.state) {
            return Err(Error::DuplicateStateName);
        }
        remaining = remaining.checked_sub(input.json.len()).ok_or(exceeded)?;
    }
    Ok(())
}

pub fn inspect_named_inputs(inputs: &[NamedInput]) -> Result<InputInventory, Error> {
    let plans = parse(inputs)?;
    let mut result = InputInventory {
        modules: Vec::new(),
        resources: Vec::new(),
    };
    for (state, plan) in &plans.states {
        let state = state.as_str();
        for mut row in
            crate::module_inventory::inventory(plan).map_err(|_| Error::UnsupportedInput)?
        {
            row.state = Some(state.to_owned());
            row.selector = row.selector.map(|selector| format!("{state}:{selector}"));
            result.modules.push(row);
        }
        for mut row in crate::resource_inventory::inventory(plan) {
            row.state = Some(state.to_owned());
            row.selector = format!("{state}:{}", row.address);
            result.resources.push(row);
        }
    }
    Ok(result)
}

pub fn generate_named(inputs: &[NamedInput], options: &Options) -> Result<Generated, Error> {
    if options.memory_budget_mib.is_some() {
        return Err(Error::InvalidOptions);
    }
    crate::work::complete(generate_step(inputs, options))
}

pub(crate) async fn generate_step(
    inputs: &[NamedInput],
    options: &Options,
) -> Result<Generated, Error> {
    crate::generation::validate_option_sizes(options)?;
    if (options.html_diagnostics || options.html_semantic_diagnostics)
        && !matches!(options.format, OutputFormat::Html)
    {
        return Err(Error::InvalidOptions);
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
    crate::generation::progress::report("parsing");
    validate(
        inputs,
        crate::generation::MAX_INPUT_BYTES,
        Error::InputTooLarge,
    )?;
    let remaining = std::cell::Cell::new(32_768);
    let mut states = BTreeMap::new();
    let (mut nodes, mut edges) = (0, 0);
    for input in inputs {
        crate::work::checkpoint().await;
        if crate::work::limited() {
            crate::generation::budget::validate_remaining(&input.json, &remaining)?;
        }
        let plan = crate::generation::parse_input(&input.json)?;
        nodes += plan.nodes.len();
        edges += plan.edges.len();
        crate::work::check_limit(nodes, 256).await;
        crate::work::check_limit(edges, 1024).await;
        states.insert(
            crate::model::StateId::new(&input.state).map_err(|_| Error::InvalidStateName)?,
            plan,
        );
    }
    let plans = crate::model::MultiPlan { states };
    crate::work::checkpoint().await;
    crate::generation::progress::report("selection");
    let mappings = options
        .remote_state_mappings
        .iter()
        .map(RemoteStateMapping::validate)
        .collect::<Result<Vec<_>, _>>()?;
    semantic::cross_state::validate_mappings(&plans, &mappings).map_err(|reason| match reason {
        crate::model::cross_state::Failure::MissingState => Error::UnknownRemoteStateMapping,
        crate::model::cross_state::Failure::Ambiguous => Error::AmbiguousRemoteStateMapping,
        _ => Error::InvalidRemoteStateMapping,
    })?;
    let cross = semantic::cross_state::resolve_step(&plans, &mappings).await;
    crate::work::check_limit(edges + cross.edges.len(), 1024).await;
    let diagnostics = plans
        .states
        .values()
        .flat_map(semantic::diagnostics::collect)
        .map(|diagnostic| diagnostic.reason.description().to_owned())
        .chain(
            cross
                .diagnostics
                .iter()
                .filter(|_| !mappings.is_empty())
                .map(|diagnostic| diagnostic.reason.description().to_owned()),
        )
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let architecture = crate::model::cross_state::Architecture {
        states: plans
            .states
            .iter()
            .map(|(id, plan)| (id.clone(), semantic::transform(plan)))
            .collect(),
        relationships: cross.edges,
    };
    let architecture = view::apply(&architecture, &filter).map_err(|_| Error::SelectionFailed)?;
    drop(plans);
    crate::work::check_limit(
        architecture
            .states
            .values()
            .map(|graph| graph.nodes.len())
            .sum(),
        256,
    )
    .await;
    crate::work::check_limit(
        architecture.relationships().len()
            + architecture
                .states
                .values()
                .map(|graph| graph.edges.len())
                .sum::<usize>(),
        1024,
    )
    .await;
    crate::work::checkpoint().await;
    crate::generation::progress::report("placement");
    let format = svg::Options {
        address_format: options.address_format,
        hide_relationship_legend: options.hide_relationship_legend,
        omit_relationship_legend: options.omit_relationship_legend,
    };
    let content = match options.format {
        OutputFormat::Svg => {
            svg::render_states_step(&architecture, format, svg::Content::Standard, None).await
        }
        OutputFormat::Html if options.html_diagnostics => {
            crate::html::render_states_report_step(
                &architecture,
                format,
                &mut serde_json::Value::Null,
                true,
            )
            .await
        }
        OutputFormat::Html => crate::html::render_states_step(&architecture, format).await,
    };
    let content = crate::html::generation::attach(
        content, &filter, format, options.html_diagnostics,
        architecture.states.keys().map(|id| id.as_str()).collect(),
        options.remote_state_mappings.iter().map(|mapping| serde_json::json!({
            "consumer": mapping.consumer, "remote": mapping.remote, "producer": mapping.producer,
        })).collect(),
    );
    crate::work::check_limit(content.len(), crate::generation::MAX_BROWSER_OUTPUT_BYTES).await;
    Ok(Generated {
        content,
        diagnostics,
    })
}
