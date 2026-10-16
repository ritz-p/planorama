use std::path::{Path, PathBuf};

pub(super) fn reject_same_file(input: &Path, output: &Path) -> Result<(), String> {
    let input_handle = same_file::Handle::from_path(input)
        .map_err(|e| format!("cannot read {}: {e}", input.display()))?;
    let output_handle = match same_file::Handle::from_path(output) {
        Ok(handle) => handle,
        // A new output has no identity to compare yet.
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(format!(
                "cannot inspect output {}: {error}",
                output.display()
            ));
        }
    };
    if input_handle == output_handle {
        return Err("input and output must be different files".into());
    }
    Ok(())
}

pub(super) fn validate_report(
    path: &Path,
    inputs: impl Iterator<Item = impl AsRef<str>>,
    output: &Path,
    option: &str,
) -> Result<(), String> {
    if path == Path::new("-") {
        return Err(format!("{option} requires a separate file, not '-'"));
    }
    for destination in [path, output] {
        if destination == Path::new("-") {
            continue;
        }
        match std::fs::symlink_metadata(destination) {
            Ok(metadata) if metadata.is_symlink() => {
                destination.canonicalize().map_err(|e| {
                    format!(
                        "cannot resolve output symlink {}: {e}",
                        destination.display()
                    )
                })?;
            }
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => {
                return Err(format!(
                    "cannot inspect output {}: {e}",
                    destination.display()
                ));
            }
        }
    }
    for input in inputs {
        super::states::protect_input(input.as_ref(), path)?;
    }
    if output != Path::new("-") {
        if output.exists() {
            super::files::reject_same_file(output, path)?;
        }
        let resolve = |p: &Path| -> Result<PathBuf, String> {
            let p = if p.is_absolute() {
                p.to_owned()
            } else {
                std::env::current_dir().map_err(|e| e.to_string())?.join(p)
            };
            Ok(p.canonicalize().unwrap_or_else(|_| {
                p.parent()
                    .and_then(|v| v.canonicalize().ok())
                    .zip(p.file_name())
                    .map(|(parent, name)| parent.join(name))
                    .unwrap_or(p)
            }))
        };
        if resolve(output)? == resolve(path)? {
            return Err("output destinations must be different files".into());
        }
    }
    Ok(())
}
