use crate::model::{AttributePathStep, ChangeMetadata, ImportMetadata, StateRemoval};
use serde_json::Value;

#[cfg(test)]
#[path = "../../tests/unit/plan/operations.rs"]
mod tests;

pub(super) fn parse(change: &Value) -> ChangeMetadata {
    let import = change["importing"]
        .as_object()
        .map(|metadata| ImportMetadata {
            has_id: metadata.get("id").is_some_and(Value::is_string),
            has_identity: metadata.get("identity").is_some_and(|v| !v.is_null()),
            unknown: metadata
                .get("unknown")
                .and_then(Value::as_bool)
                .unwrap_or(false),
        });
    let actions = change["actions"]
        .as_array()
        .and_then(|values| values.iter().map(Value::as_str).collect::<Option<Vec<_>>>());
    let state_removal = match actions.as_deref() {
        Some(["forget"]) => Some(StateRemoval::Forget),
        Some(["create", "forget"]) => Some(StateRemoval::CreateThenForget),
        Some(["forget", "create"]) => Some(StateRemoval::ForgetThenCreate),
        _ => None,
    };
    ChangeMetadata {
        import,
        state_removal,
        replace_paths: change["replace_paths"].as_array().map(|paths| {
            paths
                .iter()
                .filter_map(|path| {
                    let steps = path.as_array()?;
                    if ["before_sensitive", "after_sensitive"]
                        .iter()
                        .any(|field| sensitive_path(&change[*field], steps))
                    {
                        return None;
                    }
                    steps
                        .iter()
                        .map(|step| match step {
                            Value::String(name) => Some(AttributePathStep::Attribute(name.clone())),
                            Value::Number(index) => index.as_u64().map(AttributePathStep::Index),
                            _ => None,
                        })
                        .collect::<Option<Vec<_>>>()
                })
                .collect()
        }),
        ..Default::default()
    }
}

fn contains_sensitive(value: &Value) -> bool {
    match value {
        Value::Bool(true) => true,
        Value::Array(values) => values.iter().any(contains_sensitive),
        Value::Object(values) => values.values().any(contains_sensitive),
        _ => false,
    }
}

pub(super) fn sensitive_path(mut sensitivity: &Value, steps: &[Value]) -> bool {
    if sensitivity == &Value::Bool(true) {
        return true;
    }
    for step in steps {
        sensitivity = match step {
            Value::String(key) => sensitivity.get(key),
            Value::Number(index) => index
                .as_u64()
                .and_then(|i| usize::try_from(i).ok())
                .and_then(|i| sensitivity.get(i)),
            _ => None,
        }
        .unwrap_or(&Value::Null);
        if contains_sensitive(sensitivity) {
            return true;
        }
    }
    false
}
