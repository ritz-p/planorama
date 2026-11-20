use super::configured_links::{self, Evidence, Rule};
use crate::model::{AttributeReference, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;

fn rule(kind: &str, path: &str) -> Option<Rule> {
    Some(match (kind, path) {
        ("aws_cloudtrail", "sns_topic_name") => Rule::new(
            "Trail notification topic",
            &[("aws_sns_topic", &["name"])],
            false,
            &[],
        ),
        ("aws_cloudtrail_event_data_store", "kms_key_id") => Rule::new(
            "Event data store encryption key",
            &[
                ("aws_kms_key", &["arn", "key_id", "id"]),
                ("aws_kms_alias", &["arn", "name"]),
            ],
            false,
            &[],
        ),
        ("aws_controltower_baseline", "target_identifier") => Rule::new(
            "Control Tower target OU",
            &[("aws_organizations_organizational_unit", &["arn"])],
            false,
            &[],
        ),
        ("aws_controltower_control", "target_identifier") => Rule::new(
            "Control Tower target OU",
            &[("aws_organizations_organizational_unit", &["arn"])],
            false,
            &[],
        ),
        ("aws_licensemanager_association", "license_configuration_arn") => Rule::new(
            "License configuration",
            &[("aws_licensemanager_license_configuration", &["arn"])],
            false,
            &[],
        ),
        ("aws_licensemanager_association", "resource_arn") => Rule::new(
            "License assignment target",
            &[
                ("aws_instance", &["arn"]),
                ("aws_ec2_host", &["arn"]),
                ("aws_ami", &["arn"]),
                ("aws_ami_copy", &["arn"]),
                ("aws_ami_from_instance", &["arn"]),
            ],
            false,
            &[],
        ),
        ("aws_licensemanager_grant", "principal") => Rule::new(
            "License grant recipient",
            &[("aws_organizations_account", &["id"])],
            false,
            &[],
        )
        .with_explicit_destination()
        .decoded(account_root),
        ("aws_licensemanager_grant_accepter", "grant_arn") => Rule::new(
            "Accepted license grant",
            &[("aws_licensemanager_grant", &["arn"])],
            false,
            &[],
        )
        .with_explicit_destination(),
        _ => return None,
    })
}

pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    rule(kind, path).map(|rule| rule.label)
}

pub(super) fn bindings(raw: &Input, reference: &AttributeReference) -> bool {
    configured_links::bindings(raw, reference, rule)
}

pub(super) fn annotate(
    nodes: &[TerraformEntity],
    references: &mut [AttributeReference],
    values: &[(&Value, &Value)],
) {
    configured_links::annotate(nodes, references, values, rule, qualifies);
}

fn qualifies(e: &Evidence<'_>, r: &AttributeReference, source: usize, path: &str) -> bool {
    match (
        e.nodes[r.target].resource_type.as_str(),
        r.attribute.as_str(),
    ) {
        ("aws_controltower_baseline" | "aws_controltower_control", "target_identifier") => e
            .text(r.target, path)
            .and_then(organizations_ou_account)
            .is_some_and(|account| {
                e.nodes[source]
                    .provider_configuration
                    .as_ref()
                    .and_then(|config| config.account.as_ref())
                    .is_none_or(|owner| owner.as_str() == account)
            }),
        ("aws_licensemanager_association", "license_configuration_arn") => {
            e.text(r.target, path).is_some_and(|v| {
                arn_resource(v, "license-manager", "license-configuration:").is_some()
            })
        }
        ("aws_licensemanager_association", "resource_arn") => {
            let prefix = match e.nodes[source].resource_type.as_str() {
                "aws_instance" => "instance/",
                "aws_ec2_host" => "dedicated-host/",
                _ => "image/",
            };
            e.text(r.target, path)
                .is_some_and(|v| arn_resource(v, "ec2", prefix).is_some())
        }
        ("aws_licensemanager_grant_accepter", "grant_arn") => e
            .text(r.target, path)
            .and_then(|v| arn_resource(v, "license-manager", "grant:"))
            .is_some_and(|(region, account, id)| {
                region.is_empty()
                    && !id.is_empty()
                    && endpoint_account(e, source, account)
                    && e.text(source, "home_region")
                        .is_some_and(|home| endpoint_region(e, r.target, home))
            }),
        _ => true,
    }
}

fn organizations_ou_account(arn: &str) -> Option<&str> {
    let mut fields = arn.splitn(6, ':');
    match (
        fields.next(),
        fields.next(),
        fields.next(),
        fields.next(),
        fields.next(),
        fields.next(),
    ) {
        (
            Some("arn"),
            Some("aws" | "aws-us-gov" | "aws-cn"),
            Some("organizations"),
            Some(""),
            Some(account),
            Some(resource),
        ) if crate::model::AwsAccountScope::new(account).is_some()
            && resource
                .strip_prefix("ou/o-")
                .and_then(|path| path.split_once("/ou-"))
                .is_some_and(|(org, ou)| {
                    !org.is_empty() && !ou.is_empty() && !ou.contains('/')
                }) =>
        {
            Some(account)
        }
        _ => None,
    }
}

fn account_root<'a>(_: &str, value: &'a Value) -> Option<std::collections::BTreeSet<&'a str>> {
    let (region, account, identity) = arn_resource(value.as_str()?, "iam", "root")?;
    (region.is_empty()
        && identity.is_empty()
        && crate::model::AwsAccountScope::new(account).is_some())
    .then(|| [account].into())
}

fn arn_resource<'a>(
    value: &'a str,
    service: &str,
    prefix: &str,
) -> Option<(&'a str, &'a str, &'a str)> {
    let mut fields = value.splitn(6, ':');
    if fields.next()? != "arn"
        || !matches!(fields.next()?, "aws" | "aws-cn" | "aws-us-gov")
        || fields.next()? != service
    {
        return None;
    }
    let region = fields.next()?;
    let account = fields.next()?;
    let identity = fields.next()?.strip_prefix(prefix)?;
    Some((region, account, identity))
}

fn endpoint_account(e: &Evidence<'_>, node: usize, account: &str) -> bool {
    e.nodes[node]
        .provider_configuration
        .as_ref()
        .and_then(|config| config.account.as_ref())
        .is_some_and(|owner| owner.as_str() == account)
}

fn endpoint_region(e: &Evidence<'_>, node: usize, region: &str) -> bool {
    super::scope::scope(&e.nodes[node])
        .and_then(|scope| scope.region)
        .is_some_and(|effective| effective.0 == region)
}

#[cfg(test)]
#[path = "../../../tests/unit/semantic/operations.rs"]
mod tests;
