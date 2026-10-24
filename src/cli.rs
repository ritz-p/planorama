use crate::layout::Layout;
use crate::{plan, semantic, svg};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
mod coverage_report;
mod files;
mod layout_diagnostics;
mod modules;
mod provider_plan;
mod settings;
mod states;

pub(super) use crate::generation::OutputFormat;

const HELP: &str = "planorama — Terraform plan JSON → SVG / HTML (pure Rust)\n\nUsage: planorama <plan.json | -> [--format svg|html] [-o <path | ->] [--address-format qualified|terraform] [--diagnostics]\n\n  -o, --output PATH   Output file (default: diagram.svg or diagram.html); '-' for stdout\n  --config PATH      Load explicit TOML generation settings\n  --preset NAME      Select a named preset (requires --config)\n  --no-<BOOLEAN>     Disable a boolean generation option\n  --spatial-type TYPE  Display an allowed AWS type spatially (repeatable)\n  --no-spatial-types  Clear configured Spatial types\n  --no-modules       Clear configured module selection\n  --no-focus         Clear configured focus and inherited depth\n  --format FORMAT    Output format: svg (default) or self-contained html\n  --hide-relationship-legend  Omit relationship explanations; keep action colors\n  --omit-relationship-legend  Omit the entire relationship legend, including action colors\n  --icon-mode MODE   Service display: {icon_modes}\n  --address-format FORMAT  Resource labels: qualified (default) or terraform\n  --provider-plan PATH  Verify accounts against a saved Terraform 1.9.8 or 1.16.6 / AWS 5.100.0 plan\n  --state-provider-plan ID=PATH  Saved plan evidence for one named state\n  --list-modules     List original module instances/counts; stdout by default; no diagram\n  --modules ADDRESS...  Select module subtrees (repeatable; STATE:ADDRESS for named states)\n  --changed-only     Retain changes and required context\n  --focus ADDRESS    Select a resource (STATE:ADDRESS for named states)\n  --focus-depth N    Neighbor depth (default: 1; requires --focus)\n  --remote-state CONSUMER:ADDRESS=PRODUCER  Map a named remote-state data source\n  --timings          Report generation stage timings on stderr\n  --diagnostics      Explain unresolved references and semantic fallbacks on stderr\n  --html-diagnostics  Include bounded routing hotspots (requires --format html)\n  --html-semantic-diagnostics  Include bounded semantic findings (requires --format html)\n  --coverage-report PATH  Write a separate Markdown coverage report\n  --coverage-selection-details N  Include up to N selected identities/reasons (0-1000; requires report)\n  --layout-diagnostics  Report final layout quality on stderr\n  --layout-diagnostics-json PATH  Write a versioned layout report to a separate JSON file\n  -h, --help          Show help\n  -V, --version       Show version\n\nInput: terraform show -json <saved-plan> (not terraform plan -json).\nJSON-only inputs leave AWS accounts unknown. Provider evidence requires Terraform and initialized schemas.\nLines show reference or architectural relationships; containment uses nested boxes.\nArbitrary evaluated attribute values are not displayed; allowlisted Organizations policy-type labels are shown.\nStandalone SVG may show validated, non-sensitive Route destination constants\nfrom configuration (CIDRs / prefix-list IDs); HTML omits these constants.\n";

