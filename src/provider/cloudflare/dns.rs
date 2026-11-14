use crate::model::{Action, AttributeReference, TerraformEntity};
use serde_json::Value;

pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    match (kind, path) {
        ("cloudflare_dns_record" | "cloudflare_zone_dnssec", "zone_id") => {
            Some("Zone administration")
        }
        ("cloudflare_zone", "account" | "account.id") => Some("Account administration"),
        _ => None,
    }
}

fn current(node: &TerraformEntity) -> bool {
    node.provider.is_cloudflare()
        && node.deposed_key.is_none()
        && node.action != Action::Delete
        && node.metadata.state_removal.is_none()
}

fn verify(
    nodes: &[TerraformEntity],
    refs: &[AttributeReference],
    r: &AttributeReference,
    evidence: &[(&Value, &Value)],
) -> Option<usize> {
    let consumer = &nodes[r.target];
    if !current(consumer)
        || !r.complete
        || !r.instances_explicit
        || !r.issues.is_empty()
        || refs
            .iter()
            .filter(|other| other.target == r.target && other.attribute == r.attribute)
            .count()
            != 1
    {
        return None;
    }
    let [source] = r.sources.as_slice() else {
        return None;
    };
    let endpoint = &nodes[*source];
    let (path, kind) = if consumer.resource_type == "cloudflare_zone" {
        (&["account", "id"][..], "cloudflare_account")
    } else {
        (&["zone_id"][..], "cloudflare_zone")
    };
    if !current(endpoint) || endpoint.resource_type != kind || *source == r.target {
        return None;
    }
    let attributes = r.source_attributes_by_instance.get(source)?;
    if attributes.len() != 1 {
        return None;
    }
    let attribute = attributes.first()?.as_str();
    if attribute != "id"
        && !(attribute == "zone_id"
            && endpoint.mode == crate::model::EntityMode::Data
            && kind == "cloudflare_zone")
    {
        return None;
    }
    let (value, mask) = evidence[r.target];
    let expected = super::known_id(value, mask, path)?;
    let (value, mask) = evidence[*source];
    if super::known_id(value, mask, &[attribute])? != expected
        || super::known_id(value, mask, &["id"])? != expected
        || (kind == "cloudflare_zone"
            && endpoint.mode == crate::model::EntityMode::Data
            && super::known_id(value, mask, &["zone_id"])? != expected)
    {
        return None;
    }
    let candidates = nodes
        .iter()
        .enumerate()
        .filter(|(index, node)| {
            let (value, mask) = evidence[*index];
            current(node)
                && node.resource_type == kind
                && super::known_id(value, mask, &["id"]).as_ref() == Some(&expected)
        })
        .count();
    (candidates == 1).then_some(*source)
}

pub(super) fn annotate(
    nodes: &mut [TerraformEntity],
    refs: &mut [AttributeReference],
    evidence: &[(&Value, &Value)],
) {
    for index in 0..refs.len() {
        let r = &refs[index];
        if !nodes[r.target].provider.is_cloudflare()
            || label(&nodes[r.target].resource_type, &r.attribute).is_none()
        {
            continue;
        }
        let source = verify(nodes, refs, r, evidence);
        refs[index].service_reference_verified = source.is_some();
        if refs[index].attribute == "zone_id" {
            if let Some(source) = source {
                let account = nodes[source]
                    .cloudflare_scope
                    .as_ref()
                    .and_then(|s| s.account.clone());
                if let Some(scope) = &mut nodes[refs[index].target].cloudflare_scope {
                    scope.account = account;
                }
            }
        }
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/cloudflare_dns.rs"]
mod tests;
