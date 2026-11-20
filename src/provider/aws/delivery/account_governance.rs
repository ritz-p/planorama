use super::{Evidence, Rule};
use crate::model::AttributeReference;

pub(super) fn resource(kind: &str) -> bool {
    matches!(
        kind,
        "aws_account_alternate_contact"
            | "aws_account_primary_contact"
            | "aws_account_region"
            | "aws_cloudtrail_organization_delegated_admin_account"
            | "aws_organizations_delegated_administrator"
            | "aws_organizations_tag"
    )
}

pub(super) fn rule(kind: &str, path: &str) -> Option<Rule> {
    Some(match (kind, path) {
        (
            "aws_account_alternate_contact"
            | "aws_account_primary_contact"
            | "aws_account_region"
            | "aws_cloudtrail_organization_delegated_admin_account"
            | "aws_organizations_delegated_administrator",
            "account_id",
        ) => Rule::new(
            "Target organization account",
            &[("aws_organizations_account", &["id"])],
            false,
            &[],
        )
        .with_explicit_destination(),
        ("aws_organizations_tag", "resource_id") => Rule::new(
            "Tagged Organizations resource",
            &[
                ("aws_organizations_account", &["id"]),
                ("aws_organizations_organizational_unit", &["id"]),
                ("aws_organizations_organization", &["id", "roots[].id"]),
                ("aws_organizations_policy", &["id"]),
            ],
            false,
            &[],
        )
        .with_explicit_destination(),
        _ => return None,
    })
}

pub(super) fn qualifies(
    e: &Evidence<'_>,
    r: &AttributeReference,
    source: usize,
    path: &str,
) -> bool {
    use crate::model::AwsAccountScope;
    let value = e.text(r.target, path);
    if r.attribute == "account_id" {
        return value.and_then(AwsAccountScope::new).is_some()
            && (e.nodes[r.target].resource_type != "aws_account_region"
                || e.text(r.target, "region_name")
                    .and_then(crate::provider::aws::scope_selections::region)
                    .is_some());
    }
    value.is_some_and(|id| match e.nodes[source].resource_type.as_str() {
        "aws_organizations_account" => AwsAccountScope::new(id).is_some(),
        "aws_organizations_organizational_unit" => id.starts_with("ou-"),
        "aws_organizations_organization" => id.starts_with("o-") || id.starts_with("r-"),
        "aws_organizations_policy" => id.starts_with("p-"),
        _ => false,
    })
}
