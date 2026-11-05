use crate::model::{Action, AttributeReference, AwsAccountScope, EntityMode, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/search_connections.rs"]
mod tests;

const OUTBOUND: &str = "aws_opensearch_outbound_connection";
const INBOUND: &str = "aws_opensearch_inbound_connection_accepter";
const LOCAL: &str = "local_domain_info.domain_name";
const REMOTE: &str = "remote_domain_info.domain_name";

pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    match (kind, path) {
        (OUTBOUND, LOCAL) => Some("Local search domain"),
        (OUTBOUND, REMOTE) => Some("Remote search domain"),
        (INBOUND, "connection_id") => Some("Accepted outbound connection"),
        _ => None,
    }
}

fn current(n: &TerraformEntity) -> bool {
    n.provider.is_aws()
        && n.mode == EntityMode::Managed
        && n.action != Action::Delete
        && n.deposed_key.is_none()
}

fn side<'a>(value: &'a Value, mask: &Value, block: &str, field: &str) -> Option<&'a str> {
    if !matches!(mask, Value::Null | Value::Bool(false) | Value::Object(_)) {
        return None;
    }
    let [row] = value[block].as_array()?.as_slice() else {
        return None;
    };
    let unknown = match &mask[block] {
        Value::Null | Value::Bool(false) => &Value::Null,
        Value::Array(rows) if rows.len() == 1 => &rows[0],
        _ => return None,
    };
    super::service_evidence::known(&[(row, unknown)], 0, field)
}

pub(crate) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    evidence: &[(&Value, &Value)],
) {
    let mut counts = std::collections::BTreeMap::new();
    for r in refs.iter() {
        *counts.entry((r.target, r.attribute.clone())).or_insert(0) += 1;
    }
    for r in refs {
        let target = &nodes[r.target];
        if label(&target.resource_type, &r.attribute).is_none() {
            continue;
        }
        r.service_reference_verified = false;
        r.service_reference_owner_account = None;
        let [s] = r.sources.as_slice() else { continue };
        let source = &nodes[*s];
        if counts[&(r.target, r.attribute.clone())] != 1
            || !current(target)
            || !current(source)
            || !r.complete
            || !r.instances_explicit
            || !r.issues.is_empty()
        {
            continue;
        }
        let Some(attrs) = r.source_attributes_by_instance.get(s) else {
            continue;
        };
        let output = if target.resource_type == OUTBOUND {
            "domain_name"
        } else {
            "id"
        };
        if attrs.len() != 1 || !attrs.contains(output) {
            continue;
        }
        if target.resource_type == INBOUND {
            r.service_reference_verified = source.resource_type == OUTBOUND
                && super::service_evidence::known(evidence, r.target, "connection_id").is_some()
                && super::service_evidence::known(evidence, r.target, "connection_id")
                    == super::service_evidence::known(evidence, *s, "id");
            continue;
        }
        if source.resource_type != "aws_opensearch_domain"
            || !matches!(
                super::service_evidence::known(evidence, r.target, "connection_mode"),
                Some("DIRECT" | "VPC_ENDPOINT")
            )
        {
            continue;
        }
        let (v, m) = evidence[r.target];
        let block = r.attribute.split('.').next().unwrap_or_default();
        let Some(owner) = side(v, m, block, "owner_id").and_then(AwsAccountScope::new) else {
            continue;
        };
        let region = super::scope::scope(source).and_then(|s| s.region);
        let actual = side(v, m, block, "domain_name");
        if actual.is_none()
            || actual != super::service_evidence::known(evidence, *s, "domain_name")
            || region
                .as_ref()
                .is_none_or(|r| Some(r.0.as_str()) != side(v, m, block, "region"))
        {
            continue;
        }
        r.service_reference_owner_account = Some(owner);
        r.service_reference_verified = true;
    }
}

fn proof<'a>(raw: &'a Input, target: usize, path: &str) -> Option<&'a AttributeReference> {
    let mut found = raw
        .attributes
        .iter()
        .filter(|r| r.target == target && r.attribute == path);
    let r = found.next()?;
    (found.next().is_none() && r.service_reference_verified).then_some(r)
}

pub(super) fn bindings(raw: &Input, r: &AttributeReference) -> bool {
    let [s] = r.sources.as_slice() else {
        return false;
    };
    let account = |i: usize| {
        raw.nodes[i]
            .provider_configuration
            .as_ref()
            .filter(|p| !p.account_conflict)
            .and_then(|p| p.account.as_ref())
    };
    let region = |i: usize| {
        raw.nodes[i]
            .entity
            .scope
            .as_ref()
            .and_then(|s| s.region.as_ref())
    };
    if raw.nodes[r.target].resource_type == OUTBOUND {
        let Some(owner) = r.service_reference_owner_account.as_ref() else {
            return false;
        };
        if account(*s) != Some(owner) {
            return false;
        }
        if r.attribute == LOCAL {
            return account(r.target) == Some(owner)
                && region(r.target).is_some()
                && region(r.target) == region(*s);
        }
        return proof(raw, r.target, LOCAL).is_some_and(|local| bindings(raw, local));
    }
    let Some((local, remote)) = proof(raw, *s, LOCAL).zip(proof(raw, *s, REMOTE)) else {
        return false;
    };
    if !bindings(raw, local) || !bindings(raw, remote) {
        return false;
    }
    let [remote] = remote.sources.as_slice() else {
        return false;
    };
    account(r.target).is_some()
        && account(r.target) == account(*remote)
        && region(r.target).is_some()
        && region(r.target) == region(*remote)
}