pub(crate) fn run() -> Result<(), String> {
    let mut input = None;
    let mut list_modules = false;
    let mut specified = Vec::new();
    let mut state_inputs = std::collections::BTreeMap::new();
    let mut mappings = Vec::new();
    let mut provider_plan = provider_plan::Options::default();
    let mut output = None;
    let mut output_format = OutputFormat::default();
    let mut render_options = svg::Options::default();
    let mut diagnostics = false;
    let mut timings = false;
    let mut layout_diagnostics = layout_diagnostics::Options::default();
    let mut coverage_report = coverage_report::Options::default();
    let mut filter = crate::view::Options::default();
    let mut config = None;
    let mut preset = None;
    let mut settings = settings::Generation::default();
    let mut args = std::env::args().skip(1).peekable();
    while let Some(arg) = args.next() {
        if arg != "-" && arg.starts_with('-') {
            specified.push(arg.clone());
        }
        match arg.as_str() {
            "--config" => {
                if config
                    .replace(args.next().ok_or("--config requires PATH")?)
                    .is_some()
                {
                    return Err("duplicate --config".into());
                }
            }
            "--preset" => {
                if preset
                    .replace(args.next().ok_or("--preset requires NAME")?)
                    .is_some()
                {
                    return Err("duplicate --preset".into());
                }
            }
            "--no-spatial-types" => settings.spatial_types = Some(Vec::new()),
            "--spatial-type" => settings.spatial_types.get_or_insert_with(Vec::new).push(
                args.next()
                    .ok_or("--spatial-type requires an AWS resource type")?,
            ),
            "--no-modules" => settings.modules = Some(Vec::new()),
            "--no-focus" => settings.focus = Some(Vec::new()),
            "--list-modules" => list_modules = true,
            "--icon-mode" => {
                settings.icon_mode = Some(match args.next().as_deref() {
                    Some("icon") => crate::icons::IconMode::Icon,
                    Some("service-name") => crate::icons::IconMode::ServiceName,
                    Some(value) => {
                        return Err(format!(
                            "invalid icon mode: {value}; expected icon or service-name"
                        ));
                    }
                    None => return Err("--icon-mode requires icon or service-name".into()),
                });
            }
            "--provider-plan" => {
                provider_plan.single(args.next().ok_or("--provider-plan requires PATH")?)?
            }
            "--state-provider-plan" => provider_plan.state(
                &args
                    .next()
                    .ok_or("--state-provider-plan requires ID=PATH")?,
            )?,
            "--format" => {
                settings.format = Some(match args.next().as_deref() {
                    Some("svg") => OutputFormat::Svg,
                    Some("html") => OutputFormat::Html,
                    Some(value) => {
                        return Err(format!(
                            "invalid output format: {value}; expected svg or html"
                        ));
                    }
                    None => return Err("--format requires svg or html".into()),
                });
            }
            "--hide-relationship-legend" => settings.hide_relationship_legend = Some(true),
            "--no-hide-relationship-legend" => settings.hide_relationship_legend = Some(false),
            "--omit-relationship-legend" => settings.omit_relationship_legend = Some(true),
            "--no-omit-relationship-legend" => settings.omit_relationship_legend = Some(false),
            "--changed-only" => settings.changed_only = Some(true),
            "--no-changed-only" => settings.changed_only = Some(false),
            "--modules" => {
                let modules = settings.modules.get_or_insert_with(Vec::new);
                let start = modules.len();
                while args.peek().is_some_and(|value| !value.starts_with('-')) {
                    modules.push(args.next().unwrap());
                }
                if modules.len() == start {
                    return Err("--modules requires one or more module addresses".into());
                }
            }
            "--focus" => {
                settings.focus = Some(vec![args.next().ok_or("--focus requires an address")?])
            }
            "--focus-depth" => {
                settings.focus_depth = Some(
                    args.next()
                        .ok_or("--focus-depth requires a nonnegative integer")?
                        .parse()
                        .map_err(|_| "--focus-depth requires a nonnegative integer")?,
                )
            }
            "--remote-state" => mappings.push(crate::model::cross_state::Mapping::parse(
                &args
                    .next()
                    .ok_or("--remote-state requires CONSUMER:ADDRESS=PRODUCER")?,
            )?),
            "--state" => {
                let named = args.next().ok_or("--state requires ID=PATH")?;
                let (id, path) = named.split_once('=').ok_or("--state requires ID=PATH")?;
                let id = crate::model::StateId::new(id)?;
                if path.is_empty() {
                    return Err("--state requires a nonempty input path".into());
                }
                if state_inputs.contains_key(&id) {
                    return Err(format!("duplicate state ID: {}", id.as_str()));
                }
                state_inputs.insert(id, path.to_owned());
            }
            "--diagnostics" => settings.diagnostics = Some(true),
            "--no-diagnostics" => settings.diagnostics = Some(false),
            "--timings" => settings.timings = Some(true),
            "--no-timings" => settings.timings = Some(false),
            "--html-diagnostics" => settings.html_diagnostics = Some(true),
            "--no-html-diagnostics" => settings.html_diagnostics = Some(false),
            "--html-semantic-diagnostics" => settings.html_semantic_diagnostics = Some(true),
            "--no-html-semantic-diagnostics" => settings.html_semantic_diagnostics = Some(false),
            "--layout-diagnostics" => settings.layout_diagnostics = Some(true),
            "--no-layout-diagnostics" => settings.layout_diagnostics = Some(false),
            "--coverage-report" => {
                if coverage_report
                    .path
                    .replace(args.next().ok_or("--coverage-report requires PATH")?.into())
                    .is_some()
                {
                    return Err("duplicate --coverage-report".into());
                }
            }
            "--coverage-selection-details" => {
                filter.selection_details = args
                    .next()
                    .ok_or("--coverage-selection-details requires a limit (0-1000)")?
                    .parse()
                    .map_err(|_| "--coverage-selection-details requires a limit (0-1000)")?;
                if filter.selection_details > 1000 {
                    return Err("--coverage-selection-details limit must be 0-1000".into());
                }
            }
            "--layout-diagnostics-json" => {
                layout_diagnostics.json = Some(
                    args.next()
                        .ok_or("--layout-diagnostics-json requires a path")?
                        .into(),
                )
            }
            "-h" | "--help" => {
                let modes = "service-name (default; icon unavailable in this build)";
                let help = HELP.replace("{icon_modes}", modes);
                print!(
                    "{help}\nMultiple plans: planorama --state ID=PATH [--state ID=PATH ...] [-o diagram.svg]\nState IDs are explicit and unique (1-64 ASCII letters, digits, '.', '_' or '-').\nOne named input may use '-' for stdin; do not mix --state with a positional input.\n"
                );
                return Ok(());
            }
            "-V" | "--version" => {
                println!("planorama {}", env!("CARGO_PKG_VERSION"));
                return Ok(());
            }
            "-o" | "--output" => {
                output = Some(PathBuf::from(
                    args.next().ok_or("--output requires a path")?,
                ))
            }
            "--address-format" => {
                settings.address_format = Some(match args.next().as_deref() {
                    Some("qualified") => svg::AddressFormat::Qualified,
                    Some("terraform") => svg::AddressFormat::Terraform,
                    Some(value) => {
                        return Err(format!(
                            "invalid address format: {value}; expected qualified or terraform"
                        ));
                    }
                    None => return Err("--address-format requires qualified or terraform".into()),
                });
            }
            _ if arg != "-" && arg.starts_with('-') => {
                return Err(format!("unknown option: {arg}"));
            }
            _ => match input.replace(arg) {
                Some(_) => return Err("provide only one input file".into()),
                None => continue,
            },
        }
    }
    let settings = settings::load(config.as_deref(), preset.as_deref())?.overlay(settings);
    if let Some(value) = settings.spatial_types {
        filter.spatial_types = value;
    }
    if let Some(value) = settings.format {
        output_format = value;
    }
    if let Some(value) = settings.icon_mode {
        filter.icon_mode = value;
    }
    if let Some(value) = settings.address_format {
        render_options.address_format = value;
    }
    if let Some(value) = settings.hide_relationship_legend {
        render_options.hide_relationship_legend = value;
    }
    if let Some(value) = settings.omit_relationship_legend {
        render_options.omit_relationship_legend = value;
    }
    if let Some(value) = settings.changed_only {
        filter.changed_only = value;
    }
    if let Some(value) = settings.modules {
        filter.modules = value;
    }
    if let Some(value) = settings.focus {
        filter.focus = value.into_iter().next();
    }
    if let Some(value) = settings.focus_depth {
        filter.depth = Some(value);
    }
    if let Some(value) = settings.diagnostics {
        diagnostics = value;
    }
    if let Some(value) = settings.timings {
        timings = value;
    }
    if let Some(value) = settings.html_diagnostics {
        layout_diagnostics.html = value;
    }
    if let Some(value) = settings.html_semantic_diagnostics {
        filter.html_semantic_diagnostics = value;
    }
    if let Some(value) = settings.layout_diagnostics {
        layout_diagnostics.text = value;
    }
    let timing = crate::timing::Session::start(timings);
    filter.explain_selection = coverage_report.path.is_some();
    if specified
        .iter()
        .any(|arg| arg == "--coverage-selection-details")
        && !filter.explain_selection
    {
        return Err("--coverage-selection-details requires --coverage-report".into());
    }
    if list_modules {
        for option in &specified {
            if !matches!(
                option.as_str(),
                "--list-modules" | "--state" | "-o" | "--output" | "--timings" | "--no-timings"
            ) {
                return Err(format!(
                    "--list-modules cannot be combined with {option}; it lists the original input"
                ));
            }
        }
        modules::run(input, state_inputs, output.as_deref())?;
        return timing.finish();
    }
    let output = output.unwrap_or_else(|| {
        PathBuf::from(match output_format {
            OutputFormat::Svg => "diagram.svg",
            OutputFormat::Html => "diagram.html",
        })
    });
    if let Some(config) = &config {
        for destination in std::iter::once(output.as_path())
            .chain(layout_diagnostics.json.as_deref())
            .chain(coverage_report.path.as_deref())
            .filter(|path| *path != Path::new("-"))
        {
            files::reject_same_file(Path::new(config), destination)?;
        }
    }
    if layout_diagnostics.html && !matches!(output_format, OutputFormat::Html) {
        return Err("--html-diagnostics requires --format html".into());
    }
    filter.validate()?;
    if filter.html_semantic_diagnostics && !matches!(output_format, OutputFormat::Html) {
        return Err("--html-semantic-diagnostics requires --format html".into());
    }
    provider_plan.validate(&state_inputs, &output, layout_diagnostics.json.as_deref())?;
    layout_diagnostics.validate(input.iter().chain(state_inputs.values()), &output)?;
    coverage_report.validate(
        input.iter().chain(state_inputs.values()),
        &output,
        layout_diagnostics.json.as_deref(),
    )?;
    if let Some(path) = &coverage_report.path {
        provider_plan.validate(&state_inputs, &output, Some(path))?;
    }
    if !state_inputs.is_empty() {
        if input.is_some() {
            return Err("do not mix --state with a positional input".into());
        }
        states::run(
            (state_inputs, &provider_plan),
            &output,
            render_options,
            output_format,
            (diagnostics, &layout_diagnostics, &coverage_report),
            &mappings,
            &filter,
        )?;
        return timing.finish();
    }
    if !mappings.is_empty() {
        return Err("--remote-state requires named --state inputs".into());
    }
    let input = input.ok_or("missing input; use --help for usage")?;
    if input != "-" && output != Path::new("-") {
        files::reject_same_file(Path::new(&input), &output)?;
        let absolute = match &output {
            path if path.is_absolute() => path.clone(),
            path => std::env::current_dir()
                .map_err(|e| e.to_string())?
                .join(path),
        };
        let resolved_output = absolute
            .parent()
            .and_then(|p| p.canonicalize().ok())
            .zip(absolute.file_name())
            .map(|(p, name)| p.join(name));
        let resolved_input =
            std::fs::canonicalize(&input).map_err(|e| format!("cannot read {input}: {e}"))?;
        if output.canonicalize().ok().as_ref() == Some(&resolved_input)
            || resolved_output.as_ref() == Some(&resolved_input)
        {
            return Err("input and output must be different files".into());
        }
    }
    let input_timing = crate::timing::span("input_io", &[]);
    let json = match input.as_str() {
        "-" => {
            let mut json = String::new();
            io::stdin()
                .read_to_string(&mut json)
                .map_err(|e| format!("cannot read stdin: {e}"))?;
            json
        }
        path => std::fs::read_to_string(path).map_err(|e| format!("cannot read {path}: {e}"))?,
    };
    drop(input_timing);
    let mut raw = plan::parse(&json)?;
    if let Some(evidence) = provider_plan.load(None, &json)? {
        evidence.apply(&mut raw);
    }
    if diagnostics {
        let mut stderr = io::stderr().lock();
        for relevant in &raw.relevant_attributes {
            writeln!(
                stderr,
                "relevance: resource={:?}, matched={}, path={:?}",
                relevant.resource, relevant.matched, relevant.path
            )
            .map_err(|e| format!("cannot write diagnostics: {e}"))?;
        }
        for diagnostic in semantic::diagnostics::collect(&raw) {
            writeln!(
                stderr,
                "warning: {}: address={:?}, attribute={:?}",
                diagnostic.reason.description(),
                diagnostic.address,
                diagnostic.attribute
            )
            .map_err(|e| format!("cannot write diagnostics: {e}"))?;
        }
    }
    let graph = crate::generation::project(&raw, &filter)?;
    let layout = Layout::new(&graph);
    let mut report = serde_json::Value::Null;
    let image = crate::generation::render(
        &graph,
        &layout,
        output_format,
        render_options,
        layout_diagnostics.enabled().then_some(&mut report),
        layout_diagnostics.html,
    );
    let output_timing = crate::timing::span("output_io", &[("output_bytes", image.len())]);
    layout_diagnostics.write(&report, &output)?;
    coverage_report.write(std::iter::once(("", &graph.coverage)), &filter)?;
    match output.as_path() {
        path if path == Path::new("-") => io::stdout()
            .write_all(image.as_bytes())
            .map_err(|e| format!("cannot write stdout: {e}"))?,
        _ => {
            std::fs::write(&output, image)
                .map_err(|e| format!("cannot write {}: {e}", output.display()))?;
            let (nodes, edges) = match graph
                .edges
                .iter()
                .any(|edge| edge.kind != crate::model::EdgeKind::Dependency)
            {
                true => ("cards", "relationships"),
                false => ("resources", "reference edges"),
            };
            eprintln!(
                "Wrote {} ({} {nodes}, {} {edges})",
                output.display(),
                graph.nodes.len(),
                graph.edges.len()
            );
        }
    }
    drop(output_timing);
    timing.finish()
}
