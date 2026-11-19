use super::configured_links::{self, Evidence, Rule};
use crate::model::{AttributeReference, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;

fn rule(kind: &str, path: &str) -> Option<Rule> {
    Some(match (kind, path) {
        ("aws_accountaccess_application", "identity_source.identity_center.instance_arn") => {
            Rule::new(
                "Identity Center instance",
                &[("aws_ssoadmin_instances", &["arns"])],
                false,
                &[],
            )
        }
        ("aws_accountaccess_entitlement", "application_arn") => Rule::new(
            "Account Access application",
            &[("aws_accountaccess_application", &["arn"])],
            false,
            &[],
        ),
        ("aws_accountaccess_entitlement", "entitlement.principal_role.role_arn") => Rule::new(
            "Entitlement IAM role",
            &[("aws_iam_role", &["arn"])],
            false,
            &[],
        )
        .with_explicit_destination(),
        (
            "aws_accountaccess_entitlement",
            "entitlement.principal_role.principal.identity_center.user_id",
        ) => Rule::new(
            "Identity Center user",
            &[("aws_identitystore_user", &["user_id"])],
            false,
            &[],
        ),
        (
            "aws_accountaccess_entitlement",
            "entitlement.principal_role.principal.identity_center.group_id",
        ) => Rule::new(
            "Identity Center group",
            &[("aws_identitystore_group", &["group_id"])],
            false,
            &[],
        ),
        _ => return None,
    })
}
pub(crate) fn resource(kind: &str) -> bool {
    matches!(
        kind,
        "aws_accountaccess_application" | "aws_accountaccess_entitlement"
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
}
pub(super) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    values: &[(&Value, &Value)],
) {
    configured_links::annotate(nodes, refs, values, rule, qualifies)
}
fn qualifies(e: &Evidence<'_>, r: &AttributeReference, source: usize, path: &str) -> bool {
    if r.attribute == "entitlement.principal_role.role_arn" {
        let Some(arn) = e.text(source, "arn") else {
            return false;
        };
        let mut parts = arn.splitn(6, ':');
        let [
            Some("arn"),
            Some("aws"),
            Some("iam"),
            Some(""),
            Some(account),
            Some(resource),
        ] = std::array::from_fn(|_| parts.next())
        else {
            return false;
        };
        return resource.starts_with("role/")
            && resource.len() > 5
            && crate::model::AwsAccountScope::new(account).is_some()
            && e.nodes[source]
                .provider_configuration
                .as_ref()
                .and_then(|config| config.account.as_ref())
                .is_none_or(|actual| actual.as_str() == account);
    }
    if !r
        .attribute
        .starts_with("entitlement.principal_role.principal.identity_center.")
    {
        return true;
    }
    let Some((parent, field)) = path.rsplit_once('.') else {
        return false;
    };
    let other = if field == "user_id" {
        "group_id"
    } else {
        "user_id"
    };
    e.value(r.target, &format!("{parent}.{other}"))
        .is_some_and(Value::is_null)
        && principal_store(e, r.target, source)
}
fn principal_store(e: &Evidence<'_>, entitlement: usize, principal: usize) -> bool {
    let Some(app) = e.configured_endpoint(
        entitlement,
        "application_arn",
        "aws_accountaccess_application",
        &["arn"],
    ) else {
        return false;
    };
    if !super::identity_center::compatible(e.nodes, entitlement, app, false) {
        return false;
    }
    let path = "identity_source.0.identity_center.0.instance_arn";
    let Some(lookup) = super::lambda_core::unique(e.refs, app, path) else {
        return false;
    };
    let [endpoint] = lookup.sources.as_slice() else {
        return false;
    };
    lookup.indexed_block_lengths == [1, 1]
        && e.text(app, path).is_some_and(|value| {
            super::identity_center_instance::verify(e.nodes, lookup, e.values, value)
        })
        && super::identity_center::compatible(e.nodes, app, *endpoint, false)
        && super::identity_center::instance_bridge(
            e.nodes,
            e.refs,
            e.values,
            principal,
            "identity_store_id",
            "identity_store_ids",
        ) == Some(*endpoint)
}

#[cfg(test)]
#[path = "../../../tests/unit/semantic/accountaccess_configuration.rs"]
mod tests;
