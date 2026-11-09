use crate::model::{Action, AttributeReference, EntityMode, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/identity_center.rs"]
mod tests;

pub(super) fn owner(kind: &str) -> Option<(&'static str, &'static str)> {
    match kind {
        "aws_ssoadmin_application_access_scope"
        | "aws_ssoadmin_application_assignment"
        | "aws_ssoadmin_application_assignment_configuration" => {
            Some(("application_arn", "Identity Center application"))
        }
        "aws_ssoadmin_customer_managed_policy_attachment"
        | "aws_ssoadmin_customer_managed_policy_attachments_exclusive"
        | "aws_ssoadmin_managed_policy_attachment"
        | "aws_ssoadmin_managed_policy_attachments_exclusive"
        | "aws_ssoadmin_permission_set_inline_policy"
        | "aws_ssoadmin_permissions_boundary_attachment" => {
            Some(("permission_set_arn", "Identity Center permission set"))
        }
        _ => None,
    }
}

pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    if let Some((field, label)) = owner(kind) {
        if field == path {
            return Some(label);
        }
    }
    match (kind, path) {
        ("aws_ssoadmin_account_assignment", "permission_set_arn") => {
            Some("Identity Center permission set")
        }
        (
            "aws_ssoadmin_account_assignment" | "aws_ssoadmin_application_assignment",
            "principal_id",
        ) => Some("Identity Center principal"),
        ("aws_ssoadmin_account_assignment", "target_id") => Some("Assignment target account"),
        _ => None,
    }
}

fn current(n: &TerraformEntity, mode: EntityMode) -> bool {
    n.provider.is_aws() && n.mode == mode && n.action != Action::Delete && n.deposed_key.is_none()
}

fn scalar_shape(nodes: &[TerraformEntity], r: &AttributeReference, mode: EntityMode) -> bool {
    current(&nodes[r.target], EntityMode::Managed)
        && r.complete
        && r.instances_explicit
        && r.issues.is_empty()
        && r.sources.len() == 1
        && r.source_attributes.len() == 1
        && r.sources[0] != r.target
        && current(&nodes[r.sources[0]], mode)
}

fn compatible(nodes: &[TerraformEntity], target: usize, source: usize, global: bool) -> bool {
    let left = &nodes[target];
    let right = &nodes[source];
    super::eventbridge::bindings(
        left.provider_configuration.as_ref(),
        right.provider_configuration.as_ref(),
        super::scope::scope(left)
            .as_ref()
            .and_then(|s| s.region.as_ref()),
        super::scope::scope(right)
            .as_ref()
            .and_then(|s| s.region.as_ref()),
        global,
    )
}

fn proof(nodes: &[TerraformEntity], r: &AttributeReference, evidence: &[(&Value, &Value)]) -> bool {
    if !scalar_shape(nodes, r, EntityMode::Managed)
        || !compatible(nodes, r.target, r.sources[0], r.attribute == "target_id")
    {
        return false;
    }
    let source = r.sources[0];
    let field = r.source_attributes.first().unwrap().as_str();
    let known = |index, key| super::service_evidence::known(evidence, index, key);
    if known(r.target, &r.attribute).is_none_or(|v| known(source, field) != Some(v)) {
        return false;
    }
    match (
        r.attribute.as_str(),
        nodes[source].resource_type.as_str(),
        field,
    ) {
        ("application_arn", "aws_ssoadmin_application", "application_arn" | "arn") => true,
        ("permission_set_arn", "aws_ssoadmin_permission_set", "arn") => {
            known(r.target, "instance_arn")
                .is_some_and(|v| known(source, "instance_arn") == Some(v))
        }
        ("principal_id", "aws_identitystore_user", "user_id") => {
            known(r.target, "principal_type") == Some("USER")
        }
        ("principal_id", "aws_identitystore_group", "group_id") => {
            known(r.target, "principal_type") == Some("GROUP")
        }
        ("target_id", "aws_organizations_account", "id") => {
            known(r.target, "target_type") == Some("AWS_ACCOUNT")
                && known(source, "id")
                    .and_then(crate::model::AwsAccountScope::new)
                    .is_some()
        }
        _ => false,
    }
}

fn instance_bridge(
    nodes: &[TerraformEntity],
    refs: &[AttributeReference],
    evidence: &[(&Value, &Value)],
    target: usize,
    path: &str,
    output: &str,
) -> Option<usize> {
    let r = super::lambda_core::unique(refs, target, path)?;
    if !scalar_shape(nodes, r, EntityMode::Data) || !compatible(nodes, target, r.sources[0], false)
    {
        return None;
    }
    let source = r.sources[0];
    if nodes[source].resource_type != "aws_ssoadmin_instances"
        || !r
            .source_attributes
            .iter()
            .all(|f| f == output || f == &format!("{output}[0]"))
    {
        return None;
    }
    let (value, mask) = evidence[source];
    let values = super::streaming::values(value, mask, output, true)?;
    (values.len() == 1
        && super::service_evidence::known(evidence, target, path)
            .is_some_and(|v| values.contains(v)))
    .then_some(source)
}

fn principal_context(
    nodes: &[TerraformEntity],
    refs: &[AttributeReference],
    evidence: &[(&Value, &Value)],
    r: &AttributeReference,
) -> Option<Vec<usize>> {
    let mut context = Vec::new();
    let instance_owner = if nodes[r.target].resource_type == "aws_ssoadmin_application_assignment" {
        let app = super::lambda_core::unique(refs, r.target, "application_arn")?;
        if !proof(nodes, app, evidence) {
            return None;
        }
        context.push(app.sources[0]);
        app.sources[0]
    } else {
        r.target
    };
    let instance = instance_bridge(
        nodes,
        refs,
        evidence,
        instance_owner,
        "instance_arn",
        "arns",
    )?;
    let store = instance_bridge(
        nodes,
        refs,
        evidence,
        r.sources[0],
        "identity_store_id",
        "identity_store_ids",
    )?;
    if instance != store {
        return None;
    }
    context.push(instance);
    Some(context)
}

pub(crate) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    evidence: &[(&Value, &Value)],
) {
    let results: Vec<_> = refs
        .iter()
        .map(|r| {
            label(&nodes[r.target].resource_type, &r.attribute)?;
            let valid = super::lambda_core::unique(refs, r.target, &r.attribute).is_some()
                && proof(nodes, r, evidence);
            if !valid || r.attribute != "principal_id" {
                return Some((valid, Vec::new()));
            }
            let context = principal_context(nodes, refs, evidence, r).unwrap_or_default();
            Some((!context.is_empty(), context))
        })
        .collect();
    for (r, result) in refs.iter_mut().zip(results) {
        if let Some((valid, context)) = result {
            r.service_reference_verified = valid;
            r.endpoint_network_context = context;
        }
    }
}

pub(super) fn bindings(raw: &Input, r: &AttributeReference) -> bool {
    r.sources
        .iter()
        .chain(&r.endpoint_network_context)
        .all(|&s| {
            let left = &raw.nodes[r.target];
            let right = &raw.nodes[s];
            super::eventbridge::bindings(
                left.provider_configuration.as_ref(),
                right.provider_configuration.as_ref(),
                left.entity.scope.as_ref().and_then(|s| s.region.as_ref()),
                right.entity.scope.as_ref().and_then(|s| s.region.as_ref()),
                r.attribute == "target_id",
            )
        })
}
