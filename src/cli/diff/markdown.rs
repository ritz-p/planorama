use super::{Comparison, Key, Link, Names, Snapshot};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;

fn code_span(value: &str) -> String {
    let mut text = String::new();
    let (mut ticks, mut longest) = (0, 0);
    for ch in value.chars() {
        ticks = if ch == '`' { ticks + 1 } else { 0 };
        longest = longest.max(ticks);
        match ch {
            '\n' => text.push_str("\\n"),
            '\r' => text.push_str("\\r"),
            '\t' => text.push_str("\\t"),
            c if c.is_control()
                || matches!(c, '\u{2028}'..='\u{202e}' | '\u{2066}'..='\u{2069}') =>
            {
                write!(text, "\\u{{{:x}}}", c as u32).unwrap();
            }
            c => text.push(c),
        }
    }
    let fence = "`".repeat(longest + 1);
    format!("{fence} {text} {fence}")
}

fn identifier(key: &Key, names: &Names<'_>) -> String {
    if let Some(ids) = names {
        let id = ids[key];
        return format!(
            "{} {id}",
            if key.component {
                "Component"
            } else {
                "Resource"
            }
        );
    }
    let mut text = format!("{}:{}", key.state, key.address);
    if let Some(deposed) = &key.deposed {
        write!(text, " (deposed {deposed})").unwrap();
    }
    code_span(&text)
}

type Anchor<'a> = (&'a Key, &'static str, &'static str);

fn anchors(link: &Link) -> impl Iterator<Item = Anchor<'_>> {
    let (first, second) = match (link.kind, link.direction) {
        ("containment", _) => ((&link.to, "parent"), None),
        (_, "undirected") => (
            (&link.from, "peer"),
            (link.from != link.to).then_some((&link.to, "peer")),
        ),
        _ => ((&link.from, "target"), Some((&link.to, "source"))),
    };
    std::iter::once(first)
        .chain(second)
        .map(|(key, role)| (key, link.kind, role))
}

fn candidates<'a>(
    changes: &'a BTreeSet<Link>,
    snapshot: &'a Snapshot,
    diff: &'a Comparison,
    before: bool,
) -> BTreeMap<Anchor<'a>, &'a Link> {
    let mut candidates = BTreeMap::new();
    let mut ambiguous = BTreeSet::new();
    for link in changes {
        for anchor in anchors(link) {
            if candidates.insert(anchor, link).is_some() {
                ambiguous.insert(anchor);
            }
        }
    }
    for link in &snapshot.edges {
        if diff.unknown.contains(&link.from) || diff.unknown.contains(&link.to) {
            for (key, kind, role) in anchors(link) {
                let key = if before {
                    diff.mapping.get(key).unwrap_or(key)
                } else {
                    key
                };
                ambiguous.insert((key, kind, role));
            }
        }
    }
    candidates.retain(|a, _| !ambiguous.contains(a));
    candidates
}

fn peer<'a>(link: &'a Link, anchor: &Key) -> &'a Key {
    if &link.from == anchor {
        &link.to
    } else {
        &link.from
    }
}

fn section(output: &mut String, heading: &str, lines: impl IntoIterator<Item = String>) {
    let mut lines = lines.into_iter().peekable();
    if lines.peek().is_none() {
        return;
    }
    writeln!(output, "\n## {heading}\n").unwrap();
    for line in lines {
        writeln!(output, "- {line}").unwrap();
    }
}

pub(super) fn render(
    diff: &Comparison,
    names: &Names<'_>,
    before: &Snapshot,
    after: &Snapshot,
) -> String {
    let title = "# Architecture diff\n";
    let mut out = title.to_owned();
    let show = |key: &Key| identifier(key, names);
    section(&mut out, "Resources added", diff.added.iter().map(show));
    section(&mut out, "Resources removed", diff.removed.iter().map(show));
    section(
        &mut out,
        "Resources changed",
        diff.changed.iter().map(|change| {
            let mut line = show(&change.before);
            if change.before != change.after {
                write!(line, " → {}", show(&change.after)).unwrap();
            }
            if !change.fields.is_empty() {
                write!(line, " ({})", change.fields.join(", ")).unwrap();
            }
            line
        }),
    );

    let removed_candidates = candidates(&diff.removed_links, before, diff, true);
    let added_candidates = candidates(&diff.added_links, after, diff, false);
    let predecessors: BTreeMap<_, _> = diff.mapping.iter().map(|(old, new)| (new, old)).collect();
    let show_old = |key: &Key| show(predecessors.get(key).copied().unwrap_or(key));
    let mut proposals = Vec::new();
    let mut removed_counts = BTreeMap::new();
    let mut added_counts = BTreeMap::new();
    for (anchor, removed) in removed_candidates {
        if !predecessors.contains_key(anchor.0) {
            continue;
        }
        if let Some(&added) = added_candidates.get(&anchor) {
            proposals.push((removed, added, anchor));
            *removed_counts.entry(removed).or_insert(0) += 1;
            *added_counts.entry(added).or_insert(0) += 1;
        }
    }
    let mut paired = BTreeSet::new();
    let mut parents = Vec::new();
    let mut targets = Vec::new();
    for (removed, added, (anchor, _, role)) in proposals {
        if removed_counts[removed] != 1 || added_counts[added] != 1 {
            continue;
        }
        paired.insert(removed);
        paired.insert(added);
        let from = show_old(peer(removed, anchor));
        let to = show(peer(added, anchor));
        let anchor = show(anchor);
        if added.kind == "containment" {
            parents.push(format!("{anchor} moved from {from} to {to}"));
        } else {
            targets.push(format!(
                "{anchor} — {} {} changed from {from} to {to}",
                added.kind, role
            ));
        }
    }
    for (group, changed_heading, changes, containment) in [
        ("Containment", "Containment changed", parents, true),
        (
            "Relationships",
            "Relationship endpoints changed",
            targets,
            false,
        ),
    ] {
        section(&mut out, changed_heading, changes);
        for (action, links) in [
            ("added", &diff.added_links),
            ("removed", &diff.removed_links),
        ] {
            let display = |key: &Key| {
                if action == "removed" {
                    show_old(key)
                } else {
                    show(key)
                }
            };
            let lines = links
                .iter()
                .filter(|e| (e.kind == "containment") == containment && !paired.contains(e))
                .map(|e| {
                    let from = display(&e.from);
                    let to = display(&e.to);
                    if containment {
                        format!("{from} contains {to}")
                    } else {
                        let arrow = if e.direction == "undirected" {
                            "↔"
                        } else {
                            "→"
                        };
                        format!("{from} {arrow} {to} ({})", e.kind)
                    }
                });
            section(&mut out, &format!("{group} {action}"), lines);
        }
    }
    section(
        &mut out,
        "Unknown resource matches",
        diff.unknown.iter().map(show),
    );
    if !diff.unknown.is_empty() {
        out.push_str("\nRelationships involving these identities could not be compared.\n");
    }
    if out == title {
        out.push_str("\nNo architecture changes.\n");
    }
    out
}
