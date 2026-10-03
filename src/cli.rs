use crate::layout::Layout;
use crate::{plan, semantic, svg};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

const HELP: &str = "planorama — Terraform plan JSON → SVG (pure Rust)\n\nUsage: planorama <plan.json | -> [-o <diagram.svg | ->]\n\n  -o, --output PATH   Output file (default: diagram.svg); '-' for stdout\n  -h, --help          Show help\n  -V, --version       Show version\n\nInput: terraform show -json <saved-plan> (not terraform plan -json).\nArrows point from a dependency to its dependent resource.\nAttribute values are never included in the diagram.\n";

pub fn run() -> Result<(), String> {
    let mut input = None;
    let mut output = PathBuf::from("diagram.svg");
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" => {
                print!("{HELP}");
                return Ok(());
            }
            "-V" | "--version" => {
                println!("planorama {}", env!("CARGO_PKG_VERSION"));
                return Ok(());
            }
            "-o" | "--output" => output = args.next().ok_or("--output requires a path")?.into(),
            _ if arg != "-" && arg.starts_with('-') => {
                return Err(format!("unknown option: {arg}"));
            }
            _ => match input.replace(arg) {
                Some(_) => return Err("provide only one input file".into()),
                None => continue,
            },
        }
    }
    let input = input.ok_or("missing input; use --help for usage")?;
    if input != "-" && output != Path::new("-") {
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
    let graph = semantic::transform(&raw);
    let layout = Layout::new(&graph);
    let image = svg::render(&graph, &layout);
    match output.as_path() {
        path if path == Path::new("-") => io::stdout()
            .write_all(image.as_bytes())
            .map_err(|e| format!("cannot write stdout: {e}"))?,
        _ => {
            std::fs::write(&output, image)
                .map_err(|e| format!("cannot write {}: {e}", output.display()))?;
            eprintln!(
                "Wrote {} ({} resources, {} reference edges)",
                output.display(),
                graph.nodes.len(),
                graph.edges.len()
            );
        }
    }
    Ok(())
}
