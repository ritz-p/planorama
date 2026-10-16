use super::{Counts, Coverage};
use std::fmt::Write;

fn escape(text: &str) -> String {
    text.chars()
        .map(|c| match c {
            '\n' | '\r' | '\t' => " ".into(),
            c if c.is_control() => String::new(),
            c if c.is_ascii_punctuation() => format!("&#{};", c as u32),
            c => c.to_string(),
        })
        .collect()
}

pub(crate) fn render<'a>(
    states: impl Iterator<Item = (&'a str, &'a Coverage)>,
    options: &crate::view::Options,
) -> String {
    let mut states: Vec<_> = states.collect();
    states.sort_by_key(|(id, _)| *id);
    let mut total = Coverage {
        collected: true,
        input_complete: Some(true),
        ..Default::default()
    };
    for (_, coverage) in &states {
        total.collected &= coverage.collected;
        total.input_complete = match (total.input_complete, coverage.input_complete) {
            (Some(false), _) | (_, Some(false)) => Some(false),
            (None, _) | (_, None) => None,
            _ => Some(true),
        };
        for (sum, counts) in [
            (&mut total.input, &coverage.input),
            (&mut total.selected, &coverage.selected),
        ] {
            sum.resources += counts.resources;
            sum.account_applicable += counts.account_applicable;
            sum.unknown_account += counts.unknown_account;
            sum.region_applicable += counts.region_applicable;
            sum.unknown_region += counts.unknown_region;
        }
        for (sum, counts) in [
            (&mut total.input_findings, &coverage.input_findings),
            (&mut total.selected_findings, &coverage.selected_findings),
        ] {
            for (category, count) in counts {
                *sum.entry(category).or_default() += count;
            }
        }
        total.unassociated_findings += coverage.unassociated_findings;
    }
    let mut text = String::from("# Terraform coverage report\n\n");
    let selection = match (options.focus.is_some(), options.changed_only) {
        (true, true) => "focus + changed-only + required context",
        (true, false) => "focus + required context",
        (false, true) => "changed-only + required context",
        (false, false) if options.modules.is_empty() => "no CLI filter",
        (false, false) => "modules + required context",
    };
    writeln!(text, "Selection: {selection}.\n").unwrap();
    if !options.modules.is_empty() {
        let modules: std::collections::BTreeSet<_> = options.modules.iter().collect();
        writeln!(text, "Modules (union, intersected with other filters): {}. Required containment, Route Table and cross-state context can be outside these modules.\n", modules.into_iter().map(|module| escape(module)).collect::<Vec<_>>().join(", ")).unwrap();
    }
    if let Some(focus) = &options.focus {
        writeln!(
            text,
            "Focus: {}; neighbor depth: {}.\n",
            escape(focus),
            options.depth.unwrap_or(1)
        )
        .unwrap();
    }
    text.push_str("Input counts original Terraform resource instances. Selected counts distinct source instances represented after selection and projection, including logical rows, embedded Routes and lowered relationship provenance exactly once. Excluded = input - selected; excluded instances may be hidden by filters or projection. Group and component cards are not resources. Managed/data addresses and current/deposed instances remain distinct; states are counted separately.\n\nSupported means the provider has a semantic classification for the resource type; unsupported means no such classification. Logical presentation does not imply unsupported.\n\nUnknown account denominators include managed AWS instances. Unknown region denominators include classified, non-global managed AWS instances. Data sources and non-AWS resources are outside these scope denominators. Zero applicable instances means not applicable.\n\nFindings count diagnostic records, not unique resources. Categories can overlap and must not be summed into a resource total or coverage percentage. Input findings include unassociated findings; selected findings only belong to represented instances. Analysis covers supported rules only; zero findings does not prove complete resolution. Cross-state resolution findings are not included in this analysis.\n\nThis aggregate report does not enumerate resource findings or excluded identities. Aggregate counts are not truncated. Interactive selected finding details are available through --html-semantic-diagnostics, with its own disclosed limits.\n\n");
    section(&mut text, "Whole diagram", &total);
    for (state, coverage) in states {
        section(
            &mut text,
            &format!(
                "State: {}",
                if state.is_empty() {
                    "(single input)".into()
                } else {
                    escape(state)
                }
            ),
            coverage,
        );
    }
    text
}

fn section(text: &mut String, title: &str, coverage: &Coverage) {
    writeln!(text, "## {title}\n").unwrap();
    if !coverage.collected {
        text.push_str(
            "Resource accounting and semantic analysis unavailable; counts are unknown.\n\n",
        );
        return;
    }
    writeln!(
        text,
        "Input completeness: {}. Analysis: supported rules only.\n",
        match coverage.input_complete {
            Some(true) => "complete",
            Some(false) => "incomplete",
            None => "unknown",
        }
    )
    .unwrap();
    text.push_str("| Accounting | Input | Selected |\n| --- | ---: | ---: |\n");
    writeln!(
        text,
        "| Resource instances | {} | {} |",
        coverage.input.resources, coverage.selected.resources
    )
    .unwrap();
    for (label, unsupported) in [
        ("Supported classification", false),
        ("Unsupported classification", true),
    ] {
        let count = |counts: &Counts, findings: &std::collections::BTreeMap<&str, usize>| {
            let unknown = findings.get("classification").copied().unwrap_or(0);
            if unsupported {
                unknown
            } else {
                counts.resources.saturating_sub(unknown)
            }
        };
        writeln!(
            text,
            "| {label} | {} | {} |",
            count(&coverage.input, &coverage.input_findings),
            count(&coverage.selected, &coverage.selected_findings)
        )
        .unwrap();
    }
    for (label, account) in [
        ("Unknown account / applicable", true),
        ("Unknown region / applicable", false),
    ] {
        let fraction = |counts: &Counts| {
            let (unknown, denominator) = if account {
                (counts.unknown_account, counts.account_applicable)
            } else {
                (counts.unknown_region, counts.region_applicable)
            };
            if denominator == 0 {
                "not applicable (0)".into()
            } else {
                format!("{unknown} / {denominator}")
            }
        };
        writeln!(
            text,
            "| {label} | {} | {} |",
            fraction(&coverage.input),
            fraction(&coverage.selected)
        )
        .unwrap();
    }
    writeln!(
        text,
        "\nExcluded resource instances: {}.\n",
        coverage
            .input
            .resources
            .saturating_sub(coverage.selected.resources)
    )
    .unwrap();
    text.push_str("| Finding category | Input | Selected |\n| --- | ---: | ---: |\n");
    for category in [
        "unresolved",
        "ambiguous",
        "dynamic",
        "unavailable_endpoint",
        "classification",
        "scope",
        "drift",
    ] {
        writeln!(
            text,
            "| {} | {} | {} |",
            escape(category),
            coverage.input_findings.get(category).unwrap_or(&0),
            coverage.selected_findings.get(category).unwrap_or(&0)
        )
        .unwrap();
    }
    writeln!(
        text,
        "\nUnassociated input findings: {} (included only in input categories).\n",
        coverage.unassociated_findings
    )
    .unwrap();
}

#[cfg(test)]
#[path = "../../tests/unit/coverage_report.rs"]
mod tests;
