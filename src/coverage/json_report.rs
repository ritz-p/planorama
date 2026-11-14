use super::Coverage;
use serde_json::{Value, json};
use std::collections::BTreeMap;

fn selection<'a>(coverages: impl Iterator<Item = &'a Coverage>) -> Value {
    let mut direct = 0;
    let mut effective = 0;
    let mut reasons = BTreeMap::new();
    for coverage in coverages {
        let Some(selection) = coverage.selection.as_ref().filter(|_| coverage.collected) else {
            return json!({
                "collected": false, "direct": null, "additional_context": null,
                "retention_reasons": null,
            });
        };
        direct += selection.direct();
        effective += coverage.selected.resources;
        for entries in selection.reasons.values() {
            for reason in entries {
                *reasons.entry(*reason).or_insert(0usize) += 1;
            }
        }
    }
    json!({
        "collected": true, "direct": direct,
        "additional_context": effective.saturating_sub(direct),
        "retention_reasons": reasons,
    })
}

fn summary(coverage: &Coverage, selection: Value) -> Value {
    let available = coverage.collected;
    json!({
        "collected": available,
        "input_complete": if available { coverage.input_complete } else { None },
        "input": available.then_some(&coverage.input),
        "selected": available.then_some(&coverage.selected),
        "excluded": available.then(|| coverage.input.resources.saturating_sub(coverage.selected.resources)),
        "input_findings": available.then_some(&coverage.input_findings),
        "selected_findings": available.then_some(&coverage.selected_findings),
        "unassociated_input_findings": available.then_some(coverage.unassociated_findings),
        "selection": selection,
    })
}

fn identity(
    state: &str,
    id: &crate::model::TerraformEntityId,
    mode: crate::model::EntityMode,
) -> Value {
    json!({
        "state": state, "address": id.address, "deposed": id.deposed_key,
        "mode": match mode {
            crate::model::EntityMode::Managed => "managed",
            crate::model::EntityMode::Data => "data",
        },
    })
}

fn records(total: Option<usize>, records: Vec<Value>, limit: usize) -> Value {
    let omitted = total.map(|total| total.saturating_sub(records.len()));
    json!({
        "total": total, "omitted": omitted,
        "records_complete": omitted == Some(0),
        "truncated": limit > 0 && omitted.is_some_and(|n| n > 0),
        "records": records,
    })
}

pub(crate) fn render<'a>(
    states: impl Iterator<Item = (&'a str, &'a Coverage)>,
    limit: usize,
) -> Value {
    let mut states: Vec<_> = states.collect();
    states.sort_by_key(|(id, _)| *id);
    let total = super::report::aggregate(states.iter().map(|(_, coverage)| *coverage));
    let mut resources = Vec::new();
    let mut findings = Vec::new();
    let finding_count = total.collected.then(|| {
        states
            .iter()
            .map(|(_, coverage)| coverage.findings.len())
            .sum()
    });
    if limit > 0 && total.collected {
        for (state, coverage) in &states {
            for (id, resource) in &coverage.resources {
                if resources.len() == limit {
                    break;
                }
                let reasons = coverage.selection.as_ref().and_then(|s| s.reasons.get(id));
                let mut record = identity(state, id, resource.mode);
                record["selection"] = json!(reasons.map(|reasons| {
                    if reasons.contains("direct selection") {
                        "direct"
                    } else {
                        "context"
                    }
                }));
                record["retention_reasons"] = json!(reasons);
                resources.push(record);
            }
            let mut ordered: Vec<_> = coverage.findings.iter().collect();
            ordered.sort();
            for finding in ordered {
                if findings.len() == limit {
                    break;
                }
                let Some(resource) = coverage.resources.get(&finding.source) else {
                    continue;
                };
                let mut record = identity(state, &finding.source, resource.mode);
                record["attribute"] = json!(finding.attribute);
                record["reason"] = json!(finding.code);
                record["category"] = json!(finding.category);
                record["message"] = json!(finding.message);
                findings.push(record);
            }
        }
    }
    json!({
        "schema_version": 1,
        "analysis": "state_local_supported_rules_only",
        "cross_state": "not_collected",
        "summary": summary(&total, selection(states.iter().map(|(_, coverage)| *coverage))),
        "states": states.iter().map(|(state, coverage)| json!({
            "state": state, "summary": summary(coverage, selection(std::iter::once(*coverage))),
        })).collect::<Vec<_>>(),
        "details": {
            "enabled": limit > 0, "limit": limit,
            "resources": records(total.collected.then_some(total.selected.resources), resources, limit),
            "findings": records(finding_count, findings, limit),
        },
    })
}

#[cfg(test)]
#[path = "../../tests/unit/coverage_json.rs"]
mod tests;
