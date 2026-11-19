use super::configured_links::{self, Evidence, Rule};
use crate::model::{AttributeReference, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;

fn rule(kind: &str, path: &str) -> Option<Rule> {
    Some(match (kind, path) {
        ("aws_cloudwatch_log_s3_table_integration_source", "integration_arn") => Rule::new(
            "S3 table integration",
            &[("aws_observabilityadmin_s3_table_integration", &["arn"])],
            false,
            &[],
        ),
        ("aws_observabilityadmin_s3_table_integration", "role_arn") => Rule::new(
            "S3 table integration role",
            &[("aws_iam_role", &["arn"])],
            false,
            &[],
        ),
        ("aws_observabilityadmin_s3_table_integration", "encryption.kms_key_arn") => Rule::new(
            "S3 table integration key",
            &[("aws_kms_key", &["arn"])],
            false,
            &[],
        ),
        (
            "aws_observabilityadmin_centralization_rule_for_organization",
            "rule.destination.destination_logs_configuration.backup_configuration.kms_key_arn",
        ) => Rule::new("Backup log key", &[("aws_kms_key", &["arn"])], false, &[])
            .with_explicit_destination(),
        (
            "aws_observabilityadmin_centralization_rule_for_organization",
            "rule.destination.destination_logs_configuration.logs_encryption_configuration.kms_key_arn",
        ) => Rule::new(
            "Destination log key",
            &[("aws_kms_key", &["arn"])],
            false,
            &[],
        )
        .with_explicit_destination(),
        (
            "aws_observabilityadmin_centralization_rule_for_organization",
            "rule.destination.destination_logs_configuration.tag_propagation_configuration.destination_role_arn",
        ) => Rule::new(
            "Destination tag propagation role",
            &[("aws_iam_role", &["arn"])],
            false,
            &[],
        )
        .with_explicit_destination(),
        _ => return None,
    })
}
pub(crate) fn resource(kind: &str) -> bool {
    matches!(
        kind,
        "aws_cloudwatch_log_s3_table_integration_source"
            | "aws_observabilityadmin_s3_table_integration"
            | "aws_observabilityadmin_centralization_rule_for_organization"
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
        "encryption.kms_key_arn" => {
            e.text(r.target, "encryption.0.sse_algorithm") == Some("aws:kms")
        }
        p if p.starts_with("rule.destination.") => {
            let Some((destination, _)) = path.split_once(".destination_logs_configuration.") else {
                return false;
            };
            let Some((block, _)) = path.rsplit_once('.') else {
                return false;
            };
            let Some(account) = e.text(r.target, &format!("{destination}.account")) else {
                return false;
            };
            let Some(region) = e.text(r.target, &format!("{destination}.region")) else {
                return false;
            };
            let Some(arn) = e.text(source, "arn") else {
                return false;
            };
            let mut parts = arn.splitn(6, ':');
            let [
                Some("arn"),
                Some(_),
                Some(service),
                Some(arn_region),
                Some(arn_account),
                Some(resource),
            ] = std::array::from_fn(|_| parts.next())
            else {
                return false;
            };
            if crate::model::AwsAccountScope::new(account).is_none() || account != arn_account {
                return false;
            }
            let endpoint = &e.nodes[source];
            if endpoint
                .provider_configuration
                .as_ref()
                .and_then(|provider| provider.account.as_ref())
                .is_some_and(|scope| scope.as_str() != account)
            {
                return false;
            }
            if p.ends_with(".destination_role_arn") {
                return service == "iam" && arn_region.is_empty() && resource.starts_with("role/");
            }
            let key_region = if p.contains(".backup_configuration.") {
                e.text(r.target, &format!("{block}.region"))
            } else {
                if e.text(r.target, &format!("{block}.encryption_strategy"))
                    != Some("CUSTOMER_MANAGED")
                {
                    return false;
                }
                Some(region)
            };
            service == "kms"
                && resource.starts_with("key/")
                && key_region == Some(arn_region)
                && super::scope::scope(endpoint)
                    .and_then(|scope| scope.region)
                    .is_some_and(|scope| scope.0 == arn_region)
        }
        _ => true,
    }
}
#[cfg(test)]
#[path = "../../../tests/unit/semantic/observability_logs.rs"]
mod tests;
