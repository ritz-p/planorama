use crate::model::{StateId, TerraformPlan};
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::io::{Read, Write};
use std::path::Path;

pub(super) fn run(
    input: Option<String>,
    states: BTreeMap<StateId, String>,
    output: Option<&Path>,
) -> Result<(), String> {
    let inputs: Vec<_> = if states.is_empty() {
        vec![(
            String::new(),
            input.ok_or("missing input; use --help for usage")?,
        )]
    } else {
        if input.is_some() {
            return Err("do not mix --state with a positional input".into());
        }
        states
            .into_iter()
            .map(|(id, path)| (id.as_str().to_owned(), path))
            .collect()
    };
    if inputs.iter().filter(|(_, path)| path == "-").count() > 1 {
        return Err("stdin may be used by only one state".into());
    }
    let output = output.unwrap_or_else(|| Path::new("-"));
    for (_, input) in &inputs {
        super::states::protect_input(input, output)?;
    }
    let mut text = String::from(
        "Terraform modules (original input; no selection filters)\nCounts: distinct managed/data resource instances; current and each deposed object count separately.\nConfiguration rows aggregate backed instances; they do not prove any indexed instance exists.\nRoot rows have no --modules selector. Selectors require shell-specific quoting.\n\nSelector\tEvidence\tDirect\tSubtree\n",
    );
    for (state, input) in inputs {
        let mut json = String::new();
        if input == "-" {
            std::io::stdin()
                .read_to_string(&mut json)
                .map_err(|e| format!("cannot read stdin: {e}"))?;
        } else {
            json =
                std::fs::read_to_string(&input).map_err(|e| format!("cannot read {input}: {e}"))?;
        }
        let plan =
            crate::plan::parse(&json).map_err(|error| format!("state {state:?}: {error}"))?;
        append(&mut text, &state, &plan)?;
    }
    if output == Path::new("-") {
        std::io::stdout()
            .write_all(text.as_bytes())
            .map_err(|e| format!("cannot write stdout: {e}"))?;
    } else {
        std::fs::write(output, text)
            .map_err(|e| format!("cannot write {}: {e}", output.display()))?;
    }
    Ok(())
}

fn append(text: &mut String, state: &str, plan: &TerraformPlan) -> Result<(), String> {
    for entry in crate::module_inventory::inventory(plan)? {
        let selector = entry
            .selector
            .unwrap_or_else(|| "(root; not selectable)".into());
        let selector = if state.is_empty() {
            selector
        } else {
            format!("{state}:{selector}")
        };
        writeln!(
            text,
            "{selector}\t{}\t{}\t{}",
            if entry.instance {
                "instance"
            } else {
                "configuration (aggregate)"
            },
            entry.direct,
            entry.subtree
        )
        .unwrap();
    }
    Ok(())
}
