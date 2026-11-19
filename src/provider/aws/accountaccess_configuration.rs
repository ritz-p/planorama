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
    let path = "identity_source.0.identity_center.0.instance_arn";
    let Some(lookup) = super::lambda_core::unique(e.refs, app, path) else {
        return false;
    };
    let [endpoint] = lookup.sources.as_slice() else {
        return false;
    };
    if lookup.indexed_block_lengths != [1, 1]
        || e.text(app, path).is_none_or(|value| {
            !super::identity_center_instance::verify(e.nodes, lookup, e.values, value)
        })
    {
        return false;
    }
    let application = &e.nodes[app];
    let identity = &e.nodes[*endpoint];
    if !super::eventbridge::bindings(
        application.provider_configuration.as_ref(),
        identity.provider_configuration.as_ref(),
        super::scope::scope(application)
            .as_ref()
            .and_then(|s| s.region.as_ref()),
        super::scope::scope(identity)
            .as_ref()
            .and_then(|s| s.region.as_ref()),
        false,
    ) {
        return false;
    }
    let Some([store]) = e
        .value(*endpoint, "identity_store_ids")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
    else {
        return false;
    };
    let Some(store) = store.as_str().filter(|id| !id.is_empty()) else {
        return false;
    };
    e.text(principal, "identity_store_id") == Some(store)
        && super::lambda_core::unique(e.refs, principal, "identity_store_id").is_some_and(
            |reference| {
                reference.sources == [*endpoint]
                    && reference.complete
                    && reference.instances_explicit
                    && reference.issues.is_empty()
                    && reference.source_attributes.len() == 1
                    && reference.source_attributes.iter().all(|field| {
                        matches!(
                            field.as_str(),
                            "identity_store_ids" | "identity_store_ids[0]"
                        )
                    })
            },
        )
}

#[cfg(test)]
#[path = "../../../tests/unit/semantic/accountaccess_configuration.rs"]
mod tests;
