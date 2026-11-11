use crate::model::{AttributeReference, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/glue_catalog.rs"]
mod tests;

pub(crate) fn resource(kind: &str) -> bool {
    matches!(
        kind,
        "aws_glue_catalog"
            | "aws_glue_catalog_database"
            | "aws_glue_catalog_table"
            | "aws_glue_catalog_table_optimizer"
            | "aws_glue_data_quality_ruleset"
            | "aws_glue_partition"
            | "aws_glue_partition_index"
            | "aws_glue_schema"
            | "aws_glue_user_defined_function"
    )
}

fn rule(kind: &str, path: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match (kind, path) {
        ("aws_glue_catalog", "federated_catalog.connection_name")
        | ("aws_glue_catalog_database", "federated_database.connection_name")
        | ("aws_glue_catalog_table", "view_definition.representations.validation_connection") => {
            Some(("aws_glue_connection", "name", "Catalog connection"))
        }
        ("aws_glue_catalog", "target_redshift_catalog.catalog_arn") => {
            Some(("aws_glue_catalog", "arn", "Target catalog"))
        }
        (
            "aws_glue_catalog",
            "catalog_properties.data_lake_access_properties.data_transfer_role"
            | "catalog_properties.iceberg_optimization_properties.role_arn",
        )
        | ("aws_glue_catalog_table_optimizer", "configuration.role_arn") => {
            Some(("aws_iam_role", "arn", "Catalog execution role"))
        }
        ("aws_glue_catalog", "catalog_properties.data_lake_access_properties.kms_key") => {
            Some(("aws_kms_key", "arn", "Catalog encryption key"))
        }
        ("aws_glue_catalog_database", "target_database.database_name") => {
            Some(("aws_glue_catalog_database", "name", "Target database"))
        }
        ("aws_glue_catalog_table", "target_table.name") => {
            Some(("aws_glue_catalog_table", "name", "Target table"))
        }
        ("aws_glue_catalog_table", "storage_descriptor.schema_reference.schema_id.schema_arn") => {
            Some(("aws_glue_schema", "arn", "Referenced schema"))
        }
        ("aws_glue_catalog_table", "storage_descriptor.schema_reference.schema_id.schema_name") => {
            Some(("aws_glue_schema", "schema_name", "Referenced schema"))
        }
        (
            "aws_glue_catalog_table",
            "storage_descriptor.schema_reference.schema_id.registry_name",
        ) => Some(("aws_glue_registry", "registry_name", "Referenced registry")),
        ("aws_glue_catalog_table", "view_definition.sub_objects") => {
            Some(("aws_glue_catalog_table", "arn", "View base tables"))
        }
        (
            "aws_glue_catalog_table_optimizer" | "aws_glue_partition" | "aws_glue_partition_index",
            "table_name",
        ) => Some(("aws_glue_catalog_table", "name", "Catalog table")),
        ("aws_glue_data_quality_ruleset", "target_table.table_name") => {
            Some(("aws_glue_catalog_table", "name", "Quality target table"))
        }
        ("aws_glue_schema", "registry_arn") => {
            Some(("aws_glue_registry", "arn", "Schema registry"))
        }
        ("aws_glue_user_defined_function", "database_name") => {
            Some(("aws_glue_catalog_database", "name", "Function database"))
        }
        _ => None,
    }
}

pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    rule(kind, path).map(|r| r.2)
}

pub(crate) fn indexed(kind: &str, path: &str) -> bool {
    let normal = path
        .split('.')
        .filter(|s| s.parse::<usize>().is_err())
        .collect::<Vec<_>>()
        .join(".");
    normal != path && label(kind, &normal).is_some()
}

fn fields(kind: &str, path: &str, source: &str) -> &'static [&'static str] {
    let Some((expected, output, _)) = rule(kind, path) else {
        return &[];
    };
    if expected == "aws_kms_key" {
        return if super::kms::key_outputs(source).contains(&"arn") {
            &["arn"]
        } else {
            &[]
        };
    }
    if expected != source {
        return &[];
    }
    match output {
        "arn" => &["arn"],
        "name" => &["name"],
        "schema_name" => &["schema_name"],
        "registry_name" => &["registry_name"],
        _ => &[],
    }
}

fn shape(value: &Value, path: &[&str]) -> bool {
    let Some((name, rest)) = path.split_first() else {
        return true;
    };
    let Some(value) = value.get(*name) else {
        return false;
    };
    if rest.is_empty() {
        return if *name == "sub_objects" {
            value.is_array()
        } else {
            value.is_string()
        };
    }
    value.as_array().is_some_and(|a| {
        !a.is_empty()
            && (*name == "representations" || a.len() == 1)
            && a.iter().all(|v| shape(v, rest))
    })
}

fn value<'a>(evidence: &[(&'a Value, &Value)], target: usize, path: &str) -> Option<&'a Value> {
    super::nested_references::value(evidence[target].0, evidence[target].1, path)
}
fn string<'a>(evidence: &[(&'a Value, &Value)], target: usize, path: &str) -> Option<&'a str> {
    value(evidence, target, path)?
        .as_str()
        .filter(|s| !s.is_empty())
}
fn equal(
    evidence: &[(&Value, &Value)],
    target: usize,
    path: &str,
    source: usize,
    output: &str,
) -> bool {
    string(evidence, target, path).is_some_and(|v| string(evidence, source, output) == Some(v))
}
fn null(evidence: &[(&Value, &Value)], target: usize, path: &str) -> bool {
    value(evidence, target, path).is_some_and(Value::is_null)
}

