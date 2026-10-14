use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Default)]
pub(super) struct Options {
    pub text: bool,
    pub json: Option<PathBuf>,
}

impl Options {
    pub fn enabled(&self) -> bool {
        self.text || self.json.is_some()
    }

    pub fn validate(
        &self,
        inputs: impl Iterator<Item = impl AsRef<str>>,
        output: &Path,
    ) -> Result<(), String> {
        let Some(path) = &self.json else {
            return Ok(());
        };
        if path == Path::new("-") {
            return Err("--layout-diagnostics-json requires a separate file, not '-'".into());
        }
        for destination in [path.as_path(), output] {
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
                return Err("diagram and layout diagnostics must be different files".into());
            }
        }
        Ok(())
    }

    pub fn write(&self, report: &serde_json::Value) -> Result<(), String> {
        if let Some(path) = &self.json {
            let mut json = serde_json::to_string_pretty(report).map_err(|e| e.to_string())?;
            json.push('\n');
            std::fs::write(path, json)
                .map_err(|e| format!("cannot write layout diagnostics {}: {e}", path.display()))?;
        }
        if self.text {
            let mut stderr = std::io::stderr().lock();
            writeln!(stderr, "{}", text(report))
                .map_err(|e| format!("cannot write layout diagnostics: {e}"))?;
        }
        Ok(())
    }
}

fn text(report: &serde_json::Value) -> String {
    use std::fmt::Write;
    let mut text = format!(
        "layout diagnostics (schema 1, whole diagram, px)\nCanvas: {} x {}\nSummary: {}\n",
        report["canvas"]["width"], report["canvas"]["height"], report["summary"]
    );
    for (heading, field) in [
        ("State", "states"),
        ("Edge", "edges"),
        ("Finding", "findings"),
    ] {
        if let Some(entries) = report[field].as_array() {
            for entry in entries {
                writeln!(text, "{heading}: {entry}").expect("writing to String cannot fail");
            }
        }
    }
    writeln!(
        text,
        "Policy: {}\nCollection limits: {}",
        report["policy"], report["limits"]
    )
    .expect("writing to String cannot fail");
    text
}
