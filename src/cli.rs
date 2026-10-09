use crate::layout::Layout;
use crate::{plan, semantic, svg};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
mod files;
mod states;

const HELP: &str = "planorama — Terraform plan JSON → SVG (pure Rust)\n\nUsage: planorama <plan.json | -> [-o <diagram.svg | ->] [--address-format qualified|terraform] [--diagnostics]\n\n  -o, --output PATH   Output file (default: diagram.svg); '-' for stdout\n  --address-format FORMAT  Resource labels: qualified (default) or terraform\n  --changed-only     Retain changes and required context\n  --focus ADDRESS    Select a resource (STATE:ADDRESS for named states)\n  --focus-depth N    Neighbor depth (default: 1; requires --focus)\n  --remote-state CONSUMER:ADDRESS=PRODUCER  Map a named remote-state data source\n  --diagnostics      Explain unresolved references and semantic fallbacks on stderr\n  -h, --help          Show help\n  -V, --version       Show version\n\nInput: terraform show -json <saved-plan> (not terraform plan -json).\nLines show reference or architectural relationships; containment uses nested boxes.\nAttribute values are never included in the diagram.\n";

pub fn run() -> Result<(), String> {
    let mut input = None;
    let mut state_inputs = std::collections::BTreeMap::new();
    let mut mappings = Vec::new();
    let mut output = PathBuf::from("diagram.svg");
    let mut address_format = svg::AddressFormat::default();
    let mut diagnostics = false;
    let mut filter = semantic::filter::Options::default();
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--changed-only" => filter.changed_only = true,
            "--focus" => filter.focus = Some(args.next().ok_or("--focus requires an address")?),
            "--focus-depth" => {
                filter.depth = Some(
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
            "--diagnostics" => diagnostics = true,
            "-h" | "--help" => {
                print!(
                    "{HELP}\nMultiple plans: planorama --state ID=PATH [--state ID=PATH ...] [-o diagram.svg]\nState IDs are explicit and unique (1-64 ASCII letters, digits, '.', '_' or '-').\nOne named input may use '-' for stdin; do not mix --state with a positional input.\n"
                );
                return Ok(());
            }
            "-V" | "--version" => {
                println!("planorama {}", env!("CARGO_PKG_VERSION"));
                return Ok(());
            }
            "-o" | "--output" => output = args.next().ok_or("--output requires a path")?.into(),
            "--address-format" => {
                address_format = match args.next().as_deref() {
                    Some("qualified") => svg::AddressFormat::Qualified,
                    Some("terraform") => svg::AddressFormat::Terraform,
                    Some(value) => {
                        return Err(format!(
                            "invalid address format: {value}; expected qualified or terraform"
                        ));
                    }
                    None => return Err("--address-format requires qualified or terraform".into()),
                };
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
    filter.validate()?;
    if !state_inputs.is_empty() {
        if input.is_some() {
            return Err("do not mix --state with a positional input".into());
        }
        return states::run(
            state_inputs,
            &output,
            address_format,
            diagnostics,
            &mappings,
            &filter,
        );
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
    let raw = plan::parse(&json)?;
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
            // Debug formatting escapes control characters in untrusted names.
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
    let graph = semantic::filter::single(&semantic::transform(&raw), &filter)?;
    let layout = Layout::new(&graph);
    let image = match address_format {
        svg::AddressFormat::Qualified => svg::render(&graph, &layout),
        svg::AddressFormat::Terraform => svg::render_with_format(&graph, &layout, address_format),
    };
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
    Ok(())
}
