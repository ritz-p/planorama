use super::configured_links::{self, Evidence, Rule};
use crate::model::{AttributeReference, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;

fn rule(kind: &str, path: &str) -> Option<Rule> {
    Some(match (kind, path) {
        ("aws_cleanrooms_configured_table", "table_reference.database_name") => Rule::new(
            "Configured Glue database",
            &[("aws_glue_catalog_database", &["name"])],
            false,
            &[],
        ),
        ("aws_cleanrooms_configured_table", "table_reference.table_name") => Rule::new(
            "Configured Glue table",
            &[("aws_glue_catalog_table", &["name"])],
            false,
            &[],
        ),
        ("aws_cleanrooms_membership", "collaboration_id") => Rule::new(
            "Collaboration",
            &[("aws_cleanrooms_collaboration", &["id"])],
            false,
            &[],
        )
        .with_explicit_destination(),
        ("aws_cleanrooms_membership", "default_result_configuration.role_arn") => {
            Rule::new("Results role", &[("aws_iam_role", &["arn"])], false, &[])
        }
        (
            "aws_cleanrooms_membership",
            "default_result_configuration.output_configuration.s3.bucket",
        ) => Rule::new(
            "Results bucket",
            &[("aws_s3_bucket", &["bucket", "id"])],
            false,
            &[],
        ),
        _ => return None,
    })
}
pub(crate) fn resource(kind: &str) -> bool {
    matches!(
        kind,
        "aws_cleanrooms_configured_table" | "aws_cleanrooms_membership"
    )
}
pub(crate) fn indexed(kind: &str, path: &str) -> bool {
    resource(kind) && configured_links::indexed(kind, path, rule)
}
pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    rule(kind, path).map(|r| r.label)
}
pub(super) fn bindings(raw: &Input, r: &AttributeReference) -> bool {
    configured_links::bindings(raw, r, rule)
        && (raw.nodes[r.target].resource_type != "aws_cleanrooms_configured_table"
            || !catalog_reference(&r.attribute)
            || raw.nodes[r.target]
                .provider_configuration
                .as_ref()
                .and_then(|provider| provider.account.as_ref())
                .is_some_and(|account| Some(account) == r.service_reference_owner_account.as_ref()))
}

fn catalog_reference(attribute: &str) -> bool {
    matches!(
        attribute,
        "table_reference.database_name" | "table_reference.table_name"
    )
}
pub(super) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    values: &[(&Value, &Value)],
) {
    configured_links::annotate(nodes, refs, values, rule, qualifies);
    for reference in refs.iter_mut().filter(|r| {
        r.service_reference_verified
            && catalog_reference(&r.attribute)
            && nodes[r.target].resource_type == "aws_cleanrooms_configured_table"
    }) {
        let Some(&source) = reference.sources.first() else {
            continue;
        };
        reference.service_reference_owner_account =
            super::service_evidence::known(values, source, "catalog_id")
                .and_then(crate::model::AwsAccountScope::new);
    }
}
fn qualifies(e: &Evidence<'_>, r: &AttributeReference, source: usize, _path: &str) -> bool {
    if matches!(
        r.attribute.as_str(),
        "table_reference.database_name" | "table_reference.table_name"
    ) {
        let database = e.configured_endpoint(
            r.target,
            "table_reference.0.database_name",
            "aws_glue_catalog_database",
            &["name"],
        );
        let table = e.configured_endpoint(
            r.target,
            "table_reference.0.table_name",
            "aws_glue_catalog_table",
            &["name"],
        );
        let Some((database, table)) = database.zip(table) else {
            return false;
        };
        let catalog = e
            .text(database, "catalog_id")
            .and_then(crate::model::AwsAccountScope::new);
        return catalog
            .as_ref()
            .is_some_and(|catalog| e.text(table, "catalog_id") == Some(catalog.as_str()))
            && e.configured_endpoint(
                table,
                "database_name",
                "aws_glue_catalog_database",
                &["name"],
            ) == Some(database);
    }
    if r.attribute != "collaboration_id" {
        return true;
    }
    let Some(arn) = e.text(source, "arn") else {
        return false;
    };
    let mut arn = arn.splitn(6, ':');
    let [
        Some("arn"),
        Some("aws"),
        Some("cleanrooms"),
        Some(region),
        Some(account),
        Some(resource),
    ] = std::array::from_fn(|_| arn.next())
    else {
        return false;
    };
    let scope_matches = |i| {
        super::scope::scope(&e.nodes[i])
            .and_then(|s| s.region)
            .is_some_and(|r| r.0 == region)
    };
    scope_matches(r.target)
        && scope_matches(source)
        && crate::model::AwsAccountScope::new(account).is_some()
        && e.text(r.target, "collaboration_creator_account_id") == Some(account)
        && resource.strip_prefix("collaboration/") == e.text(source, "id")
        && e.nodes[source]
            .provider_configuration
            .as_ref()
            .and_then(|p| p.account.as_ref())
            .is_none_or(|scope| scope.as_str() == account)
}
#[cfg(test)]
#[path = "../../../tests/unit/semantic/cleanrooms_configuration.rs"]
mod tests;
