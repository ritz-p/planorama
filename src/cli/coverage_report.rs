use std::path::{Path, PathBuf};

#[derive(Default)]
pub(super) struct Options {
    pub path: Option<PathBuf>,
    pub json: Option<PathBuf>,
    pub details: usize,
}

impl Options {
    pub fn validate(
        &self,
        inputs: impl Iterator<Item = impl AsRef<str>>,
        output: &Path,
        other_report: Option<&Path>,
    ) -> Result<(), String> {
        let inputs: Vec<_> = inputs.map(|input| input.as_ref().to_owned()).collect();
        for (path, option) in [
            (&self.path, "--coverage-report"),
            (&self.json, "--coverage-report-json"),
        ] {
            if let Some(path) = path {
                super::files::validate_report(path, inputs.iter(), output, option)?;
                if let Some(other) = other_report {
                    super::files::validate_report(path, std::iter::empty::<&str>(), other, option)?;
                }
            }
        }
        if let (Some(path), Some(json)) = (&self.path, &self.json) {
            super::files::validate_report(
                json,
                std::iter::empty::<&str>(),
                path,
                "--coverage-report-json",
            )?;
        }
        Ok(())
    }

    pub fn write<'a>(
        &self,
        states: impl Iterator<Item = (&'a str, &'a crate::coverage::Coverage)>,
        options: &crate::view::Options,
    ) -> Result<(), String> {
        let states: Vec<_> = states.collect();
        if let Some(path) = &self.json {
            let report = crate::coverage::json_report::render(states.iter().copied(), self.details);
            let mut bytes = serde_json::to_vec_pretty(&report)
                .map_err(|e| format!("cannot serialize coverage JSON: {e}"))?;
            bytes.push(b'\n');
            std::fs::write(path, bytes)
                .map_err(|e| format!("cannot write coverage JSON {}: {e}", path.display()))?;
        }
        if let Some(path) = &self.path {
            std::fs::write(
                path,
                crate::coverage::report::render(states.into_iter(), options),
            )
            .map_err(|e| format!("cannot write coverage report {}: {e}", path.display()))?;
        }
        Ok(())
    }
}
