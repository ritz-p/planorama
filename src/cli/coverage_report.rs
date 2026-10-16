use std::path::{Path, PathBuf};

#[derive(Default)]
pub(super) struct Options {
    pub path: Option<PathBuf>,
}

impl Options {
    pub fn validate(
        &self,
        inputs: impl Iterator<Item = impl AsRef<str>>,
        output: &Path,
        other_report: Option<&Path>,
    ) -> Result<(), String> {
        let Some(path) = &self.path else {
            return Ok(());
        };
        super::files::validate_report(path, inputs, output, "--coverage-report")?;
        if let Some(other) = other_report {
            super::files::validate_report(
                path,
                std::iter::empty::<&str>(),
                other,
                "--coverage-report",
            )?;
        }
        Ok(())
    }

    pub fn write<'a>(
        &self,
        states: impl Iterator<Item = (&'a str, &'a crate::coverage::Coverage)>,
        options: &crate::view::Options,
    ) -> Result<(), String> {
        if let Some(path) = &self.path {
            std::fs::write(path, crate::coverage::report::render(states, options))
                .map_err(|e| format!("cannot write coverage report {}: {e}", path.display()))?;
        }
        Ok(())
    }
}
