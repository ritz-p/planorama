use crate::model::{PlanInput, StateId};
use crate::{plan, semantic, svg};
use std::collections::BTreeMap;
use std::io::{self, Read, Write};
use std::path::Path;

pub(super) fn run(
    inputs: (BTreeMap<StateId, String>, &super::provider_plan::Options),
    output: &Path,
    format: svg::AddressFormat,
    output_format: super::OutputFormat,
    diagnostics: (bool, &super::layout_diagnostics::Options),
    mappings: &[crate::model::cross_state::Mapping],
    filter: &crate::view::Options,
) -> Result<(), String> {
    let (inputs, provider_plan) = inputs;
    let (diagnostics, layout_diagnostics) = diagnostics;
    if inputs.values().filter(|path| path.as_str() == "-").count() > 1 {
        return Err("stdin may be used by only one state".into());
    }
    for (id, path) in &inputs {
        protect_input(path, output).map_err(|e| format!("state {:?}: {e}", id.as_str()))?;
    }
    let inputs = inputs
        .into_iter()
        .map(|(state_id, path)| {
            let _timing = crate::timing::span("input_io", &[]);
            let mut plan_json = String::new();
            if path == "-" {
                io::stdin().read_to_string(&mut plan_json).map_err(|e| {
                    format!("state {:?}: cannot read stdin: {e}", state_id.as_str())
                })?;
            } else {
                plan_json = std::fs::read_to_string(&path).map_err(|e| {
                    format!("state {:?}: cannot read {path}: {e}", state_id.as_str())
                })?;
            }
            Ok(PlanInput {
                state_id,
                plan_json,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let evidence = inputs
        .iter()
        .map(|input| {
            provider_plan
                .load(Some(&input.state_id), &input.plan_json)
                .map(|evidence| (input.state_id.clone(), evidence))
                .map_err(|error| format!("state {:?}: {error}", input.state_id.as_str()))
        })
        .collect::<Result<BTreeMap<_, _>, _>>()?;
    let mut plans = plan::parse_inputs(inputs)?;
    for (id, raw) in &mut plans.states {
        if let Some(Some(evidence)) = evidence.get(id) {
            evidence.apply(raw);
        }
    }
    let cross = semantic::cross_state::resolve(&plans, mappings);
    if diagnostics {
        let mut stderr = io::stderr().lock();
        for diagnostic in &cross.diagnostics {
            writeln!(
                stderr,
                "state={:?}: {}: address={:?}, remote={:?}, output={:?}",
                diagnostic.consumer.state.as_str(),
                diagnostic.reason.description(),
                diagnostic.consumer.address,
                diagnostic.remote,
                diagnostic.output
            )
            .map_err(|e| format!("cannot write diagnostics: {e}"))?;
        }
        for edge in &cross.edges {
            let crate::model::cross_state::CrossStateProvenance::TerraformRemoteState {
                remote,
                output,
                ..
            } = &edge.provenance;
            writeln!(
                stderr,
                "cross-state: {:?}:{:?} -> {:?}:{:?} via {:?}.outputs.{:?}",
                edge.from.state.as_str(),
                edge.from.address,
                edge.to.state.as_str(),
                edge.to.address,
                remote,
                output
            )
            .map_err(|e| format!("cannot write diagnostics: {e}"))?;
        }
    }
    let mut graphs = BTreeMap::new();
    for (id, raw) in &plans.states {
        if diagnostics {
            let mut stderr = io::stderr().lock();
            for relevant in &raw.relevant_attributes {
                writeln!(
                    stderr,
                    "state={:?}: relevance: resource={:?}, matched={}, path={:?}",
                    id.as_str(),
                    relevant.resource,
                    relevant.matched,
                    relevant.path
                )
                .map_err(|e| format!("cannot write diagnostics: {e}"))?;
            }
            for diagnostic in semantic::diagnostics::collect(raw) {
                writeln!(
                    stderr,
                    "state={:?}: warning: {}: address={:?}, attribute={:?}",
                    id.as_str(),
                    diagnostic.reason.description(),
                    diagnostic.address,
                    diagnostic.attribute
                )
                .map_err(|e| format!("cannot write diagnostics: {e}"))?;
            }
        }
        graphs.insert(id.clone(), semantic::transform(raw));
    }
    let architecture = crate::model::cross_state::Architecture {
        states: graphs,
        relationships: cross.edges,
    };
    let architecture = crate::view::apply(&architecture, filter)?;
    let mut report = serde_json::Value::Null;
    let composition_timing = crate::timing::span(
        "composition_serialization",
        &[("states", architecture.states.len())],
    );
    let image = match output_format {
        super::OutputFormat::Svg if layout_diagnostics.enabled() => svg::render_states_report(
            &architecture,
            format,
            svg::Content::Standard,
            Some(&mut report),
        ),
        super::OutputFormat::Html if layout_diagnostics.enabled() => {
            crate::html::render_states_report(
                &architecture,
                format,
                &mut report,
                layout_diagnostics.html,
            )
        }
        super::OutputFormat::Svg => svg::render_states(&architecture, format),
        super::OutputFormat::Html => crate::html::render_states(&architecture, format),
    };
    drop(composition_timing);
    let _timing = crate::timing::span("output_io", &[("output_bytes", image.len())]);
    layout_diagnostics.write(&report, output)?;
    if output == Path::new("-") {
        io::stdout()
            .write_all(image.as_bytes())
            .map_err(|e| format!("cannot write stdout: {e}"))?;
    } else {
        std::fs::write(output, image)
            .map_err(|e| format!("cannot write {}: {e}", output.display()))?;
        eprintln!(
            "Wrote {} ({} states)",
            output.display(),
            architecture.states.len()
        );
    }
    Ok(())
}

pub(super) fn protect_input(input: &str, output: &Path) -> Result<(), String> {
    if input == "-" {
        return Ok(());
    }
    let resolved_input =
        std::fs::canonicalize(input).map_err(|e| format!("cannot read {input}: {e}"))?;
    if output == Path::new("-") {
        return Ok(());
    }
    super::files::reject_same_file(Path::new(input), output)?;
    let absolute = if output.is_absolute() {
        output.to_owned()
    } else {
        std::env::current_dir()
            .map_err(|e| e.to_string())?
            .join(output)
    };
    let resolved_output = absolute
        .parent()
        .and_then(|p| p.canonicalize().ok())
        .zip(absolute.file_name())
        .map(|(p, name)| p.join(name));
    if output.canonicalize().ok().as_ref() == Some(&resolved_input)
        || resolved_output.as_ref() == Some(&resolved_input)
    {
        return Err("input and output must be different files".into());
    }
    Ok(())
}
