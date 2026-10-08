use crate::model::{Action, ChangeMetadata};
use std::fmt::Write;

pub(super) fn label(action: Action, metadata: &ChangeMetadata) -> String {
    let operation = metadata
        .state_removal
        .map(|removal| removal.label())
        .unwrap_or(super::style::label(action));
    if metadata.import.is_some() {
        format!("import / {operation}")
    } else {
        operation.into()
    }
}

pub(super) fn attributes(metadata: &ChangeMetadata) -> String {
    let mut result = String::new();
    if !metadata.drift.is_empty() {
        let actions = metadata
            .drift
            .iter()
            .map(|action| super::style::label(*action))
            .collect::<Vec<_>>()
            .join(",");
        write!(result, " data-drift=\"{actions}\"").unwrap();
    }
    if let Some(import) = metadata.import {
        write!(result, " data-import=\"true\" data-import-id-present=\"{}\" data-import-identity-present=\"{}\" data-import-unknown=\"{}\"", import.has_id, import.has_identity, import.unknown).unwrap();
    }
    if let Some(removal) = metadata.state_removal {
        write!(result, " data-state-removal=\"{}\"", removal.label()).unwrap();
    }
    result
}
