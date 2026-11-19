use super::configured_links::{self, Evidence, Rule};
use crate::model::{AttributeReference, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;

fn rule(kind: &str, path: &str) -> Option<Rule> {
    Some(match (kind, path) {
        ("aws_cloudwatchomni_access_grant" | "aws_cloudwatchomni_space", "domain_id") => Rule::new(
            "Omni domain",
            &[("aws_cloudwatchomni_domain", &["domain_id"])],
            false,
            &[],
        )
        .with_explicit_destination(),
        ("aws_cloudwatchomni_access_grant" | "aws_cloudwatchomni_access_profile", "space_id") => {
            Rule::new(
                "Omni space",
                &[("aws_cloudwatchomni_space", &["space_id"])],
                false,
                &[],
            )
        }
        ("aws_cloudwatchomni_access_grant", "principal.principal_id") => Rule::new(
            "Grant principal",
            &[
                ("aws_iam_role", &["arn"]),
                ("aws_iam_user", &["arn"]),
                ("aws_cloudwatchomni_access_profile", &["profile_id"]),
            ],
            false,
            &[],
        ),
        ("aws_cloudwatchomni_space", "data_access_role_arn") => Rule::new(
            "Space data access role",
            &[("aws_iam_role", &["arn"])],
            false,
            &[],
        ),
        ("aws_cloudwatchomni_space", "agent_core_evaluation_role_arn") => Rule::new(
            "Space evaluation role",
            &[("aws_iam_role", &["arn"])],
            false,
            &[],
        ),
        ("aws_cloudwatchomni_space", "encryption_configuration.kms_key_arn") => Rule::new(
            "Space encryption key",
            &[("aws_kms_key", &["arn"])],
            false,
            &[],
        ),
        (
            "aws_cloudwatchomni_domain",
            "identity_provider_configuration.identity_center_configuration.identity_center_instance_arn",
        ) => Rule::new(
            "Domain Identity Center instance",
            &[("aws_ssoadmin_instances", &["arns"])],
            false,
            &[],
        ),
        _ => return None,
    })
}
pub(crate) fn resource(kind: &str) -> bool {
    matches!(
        kind,
        "aws_cloudwatchomni_access_grant"
            | "aws_cloudwatchomni_space"
            | "aws_cloudwatchomni_access_profile"
            | "aws_cloudwatchomni_domain"
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
    match (
        e.nodes[r.target].resource_type.as_str(),
        r.attribute.as_str(),
    ) {
        (_, "domain_id") => domain_binding(e, r.target, source),
        ("aws_cloudwatchomni_access_grant", "space_id") => domain(e, r.target)
            .zip(domain(e, source))
            .is_some_and(|(grant, space)| grant == space),
        (_, "principal.principal_id") => {
            let Some((block, _)) = path.rsplit_once('.') else {
                return false;
            };
            let principal = e.text(r.target, &format!("{block}.principal_type"));
            match (principal, e.nodes[source].resource_type.as_str()) {
                (Some("IAM_ROLE"), "aws_iam_role") | (Some("IAM_USER"), "aws_iam_user") => true,
                (Some("ACCESS_PROFILE"), "aws_cloudwatchomni_access_profile") => {
                    e.same_endpoint(
                        r.target,
                        "space_id",
                        source,
                        "space_id",
                        "aws_cloudwatchomni_space",
                        "space_id",
                    ) && super::lambda_core::unique(e.refs, r.target, "space_id").is_some_and(
                        |space| {
                            let [space_index] = space.sources.as_slice() else {
                                return false;
                            };
                            domain(e, r.target)
                                .zip(domain(e, *space_index))
                                .is_some_and(|(grant, space)| grant == space)
                        },
                    )
                }
                _ => false,
            }
        }
        (_, "encryption_configuration.kms_key_arn") => {
            e.text(r.target, "encryption_configuration.0.encryption_strategy")
                == Some("CUSTOMER_MANAGED")
        }
        (
            _,
            "identity_provider_configuration.identity_center_configuration.identity_center_instance_arn",
        ) => e
            .value(r.target, "identity_providers")
            .and_then(Value::as_array)
            .is_some_and(|providers| {
                providers
                    .iter()
                    .all(|p| matches!(p.as_str(), Some("IAM" | "IDC")))
                    && providers.iter().any(|p| p.as_str() == Some("IDC"))
            }),
        _ => true,
    }
}
fn domain(e: &Evidence<'_>, child: usize) -> Option<usize> {
    use crate::model::{Action, EntityMode};
    let reference = super::lambda_core::unique(e.refs, child, "domain_id")?;
    let [index] = reference.sources.as_slice() else {
        return None;
    };
    let consumer = &e.nodes[child];
    let owner = &e.nodes[*index];
    let current = |node: &TerraformEntity| {
        node.provider.is_aws()
            && node.mode == EntityMode::Managed
            && node.action != Action::Delete
            && node.deposed_key.is_none()
    };
    (current(consumer)
        && current(owner)
        && owner.resource_type == "aws_cloudwatchomni_domain"
        && reference.complete
        && reference.instances_explicit
        && reference.issues.is_empty()
        && reference
            .source_attributes_by_instance
            .get(index)
            .is_some_and(|fields| fields.len() == 1 && fields.contains("domain_id"))
        && e.text(child, "domain_id")
            .is_some_and(|value| e.text(*index, "domain_id") == Some(value))
        && domain_binding(e, child, *index))
    .then_some(*index)
}

fn domain_binding(e: &Evidence<'_>, child: usize, owner: usize) -> bool {
    let consumer = &e.nodes[child];
    let domain = &e.nodes[owner];
    let scopes = super::scope::scope(consumer)
        .and_then(|scope| scope.region)
        .zip(super::scope::scope(domain).and_then(|scope| scope.region));
    let Some((child_region, domain_region)) = scopes else {
        return false;
    };
    let configs = consumer
        .provider_configuration
        .as_ref()
        .zip(domain.provider_configuration.as_ref());
    let Some((child_config, domain_config)) = configs else {
        return false;
    };
    if child_config.account_conflict || domain_config.account_conflict {
        return false;
    }
    if child_config == domain_config {
        return true;
    }
    let (field, kind, identifier) = if consumer.resource_type == "aws_cloudwatchomni_space" {
        ("space_arn", "space", "space_id")
    } else {
        ("arn", "access-grant", "grant_id")
    };
    let accounts = [
        (child, child_config, &child_region, field, kind, identifier),
        (
            owner,
            domain_config,
            &domain_region,
            "arn",
            "domain",
            "domain_id",
        ),
    ]
    .map(|(index, config, region, field, kind, identifier)| {
        let arn = e.text(index, field).and_then(|arn| {
            let mut parts = arn.splitn(6, ':');
            let [
                Some("arn"),
                Some("aws"),
                Some("cloudwatch"),
                Some(arn_region),
                Some(account),
                Some(resource),
            ] = std::array::from_fn(|_| parts.next())
            else {
                return None;
            };
            (arn_region == region.0
                && resource.split_once('/') == e.text(index, identifier).map(|id| (kind, id)))
            .then(|| crate::model::AwsAccountScope::new(account))
            .flatten()
        });
        match (&config.account, arn) {
            (Some(configured), Some(observed)) if *configured != observed => None,
            (Some(configured), _) => Some(configured.clone()),
            (None, observed) => observed,
        }
    });
    matches!(accounts,[Some(left),Some(right)] if left==right)
}

#[cfg(test)]
#[path = "../../../tests/unit/semantic/cloudwatch_omni.rs"]
mod tests;
