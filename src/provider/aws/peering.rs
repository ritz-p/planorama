use crate::model::{Action, AttributeReference, EntityMode, TerraformEntity};
use serde_json::Value;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/peering.rs"]
mod tests;

pub(super) fn label(kind: &str, field: &str) -> Option<&'static str> {
    match (kind, field) {
        ("aws_vpc_peering_connection", "vpc_id") => Some("Requester VPC"),
        ("aws_vpc_peering_connection", "peer_vpc_id") => Some("Accepter VPC"),
        (
            "aws_vpc_peering_connection_accepter" | "aws_vpc_peering_connection_options",
            "vpc_peering_connection_id",
        ) => Some("Peering connection"),
        _ => None,
    }
}

pub(super) fn bindings(raw: &crate::semantic::Input, r: &AttributeReference) -> bool {
    let Some(proof) = &r.peering_proof else {
        return false;
    };
    let [source] = r.sources.as_slice() else {
        return false;
    };
    let node = &raw.nodes[r.target];
    let endpoint = &raw.nodes[*source];
    let same = |a: usize, b: usize| {
        super::eventbridge::bindings(
            raw.nodes[a].provider_configuration.as_ref(),
            raw.nodes[b].provider_configuration.as_ref(),
            raw.nodes[a]
                .entity
                .scope
                .as_ref()
                .and_then(|s| s.region.as_ref()),
            raw.nodes[b]
                .entity
                .scope
                .as_ref()
                .and_then(|s| s.region.as_ref()),
            false,
        )
    };
    let account = |i: usize| {
        raw.nodes[i]
            .provider_configuration
            .as_ref()
            .filter(|c| !c.account_conflict)
            .and_then(|c| c.account.as_ref())
    };
    if node.resource_type == "aws_vpc_peering_connection" {
        if r.attribute == "vpc_id" {
            return same(r.target, *source);
        }
        let (Some(left), Some(right)) = (
            &node.provider_configuration,
            &endpoint.provider_configuration,
        ) else {
            return false;
        };
        if left.account_conflict || right.account_conflict {
            return false;
        }
        return if let Some(expected) = &proof.peer_account {
            account(r.target).is_some() && account(*source) == Some(expected)
        } else {
            match (account(r.target), account(*source)) {
                (Some(a), Some(b)) => a == b,
                (None, None) => left == right,
                _ => false,
            }
        };
    }
    let side = |field| {
        let mut found = raw
            .attributes
            .iter()
            .filter(|a| a.target == *source && a.attribute == field);
        let a = found.next()?;
        if found.next().is_some() || !super::service_references::verified(raw, a) {
            return None;
        }
        match a.sources.as_slice() {
            [s] => Some(*s),
            _ => None,
        }
    };
    let (Some(requester), Some(accepter)) = (side("vpc_id"), side("peer_vpc_id")) else {
        return false;
    };
    if requester == accepter {
        return false;
    }
    if node.resource_type == "aws_vpc_peering_connection_accepter" {
        if !same(r.target, accepter) {
            return false;
        }
        let cross = account(requester) != account(accepter);
        let other = if cross { requester } else { accepter };
        (if cross {
            proof.accepter_orientation
        } else {
            proof.requester_orientation
        }) && proof
            .helper_peer_account
            .as_ref()
            .is_none_or(|a| account(other) == Some(a))
    } else {
        (!proof.requester_options || same(r.target, requester))
            && (!proof.accepter_options || same(r.target, accepter))
    }
}

fn current(n: &TerraformEntity) -> bool {
    n.provider.is_aws()
        && n.mode == EntityMode::Managed
        && n.action != Action::Delete
        && n.deposed_key.is_none()
}

fn same(left: &TerraformEntity, right: &TerraformEntity) -> bool {
    let region = super::scope::scope(left).and_then(|s| s.region);
    region.is_some()
        && region == super::scope::scope(right).and_then(|s| s.region)
        && [&left.provider_configuration, &right.provider_configuration]
            .iter()
            .all(|c| c.as_ref().is_some_and(|c| !c.account_conflict))
}

fn peer(
    nodes: &[TerraformEntity],
    evidence: &[(&Value, &Value)],
    connection: usize,
    endpoint: usize,
) -> bool {
    let (Some(left), Some(right)) = (
        &nodes[connection].provider_configuration,
        &nodes[endpoint].provider_configuration,
    ) else {
        return false;
    };
    if left.account_conflict || right.account_conflict {
        return false;
    }
    let known = |key| super::service_evidence::known(evidence, connection, key);
    let (value, unknown) = evidence[connection];
    let account = if let Some(owner) = known("peer_owner_id") {
        crate::model::AwsAccountScope::new(owner).is_some()
    } else {
        super::vpc_gateways::absent(value, unknown, "peer_owner_id")
    };
    let lr = super::scope::scope(&nodes[connection]).and_then(|s| s.region);
    let rr = super::scope::scope(&nodes[endpoint]).and_then(|s| s.region);
    let region = if let Some(region) = known("peer_region") {
        rr.as_ref().is_some_and(|r| r.0 == region)
    } else {
        super::vpc_gateways::absent(value, unknown, "peer_region") && lr == rr
    };
    account && lr.is_some() && rr.is_some() && region
}

