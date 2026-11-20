use super::configured_links::{self, Evidence, Rule};
use crate::model::{AttributeReference, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;

fn rule(kind: &str, path: &str) -> Option<Rule> {
    Some(match (kind, path) {
        (
            "aws_resiliencehubv2_assertion"
            | "aws_resiliencehubv2_input_source"
            | "aws_resiliencehubv2_service_function",
            "service_arn",
        ) => Rule::new(
            "Resilience Hub service",
            &[("aws_resiliencehubv2_service", &["arn"])],
            false,
            &[],
        ),
        ("aws_resiliencehubv2_input_source", "resource_configuration.cfn_stack_arn") => Rule::new(
            "Discovery stack",
            &[("aws_cloudformation_stack", &["id"])],
            false,
            &["resource_configuration"],
        )
        .with_explicit_destination(),
        ("aws_resiliencehubv2_input_source", "resource_configuration.eks.cluster_arn") => {
            Rule::new(
                "Discovery cluster",
                &[("aws_eks_cluster", &["arn"])],
                false,
                &["resource_configuration", "resource_configuration.eks"],
            )
            .with_explicit_destination()
        }
        (
            "aws_resiliencehubv2_service"
            | "aws_resiliencehubv2_system"
            | "aws_resiliencehubv2_policy",
            "kms_key_id",
        ) => Rule::new("Encryption key", &[("aws_kms_key", &["arn"])], false, &[]),
        ("aws_resiliencehubv2_service" | "aws_resiliencehubv2_user_journey", "policy_arn") => {
            Rule::new(
                "Resilience policy",
                &[("aws_resiliencehubv2_policy", &["arn"])],
                false,
                &[],
            )
        }
        ("aws_resiliencehubv2_service", "associated_system.system_arn") => Rule::new(
            "Associated system",
            &[("aws_resiliencehubv2_system", &["arn"])],
            false,
            &["associated_system"],
        ),
        ("aws_resiliencehubv2_service", "associated_system.user_journey_ids") => Rule::new(
            "System journeys",
            &[("aws_resiliencehubv2_user_journey", &["user_journey_id"])],
            true,
            &["associated_system"],
        ),
        ("aws_resiliencehubv2_service", "permission_model.invoker_role_name") => Rule::new(
            "Discovery role",
            &[("aws_iam_role", &["name"])],
            false,
            &["permission_model"],
        ),
        (
            "aws_resiliencehubv2_service",
            "permission_model.cross_account_role.cross_account_role_arn",
        ) => Rule::new(
            "Cross-account discovery role",
            &[("aws_iam_role", &["arn"])],
            false,
            &["permission_model", "permission_model.cross_account_role"],
        )
        .with_explicit_destination(),
        ("aws_resiliencehubv2_user_journey", "system_arn") => Rule::new(
            "Journey system",
            &[("aws_resiliencehubv2_system", &["arn"])],
            false,
            &[],
        ),
        _ => return None,
    })
}
pub(crate) fn resource(kind: &str) -> bool {
    matches!(
        kind,
        "aws_resiliencehubv2_assertion"
            | "aws_resiliencehubv2_input_source"
            | "aws_resiliencehubv2_service_function"
            | "aws_resiliencehubv2_service"
            | "aws_resiliencehubv2_system"
            | "aws_resiliencehubv2_policy"
            | "aws_resiliencehubv2_user_journey"
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
        "associated_system.user_journey_ids" => {
            let Some((block, _)) = path.rsplit_once('.') else {
                return false;
            };
            e.same_endpoint(
                r.target,
                &format!("{block}.system_arn"),
                source,
                "system_arn",
                "aws_resiliencehubv2_system",
                "arn",
            )
        }
        "permission_model.cross_account_role.cross_account_role_arn" => e
            .text(source, "arn")
            .is_some_and(|arn| endpoint_arn(e, source, arn, "iam", "role/", true)),
        "resource_configuration.cfn_stack_arn" => e
            .text(source, "id")
            .is_some_and(|arn| endpoint_arn(e, source, arn, "cloudformation", "stack/", false)),
        "resource_configuration.eks.cluster_arn" => e
            .text(source, "arn")
            .is_some_and(|arn| endpoint_arn(e, source, arn, "eks", "cluster/", false)),
        "kms_key_id" => e
            .text(source, "arn")
            .is_some_and(|arn| endpoint_arn(e, source, arn, "kms", "key/", false)),
        _ => true,
    }
}
#[cfg(test)]
#[path = "../../../tests/unit/semantic/resilience_hub.rs"]
mod tests;

fn endpoint_arn(
    e: &Evidence<'_>,
    source: usize,
    arn: &str,
    service: &str,
    prefix: &str,
    global: bool,
) -> bool {
    let fields: Vec<_> = arn.splitn(6, ':').collect();
    let ["arn", partition, actual, region, account, resource] = fields.as_slice() else {
        return false;
    };
    let Some(binding) = e.nodes[source].provider_configuration.as_ref() else {
        return false;
    };
    !partition.is_empty()
        && *actual == service
        && crate::model::AwsAccountScope::new(account).is_some_and(|id| {
            binding
                .account
                .as_ref()
                .is_none_or(|expected| expected == &id)
        })
        && resource
            .strip_prefix(prefix)
            .is_some_and(|name| !name.is_empty())
        && if global {
            region.is_empty()
        } else {
            super::scope(&e.nodes[source])
                .and_then(|s| s.region)
                .is_some_and(|v| v.0 == *region)
        }
}
