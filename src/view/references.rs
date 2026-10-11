use crate::model::{Node, Placement, ReferenceTarget, ResourceReference};

pub(crate) const VISIBLE_LIMIT: usize = 4;

pub(crate) struct Row<'a> {
    pub reference: &'a ResourceReference,
    pub target: Option<&'a ReferenceTarget>,
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
    let count = rows(node).len();
    if count == 0 {
        0
    } else {
        24 + 18 * (count.min(VISIBLE_LIMIT) + usize::from(count > VISIBLE_LIMIT))
    }
}
