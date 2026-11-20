use super::configured_links::{self, Evidence, Rule};
use crate::model::{AttributeReference, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;

fn rule(kind: &str, path: &str) -> Option<Rule> {
    Some(match (kind, path) {
        ("aws_securitylake_aws_log_source", "source.accounts") => Rule::new(
            "Log source accounts",
            &[("aws_organizations_account", &["id"])],
            true,
            &["source"],
        ),
        ("aws_securitylake_custom_log_source", "configuration.crawler_configuration.role_arn") => {
            Rule::new(
                "Log source crawler role",
                &[("aws_iam_role", &["arn"])],
                false,
                &["configuration", "configuration.crawler_configuration"],
            )
        }
        ("aws_securitylake_data_lake", "configuration.encryption_configuration") => Rule::new(
            "Lake encryption key",
            &[("aws_kms_key", &["key_id", "arn"])],
            true,
            &["configuration"],
        )
        .with_explicit_destination()
        .decoded(encryption_key),
        ("aws_securitylake_data_lake", "configuration.replication_configuration.role_arn") => {
            Rule::new(
                "Lake replication role",
                &[("aws_iam_role", &["arn"])],
                false,
                &["configuration", "configuration.replication_configuration"],
            )
        }
        ("aws_securitylake_data_lake", "meta_store_manager_role_arn") => {
            Rule::new("Metastore role", &[("aws_iam_role", &["arn"])], false, &[])
        }
        ("aws_securitylake_subscriber", "source.custom_log_source_resource.source_name") => {
            Rule::new(
                "Custom log source",
                &[("aws_securitylake_custom_log_source", &["source_name"])],
                false,
                &["source", "source.custom_log_source_resource"],
            )
        }
        ("aws_securitylake_subscriber_notification", "subscriber_id") => Rule::new(
            "Security Lake subscriber",
            &[("aws_securitylake_subscriber", &["id"])],
            false,
            &[],
        ),
        (
            "aws_securitylake_subscriber_notification",
            "configuration.https_notification_configuration.target_role_arn",
        ) => Rule::new(
            "Notification role",
            &[("aws_iam_role", &["arn"])],
            false,
            &[
                "configuration",
                "configuration.https_notification_configuration",
            ],
        ),
        _ => return None,
    })
}
pub(crate) fn resource(kind: &str) -> bool {
    matches!(
        kind,
        "aws_securitylake_aws_log_source"
            | "aws_securitylake_custom_log_source"
            | "aws_securitylake_data_lake"
            | "aws_securitylake_subscriber"
            | "aws_securitylake_subscriber_notification"
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
    match r.attribute.as_str() {
        "configuration.encryption_configuration" => {
            path.rsplit_once('.')
                .is_some_and(|(p, _)| e.source_region(r.target, &format!("{p}.region"), source))
                && super::eventbridge::bindings(
                    e.nodes[r.target].provider_configuration.as_ref(),
                    e.nodes[source].provider_configuration.as_ref(),
                    super::scope(&e.nodes[r.target])
                        .as_ref()
                        .and_then(|s| s.region.as_ref()),
                    None,
                    true,
                )
        }
        "source.custom_log_source_resource.source_name" => path
            .rsplit_once('.')
            .and_then(|(p, _)| e.text(r.target, &format!("{p}.source_version")))
            .zip(e.text(source, "source_version"))
            .is_some_and(|(a, b)| a == b),
        "source.accounts" => e
            .text(source, "id")
            .and_then(crate::model::AwsAccountScope::new)
            .is_some(),
        _ => true,
    }
}
#[cfg(test)]
#[path = "../../../tests/unit/semantic/security_lake.rs"]
mod tests;

fn encryption_key<'a>(_: &str, value: &'a Value) -> Option<std::collections::BTreeSet<&'a str>> {
    let [config] = value.as_array()?.as_slice() else {
        return None;
    };
    Some([config.get("kms_key_id")?.as_str()?].into())
}
