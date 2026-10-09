use crate::model::architecture::{Action, AttributePathStep, ChangeMetadata};
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
    if let Some(paths) = &metadata.relevant_attributes {
        write!(
            result,
            " data-plan-relevant=\"true\" data-relevant-attributes=\"{}\"",
            super::escape(&serialize_paths(paths))
        )
        .unwrap();
    }
    if let Some(reason) = &metadata.action_reason {
        write!(result, " data-action-reason=\"{}\"", super::escape(reason)).unwrap();
    }
    if let Some(paths) = paths_json(metadata) {
        write!(result, " data-replace-paths=\"{}\"", super::escape(&paths)).unwrap();
    }
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

fn paths_json(metadata: &ChangeMetadata) -> Option<String> {
    metadata
        .replace_paths
        .as_ref()
        .map(|paths| serialize_paths(paths))
}

fn serialize_paths(paths: &[Vec<AttributePathStep>]) -> String {
    path_values(paths).to_string()
}

fn path_values(paths: &[Vec<AttributePathStep>]) -> serde_json::Value {
    serde_json::Value::Array(
        paths
            .iter()
            .map(|path| {
                serde_json::Value::Array(
                    path.iter()
                        .map(|step| match step {
                            AttributePathStep::Attribute(name) => {
                                serde_json::Value::String(name.clone())
                            }
                            AttributePathStep::Index(index) => (*index).into(),
                        })
                        .collect(),
                )
            })
            .collect(),
    )
}

pub(super) fn metadata(metadata: &ChangeMetadata) -> serde_json::Value {
    serde_json::json!({
        "import": metadata.import.map(|import| serde_json::json!({"has_id":import.has_id,"has_identity":import.has_identity,"unknown":import.unknown})),
        "state_removal": metadata.state_removal.map(|removal| removal.label()),
        "drift": metadata.drift.iter().map(|action| super::style::label(*action)).collect::<Vec<_>>(),
        "action_reason": metadata.action_reason,
        "replace_paths": metadata.replace_paths.as_deref().map(path_values),
        "relevant_attributes": metadata.relevant_attributes.as_deref().map(path_values),
    })
}

pub(super) fn details(metadata: &ChangeMetadata) -> String {
    let mut result = String::new();
    if let Some(paths) = &metadata.relevant_attributes {
        write!(
            result,
            "; Terraform-reported plan relevance; attributes: {}",
            serialize_paths(paths)
        )
        .unwrap();
    }
    if let Some(reason) = &metadata.action_reason {
        write!(result, "; reason: {reason}").unwrap();
    }
    if let Some(paths) = paths_json(metadata) {
        write!(result, "; replacement paths: {paths}").unwrap();
    }
    result
}
