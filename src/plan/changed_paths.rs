use crate::model::ChangedPaths;
use serde_json::Value;
use std::collections::BTreeSet;

const PATH_LIMIT: usize = 128;
const BYTE_LIMIT: usize = 8192;
const VISIT_LIMIT: usize = 100_000;

fn sensitive(value: &Value) -> bool {
    match value {
        Value::Bool(true) => true,
        Value::Object(values) => values.values().any(sensitive),
        Value::Array(values) => values.iter().any(sensitive),
        _ => false,
    }
}

struct Collector {
    result: ChangedPaths,
    bytes: usize,
    visited: usize,
}

impl Collector {
    fn add(&mut self, path: String, kind: u8) {
        let size = path.len().saturating_mul(6).saturating_add(3);
        let count = self.result.paths.len()
            + self.result.unknown_paths.len()
            + self.result.replacement_paths.len();
        if count >= PATH_LIMIT || self.bytes + size > BYTE_LIMIT - 512 {
            self.result.truncated = true;
            return;
        }
        self.bytes += size;
        match kind {
            0 => self.result.paths.push(path),
            1 => self.result.unknown_paths.push(path),
            _ => self.result.replacement_paths.push(path),
        }
    }

    fn walk(
        &mut self,
        before: Option<&Value>,
        after: Option<&Value>,
        unknown: &Value,
        before_sensitive: &Value,
        after_sensitive: &Value,
        path: &str,
    ) {
        self.visited += 1;
        if self.visited > VISIT_LIMIT {
            self.result.truncated = true;
            return;
        }
        if before_sensitive == &Value::Bool(true) || after_sensitive == &Value::Bool(true) {
            self.result.sensitive_omitted = true;
            return;
        }
        if unknown == &Value::Bool(true) {
            self.add(path.to_owned(), 1);
            return;
        }
        let object = |value: Option<&Value>| value.is_some_and(Value::is_object);
        let absent = |value: Option<&Value>| value.is_none_or(Value::is_null);
        if (object(before) || object(after) || unknown.is_object())
            && (object(before) || absent(before))
            && (object(after) || absent(after))
        {
            let keys: BTreeSet<_> = before
                .and_then(Value::as_object)
                .into_iter()
                .flat_map(|v| v.keys())
                .chain(
                    after
                        .and_then(Value::as_object)
                        .into_iter()
                        .flat_map(|v| v.keys()),
                )
                .chain(unknown.as_object().into_iter().flat_map(|v| v.keys()))
                .collect();
            if keys.is_empty() && before != after {
                self.add(path.to_owned(), 0);
            }
            for key in keys {
                if sensitive(&before_sensitive[key]) || sensitive(&after_sensitive[key]) {
                    self.result.sensitive_omitted = true;
                    continue;
                }
                let next = format!("{path}[{}]", Value::String(key.clone()));
                if next.len() > BYTE_LIMIT {
                    self.result.truncated = true;
                    continue;
                }
                self.walk(
                    before.and_then(|v| v.get(key)),
                    after.and_then(|v| v.get(key)),
                    &unknown[key],
                    &before_sensitive[key],
                    &after_sensitive[key],
                    &next,
                );
                if self.visited > VISIT_LIMIT {
                    break;
                }
            }
        } else if (before.is_some_and(Value::is_array)
            || after.is_some_and(Value::is_array)
            || unknown.is_array())
            && (before.is_some_and(Value::is_array) || absent(before))
            && (after.is_some_and(Value::is_array) || absent(after))
        {
            let len = [before, after, Some(unknown)]
                .into_iter()
                .filter_map(|v| v.and_then(Value::as_array).map(Vec::len))
                .max()
                .unwrap_or(0);
            if len == 0 && before != after {
                self.add(path.to_owned(), 0);
            }
            for index in 0..len {
                if sensitive(&before_sensitive[index]) || sensitive(&after_sensitive[index]) {
                    self.result.sensitive_omitted = true;
                    continue;
                }
                self.walk(
                    before.and_then(|v| v.get(index)),
                    after.and_then(|v| v.get(index)),
                    &unknown[index],
                    &before_sensitive[index],
                    &after_sensitive[index],
                    &format!("{path}[{index}]"),
                );
                if self.visited > VISIT_LIMIT {
                    break;
                }
            }
        } else if before != after {
            self.add(path.to_owned(), 0);
        }
    }
}

pub(super) fn collect(change: &Value) -> ChangedPaths {
    let mut collector = Collector {
        result: ChangedPaths::default(),
        bytes: 0,
        visited: 0,
    };
    if let (Some(before), Some(after)) = (change.get("before"), change.get("after")) {
        collector.result.available = true;
        collector.walk(
            Some(before),
            Some(after),
            &change["after_unknown"],
            &change["before_sensitive"],
            &change["after_sensitive"],
            "$",
        );
    }
    if let Some(paths) = change["replace_paths"].as_array() {
        collector.result.replacement_available = true;
        for path in paths.iter().take(VISIT_LIMIT) {
            let Some(steps) = path.as_array() else {
                collector.result.replacement_available = false;
                continue;
            };
            if super::operations::sensitive_path(&change["before_sensitive"], steps)
                || super::operations::sensitive_path(&change["after_sensitive"], steps)
            {
                collector.result.sensitive_omitted = true;
                continue;
            }
            let mut formatted = String::from("$");
            let mut valid = true;
            for step in steps {
                if !(step.is_string() || step.as_u64().is_some()) {
                    collector.result.replacement_available = false;
                    valid = false;
                    break;
                }
                formatted.push_str(&format!("[{step}]"));
                if formatted.len() > BYTE_LIMIT {
                    collector.result.truncated = true;
                    valid = false;
                    break;
                }
            }
            if valid {
                collector.add(formatted, 2);
            }
        }
        collector.result.truncated |= paths.len() > VISIT_LIMIT;
    }
    collector.result
}

#[cfg(test)]
#[path = "../../tests/unit/plan/changed_paths.rs"]
mod tests;
