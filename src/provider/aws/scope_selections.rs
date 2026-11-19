use super::nested_references::value;
use crate::model::{
    Action, AwsAccountScope, EntityMode, Graph, ReferenceKind, ResourceReference, TerraformEntity,
    TerraformPlan,
};
use serde_json::Value;

pub(super) fn region(s: &str) -> Option<&str> {
    let mut parts = s.split('-');
    let country = parts.next()?;
    let number = parts.next_back()?;
    let segments: Vec<_> = parts.collect();
    (country.len() == 2
        && country.bytes().all(|b| b.is_ascii_lowercase())
        && !segments.is_empty()
        && segments.len() <= 3
        && segments
            .iter()
            .all(|p| !p.is_empty() && p.len() <= 12 && p.bytes().all(|b| b.is_ascii_lowercase()))
        && !number.is_empty()
        && number.len() <= 2
        && number.bytes().all(|b| b.is_ascii_digit()))
    .then_some(s)
}

type Details = Vec<(&'static str, &'static str, String)>;

fn selections(kind: &str, v: &Value, mask: &Value) -> Details {
    let mut details = Vec::new();
    match kind {
        "aws_auditmanager_assessment_delegation" => {
            if let Some((assessment, control)) = value(v, mask, "assessment_id")
                .and_then(Value::as_str).filter(|v| !v.is_empty())
                .zip(value(v, mask, "control_set_id").and_then(Value::as_str).filter(|v| !v.is_empty())) {
                details.push(("control_set_id", "Assessment control set", format!("{assessment} / {control}")));
            }
        }
        "aws_auditmanager_framework_share" => {
            if let Some((account, region)) = value(v, mask, "destination_account")
                .and_then(Value::as_str)
                .and_then(AwsAccountScope::new)
                .zip(
                    value(v, mask, "destination_region")
                        .and_then(Value::as_str)
                        .and_then(region),
                )
            {
                details.push((
                    "destination_region",
                    "Framework share destination",
                    format!("{} / {region}", account.as_str()),
                ));
            }
        }
        _ => {}
    }
    details
}

pub(crate) fn enrich(nodes: &mut [TerraformEntity], values: &[(&Value, &Value)]) {
    for (node, &(v, mask)) in nodes.iter_mut().zip(values) {
        if node.provider.is_aws()
            && node.mode == EntityMode::Managed
            && node.action != Action::Delete
            && node.deposed_key.is_none()
        {
            node.configured_scopes = selections(&node.resource_type, v, mask);
        }
    }
}
pub(crate) fn facts(plan: &TerraformPlan, graph: &mut Graph) {
    for (fact, node) in plan.nodes.iter().zip(&mut graph.nodes) {
        for &(attribute, label, ref value) in &fact.configured_scopes {
            node.references.push(ResourceReference {
                kind: ReferenceKind::ConfiguredScope(value.clone()),
                preserve_spatial: false,
                show_spatial: true,
                label,
                attribute: attribute.into(),
                targets: Vec::new(),
                complete: true,
                dependency: false,
                issues: Default::default(),
            });
        }
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/semantic/scope_selections.rs"]
mod tests;
