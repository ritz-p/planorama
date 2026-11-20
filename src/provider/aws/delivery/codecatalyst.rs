use super::{Evidence, Rule};
use crate::model::AttributeReference;

pub(super) fn resource(kind: &str) -> bool {
    matches!(
        kind,
        "aws_codecatalyst_dev_environment" | "aws_codecatalyst_source_repository"
    )
}

pub(super) fn rule(kind: &str, path: &str) -> Option<Rule> {
    Some(match (kind, path) {
        (
            "aws_codecatalyst_dev_environment" | "aws_codecatalyst_source_repository",
            "project_name",
        ) => Rule::new(
            "CodeCatalyst project",
            &[("aws_codecatalyst_project", &["name"])],
            false,
            &[],
        ),
        ("aws_codecatalyst_dev_environment", "repositories.repository_name") => Rule::new(
            "CodeCatalyst repository",
            &[("aws_codecatalyst_source_repository", &["name"])],
            false,
            &["repositories"],
        ),
        _ => return None,
    })
}

pub(super) fn qualifies(e: &Evidence<'_>, r: &AttributeReference, source: usize, _: &str) -> bool {
    let consumer = r.target;
    let matches = |field| {
        e.text(consumer, field)
            .zip(e.text(source, field))
            .is_some_and(|(left, right)| left == right)
    };
    matches("space_name")
        && (r.attribute != "repositories.repository_name" || matches("project_name"))
}
