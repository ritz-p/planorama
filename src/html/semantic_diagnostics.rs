use crate::coverage::Coverage;
use serde_json::{Value, json};

pub(super) fn bridge<'a>(states: impl Iterator<Item = (&'a str, &'a Coverage)>) -> Option<Value> {
    let states: Vec<_> = states.collect();
    if !states.iter().any(|(_, coverage)| coverage.embed_findings) {
        return None;
    }
    let mut records = Vec::new();
    let mut bytes = 0;
    let mut total = 0;
    let mut unassociated = 0;
    let mut collected = true;
    let mut input_complete = Some(true);
    for (state, coverage) in states {
        total += coverage.findings.len();
        unassociated += coverage.unassociated_findings;
        collected &= coverage.collected;
        input_complete = match (input_complete, coverage.input_complete) {
            (Some(false), _) | (_, Some(false)) => Some(false),
            (None, _) | (_, None) => None,
            _ => Some(true),
        };
        for finding in &coverage.findings {
            let Some(resource) = coverage.resources.get(&finding.source) else {
                continue;
            };
            let record = json!({
                "state": state, "address": finding.source.address,
                "deposed": finding.source.deposed_key,
                "mode": match resource.mode { crate::model::EntityMode::Managed => "managed", crate::model::EntityMode::Data => "data" },
                "attribute": finding.attribute, "reason": finding.code,
                "category": finding.category, "message": finding.message,
            });
            let size = super::safe_json(&record).len() + 1;
            if records.len() < 1000 && bytes + size <= 256 * 1024 - 1024 {
                bytes += size;
                records.push(record);
            }
        }
    }
    Some(json!({"schema_version": 1, "available": collected,
        "analysis": "supported_rules_only", "cross_state": "not_collected",
        "input_complete": input_complete, "total": total,
        "omitted": total - records.len(), "records_complete": total == records.len(),
        "unassociated_input_findings": unassociated, "records": records}))
}

#[cfg(test)]
#[path = "../../tests/unit/html_semantic_diagnostics.rs"]
mod tests;
