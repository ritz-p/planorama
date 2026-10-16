use crate::coverage::{Counts, Coverage};
use std::fmt::Write;

fn add(total: &mut Counts, counts: &Counts) {
    total.resources += counts.resources;
    total.account_applicable += counts.account_applicable;
    total.unknown_account += counts.unknown_account;
    total.region_applicable += counts.region_applicable;
    total.unknown_region += counts.unknown_region;
}

fn counts(counts: &Counts) -> serde_json::Value {
    serde_json::json!({
        "resources": counts.resources,
        "account_applicable": counts.account_applicable,
        "unknown_account": counts.unknown_account,
        "region_applicable": counts.region_applicable,
        "unknown_region": counts.unknown_region,
    })
}

pub(super) fn card<'a>(
    states: impl Iterator<Item = (&'a str, &'a Coverage)>,
) -> super::annotations::Card {
    let mut input = Counts::default();
    let mut selected = Counts::default();
    let mut findings = std::collections::BTreeMap::new();
    let mut records = Vec::new();
    let (mut collected, mut focused, mut changed_only) = (true, false, false);
    let mut unassociated = 0;
    let mut complete = Some(true);
    for (state, coverage) in states {
        add(&mut input, &coverage.input);
        add(&mut selected, &coverage.selected);
        for (category, count) in &coverage.selected_findings {
            *findings.entry(*category).or_insert(0) += count;
        }
        collected &= coverage.collected;
        focused |= coverage.focused;
        changed_only |= coverage.changed_only;
        unassociated += coverage.unassociated_findings;
        complete = match (complete, coverage.input_complete) {
            (Some(false), _) | (_, Some(false)) => Some(false),
            (None, _) | (_, None) => None,
            _ => Some(true),
        };
        records.push(serde_json::json!({"state": state, "input": counts(&coverage.input), "selected": counts(&coverage.selected), "input_complete": coverage.input_complete, "collected": coverage.collected}));
    }
    let excluded = input.resources.saturating_sub(selected.resources);
    let metadata = serde_json::json!({"schema_version": 1, "collected": collected,
        "analysis": "supported_rules_only", "input_complete": complete,
        "input": counts(&input), "selected": counts(&selected), "excluded": excluded,
        "selected_findings": findings, "unassociated_input_findings": unassociated,
        "focused": focused, "changed_only": changed_only, "states": records});
    let unknown = |count, applicable| {
        if applicable == 0 {
            "not applicable".to_owned()
        } else {
            format!("{count}/{applicable}")
        }
    };
    let mut lines = if collected {
        vec![
            format!(
                "Resources: input {} | selected {} | excluded {}",
                input.resources, selected.resources, excluded
            ),
            format!(
                "Selection: {}",
                match (focused, changed_only) {
                    (true, true) => "focus + changed-only + required context",
                    (true, false) => "focus + required context",
                    (false, true) => "changed-only + required context",
                    (false, false) => "no CLI filter",
                }
            ),
            format!(
                "Selected unknown scope: account {}; region {}",
                unknown(selected.unknown_account, selected.account_applicable),
                unknown(selected.unknown_region, selected.region_applicable)
            ),
            format!(
                "Selected reference findings (subset): unresolved {}; ambiguous {}; dynamic {}",
                findings.get("unresolved").unwrap_or(&0),
                findings.get("ambiguous").unwrap_or(&0),
                findings.get("dynamic").unwrap_or(&0)
            ),
            format!(
                "Input completeness: {}; analysis: supported rules only",
                match complete {
                    Some(true) => "complete",
                    Some(false) => "incomplete",
                    None => "unknown",
                }
            ),
        ]
    } else {
        vec!["Resource coverage and semantic analysis unavailable".into()]
    };
    if records.len() > 1 {
        lines.insert(0, format!("Whole diagram: {} states", records.len()));
    }
    if unassociated > 0 {
        lines.push(format!(
            "Unassociated input findings: {unassociated} (not in selected counts)"
        ));
    }
    lines.extend([
        "Excluded means hidden by selection or projection, not absent from Terraform.".into(),
        "Scope fractions count applicable resources.".into(),
        "Finding categories overlap; totals are not unique resources.".into(),
        "Counts exclude group/component cards. Unknowns are informational.".into(),
    ]);
    let width = super::annotations::card_width(lines.iter());
    let wrapped: Vec<_> = lines
        .iter()
        .flat_map(|line| super::annotations::wrap(line, (width - 32) / 8))
        .collect();
    let height = wrapped.len() * 20 + 32;
    let mut content = format!(
        "<metadata data-view-coverage=\"true\">{}</metadata>",
        super::escape(&metadata.to_string())
    );
    for (index, line) in wrapped.iter().enumerate() {
        write!(content, "<text x=\"16\" y=\"{}\" font-family=\"ui-monospace, Consolas, monospace\" font-size=\"12\" fill=\"#334155\">{}</text>", 28 + index * 20, super::escape(line)).unwrap();
    }
    super::annotations::Card {
        width,
        height,
        content,
    }
}

#[cfg(test)]
#[path = "../../tests/unit/svg/coverage.rs"]
mod tests;
