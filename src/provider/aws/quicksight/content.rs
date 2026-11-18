use crate::model::{AttributeReference, TerraformEntity};
use serde_json::Value;
use std::collections::BTreeSet;

const ANALYSIS: &str = "aws_quicksight_analysis";
const DASHBOARD: &str = "aws_quicksight_dashboard";
const TEMPLATE: &str = "aws_quicksight_template";
const DATASET: &str = "aws_quicksight_data_set";
const THEME: &str = "aws_quicksight_theme";
const FOLDER: &str = "aws_quicksight_folder";
const MEMBERSHIP: &str = "aws_quicksight_folder_membership";
const ALIAS: &str = "aws_quicksight_template_alias";

pub(super) fn rule(kind: &str, path: &str) -> Option<super::Rule> {
    Some(match (kind, path) {
        (ANALYSIS | DASHBOARD, "theme_arn") => (THEME, &["arn"], "Theme"),
        (ANALYSIS | DASHBOARD | TEMPLATE, "source_entity.source_template.arn") => {
            (TEMPLATE, &["arn"], "Source template")
        }
        (
            ANALYSIS | DASHBOARD,
            "source_entity.source_template.data_set_references.data_set_arn"
            | "definition.data_set_identifiers_declarations.data_set_arn",
        )
        | (TEMPLATE, "source_entity.source_analysis.data_set_references.data_set_arn") => {
            (DATASET, &["arn"], "Source datasets")
        }
        (TEMPLATE, "source_entity.source_analysis.arn") => (ANALYSIS, &["arn"], "Source analysis"),
        (ALIAS, "template_id") => (TEMPLATE, &["template_id"], "Aliased template"),
        (
            DATASET,
            "physical_table_map.custom_sql.data_source_arn"
            | "physical_table_map.relational_table.data_source_arn"
            | "physical_table_map.s3_source.data_source_arn",
        ) => (
            "aws_quicksight_data_source",
            &["arn"],
            "Physical table data sources",
        ),
        (DATASET, "logical_table_map.source.data_set_arn") => (DATASET, &["arn"], "Parent dataset"),
        (DATASET, "row_level_permission_data_set.arn") => {
            (DATASET, &["arn"], "Row permission dataset")
        }
        (FOLDER, "parent_folder_arn") => (FOLDER, &["arn"], "Parent folder"),
        (MEMBERSHIP, "folder_id") => (FOLDER, &["folder_id"], "Folder"),
        (MEMBERSHIP, "member_id") => ("", &[], "Folder member"),
        ("aws_quicksight_ingestion" | "aws_quicksight_refresh_schedule", "data_set_id") => {
            (DATASET, &["data_set_id"], "Dataset")
        }
        (THEME, "base_theme_id") => (THEME, &["theme_id"], "Base theme"),
        _ => return None,
    })
}

pub(super) fn resource(kind: &str) -> bool {
    matches!(
        kind,
        ANALYSIS
            | DASHBOARD
            | TEMPLATE
            | ALIAS
            | DATASET
            | FOLDER
            | MEMBERSHIP
            | THEME
            | "aws_quicksight_ingestion"
            | "aws_quicksight_refresh_schedule"
    )
}

pub(super) fn member_output(endpoint: &str) -> &'static [&'static str] {
    match endpoint {
        ANALYSIS => &["analysis_id"],
        DASHBOARD => &["dashboard_id"],
        DATASET => &["data_set_id"],
        _ => &[],
    }
}

pub(super) const COLLECTIONS: &[&str] = &[
    "source_entity.source_template.data_set_references",
    "definition.data_set_identifiers_declarations",
    "source_entity.source_analysis.data_set_references",
    "physical_table_map",
    "logical_table_map",
];

fn absent(value: &Value, mask: &Value, path: &str) -> bool {
    super::super::nested_references::value(value, mask, path)
        .is_some_and(|v| v.is_null() || v.as_array().is_some_and(Vec::is_empty))
}

