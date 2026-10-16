use crate::model::StateId;
use crate::plan::evidence::Evidence;
use std::collections::BTreeMap;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

#[derive(Default)]
pub(super) struct Options {
    single: Option<String>,
    states: BTreeMap<StateId, String>,
}

impl Options {
    pub(super) fn single(&mut self, path: String) -> Result<(), String> {
        if self.single.replace(path).is_some() {
            return Err("duplicate --provider-plan".into());
        }
        Ok(())
    }

    pub(super) fn state(&mut self, value: &str) -> Result<(), String> {
        let (id, path) = value
            .split_once('=')
            .ok_or("--state-provider-plan requires ID=PATH")?;
        let id = StateId::new(id)?;
        if self.states.insert(id, path.into()).is_some() {
            return Err("duplicate --state-provider-plan ID".into());
        }
        Ok(())
    }

    pub(super) fn validate(
        &self,
        states: &BTreeMap<StateId, String>,
        output: &Path,
        report: Option<&Path>,
    ) -> Result<(), String> {
        if (!states.is_empty() && self.single.is_some())
            || (states.is_empty() && !self.states.is_empty())
        {
            return Err("use --provider-plan with a positional input, or --state-provider-plan with --state".into());
        }
        if self.states.keys().any(|id| !states.contains_key(id)) {
            return Err("provider plan refers to an unknown state ID".into());
        }
        for path in self.single.iter().chain(self.states.values()) {
            if path.is_empty() || path == "-" {
                return Err("provider plan requires a saved plan file path".into());
            }
            super::states::protect_input(path, output)?;
            if let Some(report) = report {
                super::states::protect_input(path, report)?;
            }
        }
        Ok(())
    }

    pub(super) fn load(
        &self,
        state: Option<&StateId>,
        json: &str,
    ) -> Result<Option<Evidence>, String> {
        let _timing = crate::timing::span("provider_evidence", &[]);
        let path = match state {
            Some(id) => self.states.get(id),
            None => self.single.as_ref(),
        };
        let Some(path) = path else { return Ok(None) };
        let bytes = std::fs::read(path).map_err(|_| "cannot read provider plan")?;
        let mut copy =
            tempfile::NamedTempFile::new().map_err(|_| "cannot create provider plan copy")?;
        copy.write_all(&bytes)
            .map_err(|_| "cannot copy provider plan")?;
        let mut command = Command::new("terraform");
        command
            .args(["show", "-json"])
            .arg(copy.path())
            .stdin(Stdio::null());
        for (name, _) in std::env::vars_os() {
            if name.to_string_lossy().starts_with("TF_CLI_ARGS")
                || name.to_string_lossy().starts_with("TF_LOG")
            {
                command.env_remove(name);
            }
        }
        let shown = command.output().map_err(
            |_| "provider plan requires Terraform 1.9.8 on PATH and initialized provider schemas",
        )?;
        if !shown.status.success() {
            return Err("provider plan: terraform show failed; check Terraform version and installed provider schemas (details suppressed for privacy)".into());
        }
        Evidence::validate(&bytes, json, &shown.stdout).map(Some)
    }
}