fn compatible(nodes: &[TerraformEntity], target: usize, source: usize, global: bool) -> bool {
    let left = &nodes[target];
    let right = &nodes[source];
    let l = super::scope::scope(left);
    let r = super::scope::scope(right);
    (global
        || (l.as_ref().and_then(|s| s.region.as_ref()).is_some()
            && r.as_ref().and_then(|s| s.region.as_ref()).is_some()))
        && super::eventbridge::bindings(
            left.provider_configuration.as_ref(),
            right.provider_configuration.as_ref(),
            l.as_ref().and_then(|s| s.region.as_ref()),
            r.as_ref().and_then(|s| s.region.as_ref()),
            global,
        )
}

fn qualified(
    nodes: &[TerraformEntity],
    evidence: &[(&Value, &Value)],
    r: &AttributeReference,
    source: usize,
    indexed: &str,
) -> bool {
    let target = r.target;
    let Some((kind, output, _)) = rule(&nodes[target].resource_type, &r.attribute) else {
        return false;
    };
    if !compatible(nodes, target, source, kind == "aws_iam_role") {
        return false;
    }
    if nodes[target].resource_type == "aws_glue_catalog_table_optimizer" {
        if !matches!(
            string(evidence, target, "type"),
            Some("compaction" | "retention" | "orphan_file_deletion")
        ) {
            return false;
        }
        if kind == "aws_iam_role"
            && value(evidence, target, "configuration.0.enabled") != Some(&Value::Bool(true))
        {
            return false;
        }
    }
    if kind == "aws_glue_connection" {
        return equal(evidence, target, "catalog_id", source, "catalog_id");
    }
    if output == "name" && matches!(kind, "aws_glue_catalog_database" | "aws_glue_catalog_table") {
        let prefix = indexed.rsplit_once('.').map_or("", |(p, _)| p);
        let sibling = |name: &str| {
            if prefix.is_empty() {
                name.to_string()
            } else {
                format!("{prefix}.{name}")
            }
        };
        if !equal(
            evidence,
            target,
            &sibling("catalog_id"),
            source,
            "catalog_id",
        ) {
            return false;
        }
        if kind == "aws_glue_catalog_table"
            && !equal(
                evidence,
                target,
                &sibling("database_name"),
                source,
                "database_name",
            )
        {
            return false;
        }
        if !prefix.is_empty() {
            let region = value(evidence, target, &sibling("region"));
            if region.is_none()
                || region.is_some_and(|v| {
                    !v.is_null()
                        && v.as_str()
                            != super::scope::scope(&nodes[source])
                                .as_ref()
                                .and_then(|s| s.region.as_ref())
                                .map(|r| r.0.as_str())
                })
            {
                return false;
            }
        }
    }
    if r.attribute
        .starts_with("storage_descriptor.schema_reference.schema_id.")
    {
        let base = "storage_descriptor.0.schema_reference.0";
        if !null(evidence, target, &format!("{base}.schema_version_id")) {
            return false;
        }
        if output == "arn" {
            if !null(
                evidence,
                target,
                &format!("{base}.schema_id.0.registry_name"),
            ) || !null(evidence, target, &format!("{base}.schema_id.0.schema_name"))
            {
                return false;
            }
        } else {
            if !null(evidence, target, &format!("{base}.schema_id.0.schema_arn")) {
                return false;
            }
            if string(evidence, target, &format!("{base}.schema_id.0.schema_name")).is_none() {
                return false;
            }
        }
        if kind == "aws_glue_schema" {
            if output == "schema_name"
                && !equal(
                    evidence,
                    target,
                    &format!("{base}.schema_id.0.registry_name"),
                    source,
                    "registry_name",
                )
            {
                return false;
            }
            let version = value(evidence, target, &format!("{base}.schema_version_number"))
                .and_then(Value::as_u64);
            if version.is_none_or(|v| v == 0)
                || version
                    != value(evidence, source, "latest_schema_version").and_then(Value::as_u64)
            {
                return false;
            }
        }
    }
    true
}

pub(super) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    evidence: &[(&Value, &Value)],
) {
    let results: Vec<_> = refs
        .iter()
        .map(|r| {
            label(&nodes[r.target].resource_type, &r.attribute)?;
            Some(
                shape(
                    evidence[r.target].0,
                    &r.attribute.split('.').collect::<Vec<_>>(),
                ) && super::nested_references::verify(
                    nodes,
                    refs,
                    evidence,
                    r,
                    fields,
                    |source, path| qualified(nodes, evidence, r, source, path),
                )
                .is_some(),
            )
        })
        .collect();
    for (r, result) in refs.iter_mut().zip(results) {
        if let Some(valid) = result {
            r.service_reference_verified = valid;
            if valid {
                r.complete = true;
            }
        }
    }
}

pub(super) fn bindings(raw: &Input, r: &AttributeReference) -> bool {
    let Some((kind, _, _)) = rule(&raw.nodes[r.target].resource_type, &r.attribute) else {
        return true;
    };
    r.sources.iter().all(|&s| {
        let left = &raw.nodes[r.target];
        let right = &raw.nodes[s];
        super::eventbridge::bindings(
            left.provider_configuration.as_ref(),
            right.provider_configuration.as_ref(),
            left.entity.scope.as_ref().and_then(|s| s.region.as_ref()),
            right.entity.scope.as_ref().and_then(|s| s.region.as_ref()),
            kind == "aws_iam_role",
        )
    })
}

pub(super) fn owner(kind: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match kind {
        "aws_glue_catalog_table_optimizer" => {
            Some(("table_name", "Catalog table", "Table optimizers"))
        }
        "aws_glue_partition" | "aws_glue_partition_index" => {
            Some(("table_name", "Catalog table", "Partitions"))
        }
        "aws_glue_user_defined_function" => {
            Some(("database_name", "Function database", "Functions"))
        }
        "aws_glue_schema" => Some(("registry_arn", "Schema registry", "Schemas")),
        _ => None,
    }
}
