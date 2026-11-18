use super::configured_links::{self, Evidence, Rule};
use crate::model::{AttributeReference, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;

pub(crate) fn resource(kind: &str) -> bool {
    matches!(
        kind,
        "aws_lakeformation_data_cells_filter"
            | "aws_lakeformation_identity_center_configuration"
            | "aws_lakeformation_data_lake_settings"
            | "aws_lakeformation_lf_tag_expression"
            | "aws_lakeformation_resource"
            | "aws_lakeformation_permissions"
            | "aws_lakeformation_opt_in"
            | "aws_lakeformation_resource_lf_tag"
            | "aws_lakeformation_resource_lf_tags"
    )
}
fn rule(kind: &str, path: &str) -> Option<Rule> {
    Some(match (kind, path) {
        ("aws_lakeformation_data_cells_filter", "table_data.table_name") => Rule::new(
            "Filtered table",
            &[("aws_glue_catalog_table", &["name"])],
            false,
            &["table_data"],
        ),
        ("aws_lakeformation_data_cells_filter", "table_data.database_name") => Rule::new(
            "Filter database",
            &[("aws_glue_catalog_database", &["name"])],
            false,
            &["table_data"],
        ),
        ("aws_lakeformation_identity_center_configuration", "instance_arn") => Rule::new(
            "Identity Center instance",
            &[("aws_ssoadmin_instances", &["arns"])],
            false,
            &[],
        ),
        ("aws_lakeformation_data_lake_settings", "admins" | "read_only_admins") => Rule::new(
            "Configured lake administrators",
            &[("aws_iam_role", &["arn"]), ("aws_iam_user", &["arn"])],
            true,
            &[],
        ),
        ("aws_lakeformation_lf_tag_expression", "expression.tag_key") => Rule::new(
            "Catalog tag",
            &[("aws_lakeformation_lf_tag", &["key"])],
            false,
            &["expression"],
        ),
        ("aws_lakeformation_resource", "arn") => Rule::new(
            "Registered resource",
            &[
                ("aws_s3_bucket", &["arn"]),
                ("aws_glue_connection", &["arn"]),
            ],
            false,
            &[],
        ),
        ("aws_lakeformation_resource", "role_arn") => Rule::new(
            "Registration role",
            &[("aws_iam_role", &["arn"])],
            false,
            &[],
        ),
        ("aws_lakeformation_permissions", "principal") => Rule::new(
            "Configured principal",
            &[("aws_iam_role", &["arn"]), ("aws_iam_user", &["arn"])],
            false,
            &[],
        ),
        ("aws_lakeformation_permissions", "data_location.arn") => Rule::new(
            "Registered location",
            &[("aws_lakeformation_resource", &["arn"])],
            false,
            &[],
        ),
        ("aws_lakeformation_permissions", "database.name") => Rule::new(
            "Catalog database",
            &[("aws_glue_catalog_database", &["name"])],
            false,
            &[],
        ),
        ("aws_lakeformation_permissions", "table.name" | "table_with_columns.name") => Rule::new(
            "Catalog table",
            &[("aws_glue_catalog_table", &["name"])],
            false,
            &[],
        ),
        (
            "aws_lakeformation_permissions",
            "table.database_name" | "table_with_columns.database_name",
        ) => Rule::new(
            "Table database",
            &[("aws_glue_catalog_database", &["name"])],
            false,
            &[],
        ),
        ("aws_lakeformation_permissions", "lf_tag.key") => Rule::new(
            "Catalog tag",
            &[("aws_lakeformation_lf_tag", &["key"])],
            false,
            &[],
        ),
        ("aws_lakeformation_opt_in", "principal.data_lake_principal_identifier") => Rule::new(
            "Configured principal",
            &[("aws_iam_role", &["arn"]), ("aws_iam_user", &["arn"])],
            false,
            &["principal"],
        ),
        ("aws_lakeformation_opt_in", "resource_data.data_location.resource_arn") => Rule::new(
            "Registered location",
            &[("aws_lakeformation_resource", &["arn"])],
            false,
            &["resource_data", "resource_data.data_location"],
        ),
        ("aws_lakeformation_opt_in", "resource_data.lf_tag_expression.name") => Rule::new(
            "Catalog tag expression",
            &[("aws_lakeformation_lf_tag_expression", &["name"])],
            false,
            &["resource_data", "resource_data.lf_tag_expression"],
        ),
        ("aws_lakeformation_opt_in", "resource_data.database.name") => Rule::new(
            "Catalog database",
            &[("aws_glue_catalog_database", &["name"])],
            false,
            &["resource_data", "resource_data.database"],
        ),
        ("aws_lakeformation_opt_in", "resource_data.table.name") => Rule::new(
            "Catalog table",
            &[("aws_glue_catalog_table", &["name"])],
            false,
            &["resource_data", "resource_data.table"],
        ),
        ("aws_lakeformation_opt_in", "resource_data.table.database_name") => Rule::new(
            "Table database",
            &[("aws_glue_catalog_database", &["name"])],
            false,
            &["resource_data", "resource_data.table"],
        ),
        ("aws_lakeformation_opt_in", "resource_data.table_with_columns.name") => Rule::new(
            "Catalog table",
            &[("aws_glue_catalog_table", &["name"])],
            false,
            &["resource_data", "resource_data.table_with_columns"],
        ),
        ("aws_lakeformation_opt_in", "resource_data.table_with_columns.database_name") => {
            Rule::new(
                "Table database",
                &[("aws_glue_catalog_database", &["name"])],
                false,
                &["resource_data", "resource_data.table_with_columns"],
            )
        }
        ("aws_lakeformation_opt_in", "resource_data.lf_tag.key") => Rule::new(
            "Catalog tag",
            &[("aws_lakeformation_lf_tag", &["key"])],
            false,
            &["resource_data", "resource_data.lf_tag"],
        ),
        ("aws_lakeformation_resource_lf_tag", "database.name") => Rule::new(
            "Catalog database",
            &[("aws_glue_catalog_database", &["name"])],
            false,
            &["database"],
        ),
        ("aws_lakeformation_resource_lf_tag", "table.name") => Rule::new(
            "Catalog table",
            &[("aws_glue_catalog_table", &["name"])],
            false,
            &["table"],
        ),
        ("aws_lakeformation_resource_lf_tag", "table.database_name") => Rule::new(
            "Table database",
            &[("aws_glue_catalog_database", &["name"])],
            false,
            &["table"],
        ),
        ("aws_lakeformation_resource_lf_tag", "table_with_columns.name") => Rule::new(
            "Catalog table",
            &[("aws_glue_catalog_table", &["name"])],
            false,
            &["table_with_columns"],
        ),
        ("aws_lakeformation_resource_lf_tag", "table_with_columns.database_name") => Rule::new(
            "Table database",
            &[("aws_glue_catalog_database", &["name"])],
            false,
            &["table_with_columns"],
        ),
        ("aws_lakeformation_resource_lf_tag", "lf_tag.key") => Rule::new(
            "Catalog tag",
            &[("aws_lakeformation_lf_tag", &["key"])],
            false,
            &["lf_tag"],
        ),
        ("aws_lakeformation_resource_lf_tags", "database.name") => Rule::new(
            "Catalog database",
            &[("aws_glue_catalog_database", &["name"])],
            false,
            &[],
        ),
        ("aws_lakeformation_resource_lf_tags", "table.name" | "table_with_columns.name") => {
            Rule::new(
                "Catalog table",
                &[("aws_glue_catalog_table", &["name"])],
                false,
                &[],
            )
        }
        (
            "aws_lakeformation_resource_lf_tags",
            "table.database_name" | "table_with_columns.database_name",
        ) => Rule::new(
            "Table database",
            &[("aws_glue_catalog_database", &["name"])],
            false,
            &[],
        ),
        ("aws_lakeformation_resource_lf_tags", "lf_tag.key") => Rule::new(
            "Catalog tag",
            &[("aws_lakeformation_lf_tag", &["key"])],
            false,
            &["lf_tag"],
        ),
        ("aws_lakeformation_permissions", "data_cells_filter.database_name") => Rule::new(
            "Filter database",
            &[("aws_glue_catalog_database", &["name"])],
            false,
            &[],
        ),
        ("aws_lakeformation_permissions", "data_cells_filter.table_name") => Rule::new(
            "Filtered table",
            &[("aws_glue_catalog_table", &["name"])],
            false,
            &[],
        ),
        ("aws_lakeformation_opt_in", "resource_data.data_cells_filter.database_name") => Rule::new(
            "Filter database",
            &[("aws_glue_catalog_database", &["name"])],
            false,
            &["resource_data", "resource_data.data_cells_filter"],
        ),
        ("aws_lakeformation_opt_in", "resource_data.data_cells_filter.table_name") => Rule::new(
            "Filtered table",
            &[("aws_glue_catalog_table", &["name"])],
            false,
            &["resource_data", "resource_data.data_cells_filter"],
        ),
        ("aws_lakeformation_permissions", "lf_tag_policy.expression.key") => Rule::new(
            "Policy tag key",
            &[("aws_lakeformation_lf_tag", &["key"])],
            false,
            &["lf_tag_policy.expression"],
        ),
        ("aws_lakeformation_opt_in", "resource_data.lf_tag_policy.expression_name") => Rule::new(
            "Named tag policy expression",
            &[("aws_lakeformation_lf_tag_expression", &["name"])],
            false,
            &["resource_data", "resource_data.lf_tag_policy"],
        ),
        _ => return None,
    })
}
pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    rule(kind, path).map(|r| r.label)
}
pub(crate) fn indexed(kind: &str, path: &str) -> bool {
    resource(kind) && configured_links::indexed(kind, path, rule)
}
pub(super) fn bindings(raw: &Input, r: &AttributeReference) -> bool {
    configured_links::bindings(raw, r, rule)
}
pub(super) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    values: &[(&Value, &Value)],
) {
    configured_links::annotate(nodes, refs, values, rule, qualifies)
}
fn qualifies(e: &Evidence<'_>, r: &AttributeReference, s: usize, path: &str) -> bool {
    match (
        e.nodes[r.target].resource_type.as_str(),
        r.attribute.as_str(),
    ) {
        ("aws_lakeformation_resource", "arn") => {
            let fed = e.value(r.target, "with_federation");
            match e.nodes[s].resource_type.as_str() {
                "aws_glue_connection" => fed.and_then(Value::as_bool) == Some(true),
                "aws_s3_bucket" => fed.is_some_and(|v| v.is_null() || v.as_bool() == Some(false)),
                _ => false,
            }
        }
        ("aws_lakeformation_resource", "role_arn") => e
            .value(r.target, "use_service_linked_role")
            .is_some_and(|v| v.is_null() || v.as_bool() == Some(false)),
        (_, p)
            if matches!(
                p,
                "instance_arn"
                    | "admins"
                    | "read_only_admins"
                    | "principal"
                    | "principal.data_lake_principal_identifier"
            ) =>
        {
            true
        }
        (_, _) => qualified(e, r, s, path),
    }
}
fn qualified(e: &Evidence<'_>, r: &AttributeReference, s: usize, path: &str) -> bool {
    let Some((row, last)) = path.rsplit_once('.') else {
        return false;
    };
    let kind = e.nodes[s].resource_type.as_str();
    let catalog_key = if row == "table_data.0" || row.ends_with("data_cells_filter.0") {
        "table_catalog_id"
    } else {
        "catalog_id"
    };
    let scope = |i: usize, path: &str| -> Option<String> {
        let v = e.value(i, path)?;
        if v.is_null() {
            let c = e.nodes[i].provider_configuration.as_ref()?;
            return (!c.account_conflict)
                .then(|| c.account.as_ref().map(|a| a.as_str().to_owned()))
                .flatten();
        }
        v.as_str()
            .filter(|x| x.len() == 12 && x.bytes().all(|c| c.is_ascii_digit()))
            .map(str::to_owned)
    };
    let owner = if row.starts_with("expression.") {
        "catalog_id".to_owned()
    } else if row.starts_with("lf_tag_policy.0.expression.") {
        "lf_tag_policy.0.catalog_id".to_owned()
    } else {
        format!("{row}.{catalog_key}")
    };
    let selector_kind = e.nodes[r.target].resource_type.as_str();
    if matches!(
        selector_kind,
        "aws_lakeformation_permissions"
            | "aws_lakeformation_opt_in"
            | "aws_lakeformation_resource_lf_tag"
            | "aws_lakeformation_resource_lf_tags"
    ) {
        let prefix = if selector_kind == "aws_lakeformation_opt_in" {
            "resource_data.0."
        } else {
            ""
        };
        let association = selector_kind.ends_with("_lf_tag") || selector_kind.ends_with("_lf_tags");
        let first = row
            .strip_prefix(prefix)
            .unwrap_or(row)
            .split('.')
            .next()
            .unwrap_or("");
        for sibling in [
            "database",
            "table",
            "table_with_columns",
            "data_cells_filter",
            "data_location",
            "lf_tag",
            "lf_tag_expression",
            "lf_tag_policy",
            "catalog",
        ] {
            if sibling == first || (association && (sibling == "lf_tag" || first == "lf_tag")) {
                continue;
            }
            if !e
                .value(r.target, &format!("{prefix}{sibling}"))
                .is_some_and(|v| v.is_null() || v.as_array().is_some_and(Vec::is_empty))
            {
                return false;
            }
        }
        if e.value(r.target, "catalog_resource")
            .is_none_or(|v| v.as_bool() == Some(true))
        {
            return false;
        }
    }
    if !scope(r.target, &owner)
        .zip(scope(s, "catalog_id"))
        .is_some_and(|(a, b)| a == b)
    {
        return false;
    }
    if row.contains(".table.") || row.starts_with("table.") || row.contains("table_with_columns.") {
        if !e
            .value(r.target, &format!("{row}.wildcard"))
            .is_some_and(|v| v.is_null() || v.as_bool() == Some(false))
        {
            return false;
        }
    }
    if kind == "aws_glue_catalog_table" && matches!(last, "name" | "table_name") {
        let Some(db) = e.text(s, "database_name") else {
            return false;
        };
        return e.text(r.target, &format!("{row}.database_name")) == Some(db);
    }
    true
}
#[cfg(test)]
#[path = "../../../tests/unit/semantic/lakeformation_selectors.rs"]
mod tests;
