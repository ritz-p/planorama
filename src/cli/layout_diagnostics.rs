use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Default)]
pub(super) struct Options {
    pub html: bool,
    pub text: bool,
    pub json: Option<PathBuf>,
}

impl Options {
    pub fn enabled(&self) -> bool {
        self.html || self.text || self.json.is_some()
    }

    pub fn validate(
        &self,
        inputs: impl Iterator<Item = impl AsRef<str>>,
        output: &Path,
    ) -> Result<(), String> {
        let Some(path) = &self.json else {
            return Ok(());
        };
        super::files::validate_report(path, inputs, output, "--layout-diagnostics-json")
    }

    pub fn write(&self, report: &serde_json::Value, output: &Path) -> Result<(), String> {
        if let Some(path) = &self.json {
            let mut json = serde_json::to_string_pretty(report).map_err(|e| e.to_string())?;
            json.push('\n');
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(false)
                .open(path)
                .map_err(|e| format!("cannot write layout diagnostics {}: {e}", path.display()))?;
            if output != Path::new("-") {
                super::files::reject_same_file(path, output)?;
            }
            file.set_len(0)
                .and_then(|()| file.write_all(json.as_bytes()))
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