fn endpoint(
    nodes: &[TerraformEntity],
    r: &AttributeReference,
    evidence: &[(&Value, &Value)],
) -> Option<usize> {
    let [source] = r.sources.as_slice() else {
        return None;
    };
    if *source == r.target
        || !current(&nodes[r.target])
        || !current(&nodes[*source])
        || !r.complete
        || !r.instances_explicit
        || !r.issues.is_empty()
        || !r.ids_only
    {
        return None;
    }
    let known = |i, key| super::service_evidence::known(evidence, i, key);
    (known(r.target, &r.attribute).is_some()
        && known(r.target, &r.attribute) == known(*source, "id"))
    .then_some(*source)
}

fn vpc(
    nodes: &[TerraformEntity],
    r: &AttributeReference,
    evidence: &[(&Value, &Value)],
) -> Option<usize> {
    let source = endpoint(nodes, r, evidence)?;
    if nodes[r.target].resource_type != "aws_vpc_peering_connection"
        || nodes[source].resource_type != "aws_vpc"
    {
        return None;
    }
    match r.attribute.as_str() {
        "vpc_id" if same(&nodes[r.target], &nodes[source]) => Some(source),
        "peer_vpc_id" if peer(nodes, evidence, r.target, source) => Some(source),
        _ => None,
    }
}

fn side(
    nodes: &[TerraformEntity],
    refs: &[AttributeReference],
    evidence: &[(&Value, &Value)],
    target: usize,
    field: &str,
) -> Option<usize> {
    let mut found = refs
        .iter()
        .filter(|r| r.target == target && r.attribute == field);
    let r = found.next()?;
    if found.next().is_some() {
        return None;
    }
    vpc(nodes, r, evidence)
}

fn option(value: &Value, unknown: &Value, field: &str) -> Option<bool> {
    if super::vpc_gateways::absent(value, unknown, field) {
        return Some(false);
    }
    let [entry] = value[field].as_array()?.as_slice() else {
        return None;
    };
    let mask = match &unknown[field] {
        Value::Null | Value::Bool(false) => &Value::Null,
        Value::Array(m) if m.len() == 1 => &m[0],
        _ => return None,
    };
    (matches!(mask, Value::Null | Value::Bool(false) | Value::Object(_))
        && matches!(
            &mask["allow_remote_vpc_dns_resolution"],
            Value::Null | Value::Bool(false)
        )
        && entry["allow_remote_vpc_dns_resolution"].is_boolean())
    .then_some(true)
}

pub(crate) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    evidence: &[(&Value, &Value)],
) {
    let results: Vec<_> =
        refs.iter()
            .map(|r| -> Option<crate::model::PeeringProof> {
                label(&nodes[r.target].resource_type, &r.attribute)?;
                let known = |i, key| super::service_evidence::known(evidence, i, key);
                let mut proof = crate::model::PeeringProof::default();
                if nodes[r.target].resource_type == "aws_vpc_peering_connection" {
                    vpc(nodes, r, evidence)?;
                    proof.peer_account = known(r.target, "peer_owner_id")
                        .and_then(crate::model::AwsAccountScope::new);
                    return Some(proof);
                }
                let connection = endpoint(nodes, r, evidence)?;
                if nodes[connection].resource_type != "aws_vpc_peering_connection" {
                    return None;
                }
                let requester = side(nodes, refs, evidence, connection, "vpc_id")?;
                let accepter = side(nodes, refs, evidence, connection, "peer_vpc_id")?;
                if requester == accepter {
                    return None;
                }
                let (value, unknown) = evidence[r.target];
                if nodes[r.target].resource_type == "aws_vpc_peering_connection_accepter" {
                    if !same(&nodes[r.target], &nodes[accepter]) {
                        return None;
                    }
                    let agrees = |field, expected| {
                        super::vpc_gateways::absent(value, unknown, field)
                            || (known(r.target, field).is_some()
                                && known(r.target, field) == known(expected, "id"))
                    };
                    proof.requester_orientation =
                        agrees("vpc_id", requester) && agrees("peer_vpc_id", accepter);
                    proof.accepter_orientation =
                        agrees("vpc_id", accepter) && agrees("peer_vpc_id", requester);
                    if !proof.requester_orientation && !proof.accepter_orientation {
                        return None;
                    }
                    if !super::vpc_gateways::absent(value, unknown, "peer_owner_id") {
                        proof.helper_peer_account = Some(crate::model::AwsAccountScope::new(
                            known(r.target, "peer_owner_id")?,
                        )?);
                    }
                    if !super::vpc_gateways::absent(value, unknown, "peer_region")
                        && known(r.target, "peer_region")
                            != super::scope::scope(&nodes[accepter])
                                .and_then(|s| s.region)
                                .as_ref()
                                .map(|r| r.0.as_str())
                    {
                        return None;
                    }
                } else {
                    let configured = |field: &str| {
                        refs.iter()
                            .any(|a| a.target == r.target && a.attribute == field)
                    };
                    proof.requester_options = configured("requester");
                    proof.accepter_options = configured("accepter");
                    if (!proof.requester_options && !proof.accepter_options)
                        || (proof.requester_options
                            && (option(value, unknown, "requester") != Some(true)
                                || !same(&nodes[r.target], &nodes[requester])))
                        || (proof.accepter_options
                            && (option(value, unknown, "accepter") != Some(true)
                                || !same(&nodes[r.target], &nodes[accepter])))
                    {
                        return None;
                    }
                }
                Some(proof)
            })
            .collect();
    for (r, proof) in refs.iter_mut().zip(results) {
        if label(&nodes[r.target].resource_type, &r.attribute).is_some() {
            r.service_reference_verified = proof.is_some();
            r.peering_proof = proof;
        }
    }
}
