use crate::model::{Node, Placement, ReferenceTarget, ResourceReference};

pub(crate) const LINE_HEIGHT: usize = 18;
const TEXT_INSET: usize = 32;

pub(crate) struct Row<'a> {
    pub reference: &'a ResourceReference,
    pub target: Option<&'a ReferenceTarget>,
}

impl Row<'_> {
    pub(crate) fn label(&self, available: bool) -> String {
        let Some(target) = self.target else {
            return format!("{}: unresolved", self.reference.label);
        };
        let mut address = target.id.address.clone();
        if let Some(state) = &target.state {
            address = format!("{}:{address}", state.as_str());
        }
        if let Some(key) = &target.id.deposed_key {
            address.push_str(&format!(" (deposed: {key})"));
        }
        format!(
            "{}{}: {address}",
            if available { "" } else { "Unavailable — " },
            self.reference.label
        )
    }

    pub(crate) fn line_count(&self, width: usize) -> usize {
        super::text::wrap(&self.label(false), width / 8).len()
    }
}

pub(crate) fn width(node: &Node) -> usize {
    rows(node)
        .iter()
        .map(|row| {
            row.label(false)
                .chars()
                .map(|c| if c.is_ascii() { 8 } else { 16 })
                .sum::<usize>()
                + TEXT_INSET
        })
        .max()
        .unwrap_or(crate::layout::NODE_WIDTH)
        .clamp(crate::layout::NODE_WIDTH, 640)
}

pub(crate) fn text_width(node: &Node) -> usize {
    width(node) - TEXT_INSET
}

fn verified(reference: &ResourceReference) -> bool {
    reference.complete
        && !reference
            .issues
            .contains(&crate::model::DiagnosticReason::DynamicInstanceSelection)
}

pub(crate) fn rows(node: &Node) -> Vec<Row<'_>> {
    let mut rows = Vec::new();
    for reference in &node.references {
        if !verified(reference) {
            if node.placement == Placement::Logical
                || reference
                    .targets
                    .iter()
                    .any(|t| t.placement == Placement::Logical)
                || !matches!(reference.label, "References" | "Dependency")
            {
                rows.push(Row {
                    reference,
                    target: None,
                });
            }
            continue;
        }
        for target in &reference.targets {
            if node.placement != Placement::Logical && target.placement != Placement::Logical {
                continue;
            }
            if node.references.iter().any(|other| {
                !other.dependency
                    && verified(other)
                    && other
                        .attribute
                        .strip_prefix(&reference.attribute)
                        .is_some_and(|suffix| suffix.starts_with('.'))
                    && other.targets.contains(target)
            }) {
                continue;
            }
            rows.push(Row {
                reference,
                target: Some(target),
            });
        }
    }
    rows.sort_by_key(|r| {
        (
            r.reference.dependency,
            r.reference.label,
            &r.reference.attribute,
            r.target,
        )
    });
    rows.dedup_by(|a, b| a.reference == b.reference && a.target == b.target);
    rows
}

pub(crate) fn height(node: &Node) -> usize {
    let rows = rows(node);
    if rows.is_empty() {
        0
    } else {
        let width = text_width(node);
        24 + LINE_HEIGHT * rows.iter().map(|row| row.line_count(width)).sum::<usize>()
    }
}