fn rooted_folder(refs: &[AttributeReference], target: usize) -> bool {
    let mut seen = BTreeSet::new();
    let mut next = Some(target);
    while let Some(node) = next {
        if !seen.insert(node) {
            return false;
        }
        let mut parents = refs
            .iter()
            .filter(|r| r.target == node && r.attribute == "parent_folder_arn" && r.has_references);
        next = match parents.next() {
            None => None,
            Some(r) => match r.sources.as_slice() {
                [only] if parents.next().is_none() => Some(*only),
                _ => return false,
            },
        };
    }
    true
}

fn split_version(arn: &str) -> Option<(&str, u64)> {
    let (base, version) = arn.rsplit_once("/version/")?;
    let version = version.parse().ok().filter(|n| *n > 0)?;
    Some((base, version))
}

pub(super) fn decode<'a>(path: &str, actual: &'a Value) -> Option<BTreeSet<&'a str>> {
    let value = actual.as_str()?;
    let id = if path == "source_entity.source_template.arn" {
        split_version(value).map_or(value, |v| v.0)
    } else {
        value
    };
    Some([id].into())
}

pub(super) fn qualifies(
    nodes: &[TerraformEntity],
    refs: &[AttributeReference],
    evidence: &[(&Value, &Value)],
    r: &AttributeReference,
    source: usize,
    path: &str,
) -> bool {
    let shape: Vec<_> = path.split('.').collect();
    if shape.len() != 2 * r.attribute.split('.').count() - 1
        || !shape
            .iter()
            .skip(1)
            .step_by(2)
            .all(|i| i.parse::<usize>().is_ok())
    {
        return false;
    }
    let kind = nodes[r.target].resource_type.as_str();
    let (v, m) = evidence[r.target];
    let (ev, em) = evidence[source];
    let val = super::super::nested_references::value;
    if (path.starts_with("definition.") && !absent(v, m, "source_entity"))
        || (path.starts_with("source_entity.") && !absent(v, m, "definition"))
    {
        return false;
    }
    if path.starts_with("source_entity.0.source_template.")
        && !absent(v, m, "source_entity.0.source_analysis")
        || path.starts_with("source_entity.0.source_analysis.")
            && !absent(v, m, "source_entity.0.source_template")
    {
        return false;
    }
    if kind == MEMBERSHIP && path == "member_id" {
        return matches!(
            (
                val(v, m, "member_type").and_then(Value::as_str),
                nodes[source].resource_type.as_str()
            ),
            (Some("ANALYSIS"), ANALYSIS)
                | (Some("DASHBOARD"), DASHBOARD)
                | (Some("DATASET"), DATASET)
        );
    }
    if kind == ALIAS {
        return val(v, m, "template_version_number")
            .and_then(Value::as_u64)
            .is_some_and(|n| {
                n > 0 && val(ev, em, "version_number").and_then(Value::as_u64) == Some(n)
            });
    }
    if r.attribute == "source_entity.source_template.arn" {
        if let Some((_, version)) = val(v, m, path)
            .and_then(Value::as_str)
            .and_then(split_version)
        {
            return val(ev, em, "version_number").and_then(Value::as_u64) == Some(version);
        }
    }
    if kind == FOLDER {
        return rooted_folder(refs, r.target);
    }
    if path.starts_with("physical_table_map.") {
        let mut parts = path.split('.');
        let row = format!("physical_table_map.{}", parts.nth(1).unwrap_or(""));
        let branch = parts.next().unwrap_or("");
        return ["custom_sql", "relational_table", "s3_source"]
            .into_iter()
            .filter(|b| *b != branch)
            .all(|b| absent(v, m, &format!("{row}.{b}")));
    }
    if let Some(row) = path
        .strip_suffix(".data_set_arn")
        .filter(|s| s.ends_with(".source.0"))
    {
        return absent(v, m, &format!("{row}.join_instruction"))
            && absent(v, m, &format!("{row}.physical_table_id"));
    }
    true
}
