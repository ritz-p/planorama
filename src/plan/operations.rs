use crate::model::{ChangeMetadata, ImportMetadata, StateRemoval};
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
    // Match full arrays, including order, rather than guessing from one token.
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
    }
}
