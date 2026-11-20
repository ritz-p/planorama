use super::{Evidence, Rule};
use crate::model::AttributeReference;

pub(super) fn resource(kind: &str) -> bool {
    matches!(
        kind,
        "aws_cloudformation_stack_set"
            | "aws_cloudformation_stack_instances"
            | "aws_cloudformation_stack_set_instance"
    )
}

pub(super) fn rule(kind: &str, path: &str) -> Option<Rule> {
    Some(match (kind, path) {
        ("aws_cloudformation_stack_set", "administration_role_arn") => Rule::new(
            "StackSet administration role",
            &[("aws_iam_role", &["arn"])],
            false,
            &[],
        ),
        ("aws_cloudformation_stack_set", "execution_role_name") => Rule::new(
            "StackSet target execution role",
            &[("aws_iam_role", &["name"])],
            false,
            &[],
        )
        .with_explicit_destination(),
        ("aws_cloudformation_stack_set", "auto_deployment.depends_on_stack_sets") => Rule::new(
            "StackSet auto-deployment prerequisites",
            &[("aws_cloudformation_stack_set", &["arn"])],
            true,
            &[],
        ),
        (
            "aws_cloudformation_stack_set_instance" | "aws_cloudformation_stack_instances",
            "stack_set_name",
        ) => Rule::new(
            "Administering StackSet",
            &[("aws_cloudformation_stack_set", &["name"])],
            false,
            &[],
        )
        .in_control_plane_account(),
        ("aws_cloudformation_stack_set_instance", "account_id") => Rule::new(
            "StackSet instance target account",
            &[("aws_organizations_account", &["id"])],
            false,
            &[],
        )
        .with_explicit_destination(),
        ("aws_cloudformation_stack_instances", "accounts") => Rule::new(
            "StackSet instances target accounts",
            &[("aws_organizations_account", &["id"])],
            true,
            &[],
        )
        .with_explicit_destination(),
        (
            "aws_cloudformation_stack_set_instance" | "aws_cloudformation_stack_instances",
            "deployment_targets.organizational_unit_ids",
        ) => Rule::new(
            "StackSet deployment selector",
            &[
                ("aws_organizations_organizational_unit", &["id"]),
                ("aws_organizations_organization", &["roots[].id"]),
            ],
            true,
            &[],
        )
        .with_explicit_destination(),
        (
            "aws_cloudformation_stack_set_instance" | "aws_cloudformation_stack_instances",
            "deployment_targets.accounts",
        ) => Rule::new(
            "StackSet account filter",
            &[("aws_organizations_account", &["id"])],
            true,
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
    let target = r.target;
    match r.attribute.as_str() {
        "administration_role_arn" => self_managed(e, target),
        "execution_role_name" => self_managed(e, target) && execution_account(e, target, source),
        "auto_deployment.depends_on_stack_sets" => {
            e.text(target, "permission_model") == Some("SERVICE_MANAGED")
                && e.value(target, "auto_deployment.0.enabled")
                    .and_then(serde_json::Value::as_bool)
                    == Some(true)
        }
        "stack_set_name" => administration_matches(e, target, source),
        "account_id" | "accounts" | "deployment_targets.accounts" => {
            e.text(source, "id")
                .and_then(AwsAccountScope::new)
                .is_some()
                && targets_have_region(e, target)
                && stack_set(e, target).is_some_and(|stack| {
                    if r.attribute.starts_with("deployment_targets.") {
                        e.text(stack, "permission_model") == Some("SERVICE_MANAGED")
                    } else {
                        self_managed(e, stack) && e.absent(target, "deployment_targets")
                    }
                })
        }
        "deployment_targets.organizational_unit_ids" => {
            e.value(target, path)
                .and_then(serde_json::Value::as_array)
                .is_some_and(|ids| {
                    ids.iter().all(|id| {
                        id.as_str()
                            .is_some_and(|id| id.starts_with("ou-") || id.starts_with("r-"))
                    })
                })
                && targets_have_region(e, target)
                && stack_set(e, target).is_some_and(|stack| {
                    e.text(stack, "permission_model") == Some("SERVICE_MANAGED")
                })
        }
        _ => true,
    }
}

fn self_managed(e: &Evidence<'_>, node: usize) -> bool {
    e.absent(node, "permission_model") || e.text(node, "permission_model") == Some("SELF_MANAGED")
}

fn administration_matches(e: &Evidence<'_>, instance: usize, stack: usize) -> bool {
    use crate::provider::aws::{configured_links::same_account, scope::scope};
    let declaration = &e.nodes[instance];
    let left = declaration.provider_configuration.as_ref();
    let region = if declaration.resource_type == "aws_cloudformation_stack_instances"
        && declaration.has_region_override
    {
        declaration.region_override.as_ref()
    } else {
        left.and_then(|config| config.region.as_ref())
    };
    let right = e.nodes[stack].provider_configuration.as_ref();
    let call_as = |node| {
        e.text(node, "call_as")
            .or_else(|| e.absent(node, "call_as").then_some("SELF"))
    };
    same_account(left, right)
        && region
            .zip(scope(&e.nodes[stack]).and_then(|scope| scope.region))
            .is_some_and(|(admin, region)| admin == &region)
        && call_as(instance)
            .zip(call_as(stack))
            .is_some_and(|(a, b)| a == b)
}

fn stack_set(e: &Evidence<'_>, instance: usize) -> Option<usize> {
    use crate::model::{Action, EntityMode};
    let reference = crate::provider::aws::lambda_core::unique(e.refs, instance, "stack_set_name")?;
    let [stack] = *reference.sources.as_slice() else {
        return None;
    };
    let node = &e.nodes[stack];
    (node.provider.is_aws()
        && node.mode == EntityMode::Managed
        && node.action != Action::Delete
        && node.deposed_key.is_none()
        && node.resource_type == "aws_cloudformation_stack_set"
        && reference.complete
        && reference.instances_explicit
        && reference.issues.is_empty()
        && reference
            .source_attributes_by_instance
            .get(&stack)
            .is_some_and(|fields| fields.len() == 1 && fields.contains("name"))
        && e.text(instance, "stack_set_name")
            .zip(e.text(stack, "name"))
            .is_some_and(|(a, b)| a == b)
        && administration_matches(e, instance, stack))
    .then_some(stack)
}

fn targets_have_region(e: &Evidence<'_>, node: usize) -> bool {
    use crate::provider::aws::scope_selections::{region, stackset_target_region};
    if e.nodes[node].resource_type == "aws_cloudformation_stack_set_instance" {
        return stackset_target_region(node, e.values[node], e.refs).is_some();
    }
    e.value(node, "regions")
        .and_then(serde_json::Value::as_array)
        .is_some_and(|values| {
            !values.is_empty() && values.iter().all(|v| v.as_str().and_then(region).is_some())
        })
}

fn execution_account(e: &Evidence<'_>, stack: usize, source: usize) -> bool {
    use crate::model::{Action, AwsAccountScope, EntityMode};
    let account = e.nodes[source]
        .provider_configuration
        .as_ref()
        .and_then(|config| config.account.as_ref());
    let from_arn = e
        .text(source, "arn")
        .and_then(|arn| arn.split(':').nth(4))
        .and_then(AwsAccountScope::new);
    let Some(account) = account.or(from_arn.as_ref()) else {
        return false;
    };
    if from_arn
        .as_ref()
        .is_some_and(|arn_account| arn_account != account)
    {
        return false;
    }
    e.nodes.iter().enumerate().any(|(instance, node)| {
        node.provider.is_aws()
            && node.mode == EntityMode::Managed
            && node.action != Action::Delete
            && node.deposed_key.is_none()
            && matches!(
                node.resource_type.as_str(),
                "aws_cloudformation_stack_instances" | "aws_cloudformation_stack_set_instance"
            )
            && stack_set(e, instance) == Some(stack)
            && targets_have_region(e, instance)
            && e.absent(instance, "deployment_targets")
            && (e.text(instance, "account_id") == Some(account.as_str())
                || e.value(instance, "accounts")
                    .and_then(serde_json::Value::as_array)
                    .is_some_and(|accounts| {
                        accounts
                            .iter()
                            .any(|v| v.as_str() == Some(account.as_str()))
                    }))
    })
}
