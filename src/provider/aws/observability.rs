use crate::model::{Action, AttributeReference, EntityMode, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;
use std::collections::BTreeSet;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/observability.rs"]
mod tests;

type Rule = (&'static str, &'static str, bool, &'static str);

fn rule(resource: &str, path: &str) -> Option<Rule> {
    match (resource, path) {
        ("aws_grafana_workspace", "role_arn") => {
            Some(("aws_iam_role", "arn", false, "Workspace role"))
        }
        ("aws_grafana_workspace", "kms_key_id") => {
            Some(("aws_kms_key", "arn", false, "Workspace encryption key"))
        }
        ("aws_grafana_workspace", "organizational_units") => Some((
            "aws_organizations_organizational_unit",
            "id",
            true,
            "Workspace organizational units",
        )),
        ("aws_grafana_workspace", "organization_role_name") => {
            Some(("aws_iam_role", "name", false, "Organization role"))
        }
        ("aws_grafana_workspace", "stack_set_name") => Some((
            "aws_cloudformation_stack_set",
            "name",
            false,
            "Workspace StackSet",
        )),
        ("aws_grafana_workspace", "network_access_control.prefix_list_ids") => Some((
            "aws_ec2_managed_prefix_list",
            "id",
            true,
            "Workspace access prefix lists",
        )),
        ("aws_grafana_workspace", "network_access_control.vpce_ids") => {
            Some(("aws_vpc_endpoint", "id", true, "Workspace access endpoints"))
        }
        _ => None,
    }
}

pub(super) fn label(resource: &str, path: &str) -> Option<&'static str> {
    rule(resource, path).map(|r| r.3)
}

fn current(node: &TerraformEntity) -> bool {
    node.provider.is_aws()
        && node.mode == EntityMode::Managed
        && node.action != Action::Delete
        && node.deposed_key.is_none()
}

fn compatible(node: &TerraformEntity, endpoint: &TerraformEntity) -> bool {
    super::eventbridge::bindings(
        node.provider_configuration.as_ref(),
        endpoint.provider_configuration.as_ref(),
        super::scope::scope(node)
            .as_ref()
            .and_then(|s| s.region.as_ref()),
        super::scope::scope(endpoint)
            .as_ref()
            .and_then(|s| s.region.as_ref()),
        matches!(
            endpoint.resource_type.as_str(),
            "aws_iam_role" | "aws_organizations_organizational_unit"
        ),
    )
}

pub(super) fn bindings(raw: &Input, r: &AttributeReference) -> bool {
    r.sources.iter().all(|&source| {
        let node = &raw.nodes[r.target];
        let target = &raw.nodes[source];
        super::eventbridge::bindings(
            node.provider_configuration.as_ref(),
            target.provider_configuration.as_ref(),
            node.entity.scope.as_ref().and_then(|s| s.region.as_ref()),
            target.entity.scope.as_ref().and_then(|s| s.region.as_ref()),
            matches!(
                target.resource_type.as_str(),
                "aws_iam_role" | "aws_organizations_organizational_unit"
            ),
        )
    })
}

fn verify(
    nodes: &[TerraformEntity],
    r: &AttributeReference,
    evidence: &[(&Value, &Value)],
) -> bool {
    let node = &nodes[r.target];
    let Some((kind, field, collection, _)) = rule(&node.resource_type, &r.attribute) else {
        return false;
    };
    if !current(node)
        || !r.complete
        || !r.instances_explicit
        || !r.issues.is_empty()
        || r.sources.is_empty()
        || (!collection && r.sources.len() != 1)
        || r.source_attributes.len() != 1
        || !r.source_attributes.contains(field)
    {
        return false;
    }
    let (value, mask) = evidence[r.target];
    if r.attribute.starts_with("network_access_control.")
        && value["network_access_control"]
            .as_array()
            .is_none_or(|items| items.len() != 1)
    {
        return false;
    }
    let known = |path| super::service_evidence::known(evidence, r.target, path);
    if matches!(
        r.attribute.as_str(),
        "organizational_units" | "organization_role_name" | "stack_set_name"
    ) && known("account_access_type") != Some("ORGANIZATION")
    {
        return false;
    }
    if r.attribute == "role_arn"
        && !matches!(
            known("permission_type"),
            Some("CUSTOMER_MANAGED" | "SERVICE_MANAGED")
        )
    {
        return false;
    }
    let Some(actual) = super::streaming::values(value, mask, &r.attribute, collection) else {
        return false;
    };
    let mut expected = BTreeSet::new();
    for &source in &r.sources {
        let target = &nodes[source];
        if !current(target) || target.resource_type != kind || !compatible(node, target) {
            return false;
        }
        let Some(identifier) = super::service_evidence::known(evidence, source, field) else {
            return false;
        };
        if !expected.insert(identifier) {
            return false;
        }
        if r.attribute == "kms_key_id" && !super::s3_exports::kms_arn(kind, identifier) {
            return false;
        }
        if matches!(
            r.attribute.as_str(),
            "organization_role_name" | "stack_set_name"
        ) {
            let left = node
                .provider_configuration
                .as_ref()
                .and_then(|c| c.account.as_ref());
            let right = target
                .provider_configuration
                .as_ref()
                .and_then(|c| c.account.as_ref());
            if left.is_none() || left != right {
                return false;
            }
        }
        if r.attribute == "network_access_control.vpce_ids" {
            let region = super::scope::scope(node).and_then(|s| s.region);
            let Some(region) = region else {
                return false;
            };
            if super::service_evidence::known(evidence, source, "vpc_endpoint_type")
                != Some("Interface")
                || super::service_evidence::known(evidence, source, "service_name")
                    != Some(format!("com.amazonaws.{}.grafana-workspace", region.0).as_str())
            {
                return false;
            }
        }
    }
    actual == expected
}

pub(super) fn annotate(
    nodes: &[TerraformEntity],
    references: &mut [AttributeReference],
    evidence: &[(&Value, &Value)],
) {
    for r in references {
        if label(&nodes[r.target].resource_type, &r.attribute).is_some() {
            r.service_reference_verified = verify(nodes, r, evidence);
        }
    }
}
