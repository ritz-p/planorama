use crate::{Error, Generated, InputInventory, Options, OutputFormat, semantic, svg, view};
use std::collections::{BTreeMap, BTreeSet};

#[cfg(test)]
#[path = "../tests/unit/named_inputs.rs"]
mod tests;

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NamedInput {
    pub state: String,
    pub json: String,
}

fn parse(inputs: &[NamedInput]) -> Result<crate::model::MultiPlan, Error> {
    if inputs.is_empty() || inputs.len() > 64 {
        return Err(Error::EmptyInputs);
    }
    let mut remaining = crate::generation::MAX_INPUT_BYTES;
    let mut names = BTreeSet::new();
    for input in inputs {
        crate::model::StateId::new(&input.state).map_err(|_| Error::InvalidStateName)?;
        if !names.insert(&input.state) {
            return Err(Error::DuplicateStateName);
        }
        remaining = remaining
            .checked_sub(input.json.len())
            .ok_or(Error::InputTooLarge)?;
    }
    let mut states = BTreeMap::new();
    for input in inputs {
        let id = crate::model::StateId::new(&input.state).map_err(|_| Error::InvalidStateName)?;
        states.insert(id, crate::generation::parse_input(&input.json)?);
    }
    Ok(crate::model::MultiPlan { states })
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
    if options.memory_budget_mib.is_some()
        || ((options.html_diagnostics || options.html_semantic_diagnostics)
            && !matches!(options.format, OutputFormat::Html))
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
    let plans = parse(inputs)?;
    crate::generation::progress::report("selection");
    let diagnostics = plans
        .states
        .values()
        .flat_map(semantic::diagnostics::collect)
        .map(|diagnostic| diagnostic.reason.description().to_owned())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let architecture = crate::model::cross_state::Architecture {
        states: plans
            .states
            .iter()
            .map(|(id, plan)| (id.clone(), semantic::transform(plan)))
            .collect(),
        relationships: semantic::cross_state::resolve(&plans, &[]).edges,
    };
    let architecture = view::apply(&architecture, &filter).map_err(|_| Error::SelectionFailed)?;
    crate::generation::progress::report("placement");
    let format = svg::Options {
        address_format: options.address_format,
        hide_relationship_legend: options.hide_relationship_legend,
        omit_relationship_legend: options.omit_relationship_legend,
    };
    let content = match options.format {
        OutputFormat::Svg => svg::render_states(&architecture, format),
        OutputFormat::Html if options.html_diagnostics => crate::html::render_states_report(
            &architecture,
            format,
            &mut serde_json::Value::Null,
            true,
        ),
        OutputFormat::Html => crate::html::render_states(&architecture, format),
    };
    Ok(Generated {
        content,
        diagnostics,
    })
}
