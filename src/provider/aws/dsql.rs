use super::{lambda_core, nested_references, service_evidence};
use crate::model::{AttributeReference, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;
const CLUSTER: &str = "aws_dsql_cluster";
const PEERING: &str = "aws_dsql_cluster_peering";
const POLICY: &str = "aws_dsql_cluster_policy";
const PEERS: &str = "multi_region_properties.clusters";

pub(super) fn label(k: &str, p: &str) -> Option<&'static str> {
    match (k, p) {
        (CLUSTER, "kms_encryption_key") => Some("DSQL encryption key"),
        (CLUSTER, PEERS) | (PEERING, "clusters") => Some("DSQL peer clusters"),
        (PEERING | POLICY, "identifier") => Some("DSQL cluster"),
        _ => None,
    }
}
fn fields(_: &str, p: &str, s: &str) -> &'static [&'static str] {
    match (p, s) {
        ("kms_encryption_key", "aws_kms_key") | (PEERS | "clusters", CLUSTER) => &["arn"],
        ("identifier", CLUSTER) => &["identifier"],
        _ => &[],
    }
}

fn peer_scopes(
    left: Option<&crate::model::ProviderConfiguration>,
    right: Option<&crate::model::ProviderConfiguration>,
    region: Option<&crate::model::RegionScope>,
    other: Option<&crate::model::RegionScope>,
) -> bool {
    left.zip(right).is_some_and(|(a, b)| {
        !a.account_conflict && !b.account_conflict && a.account.is_some() && a.account == b.account
    }) && region.zip(other).is_some_and(|(a, b)| a != b)
}
fn cluster_arn<'a>(
    e: &[(&'a Value, &Value)],
    nodes: &[TerraformEntity],
    i: usize,
) -> Option<(&'a str, crate::model::AwsAccountScope)> {
    let arn = service_evidence::known(e, i, "arn")?;
    let parts: Vec<_> = arn.splitn(6, ':').collect();
    let ["arn", partition, "dsql", region, account, resource] = parts.as_slice() else {
        return None;
    };
    let account = crate::model::AwsAccountScope::new(account)?;
    let effective = super::scope::scope(&nodes[i])?.region?;
    (effective.0 == *region
        && !partition.is_empty()
        && resource.strip_prefix("cluster/") == Some(service_evidence::known(e, i, "identifier")?))
    .then_some((*partition, account))
}
pub(super) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    e: &[(&Value, &Value)],
) {
    let verified: Vec<_> = refs
        .iter()
        .map(|r| {
            let kind = nodes[r.target].resource_type.as_str();
            label(kind, &r.attribute)?;
            let peers = matches!(r.attribute.as_str(), PEERS | "clusters");
            let local = if kind == PEERING && peers {
                lambda_core::unique(refs, r.target, "identifier").and_then(|owner| {
                    nested_references::verify(nodes, refs, e, owner, fields, |_, _| true)?;
                    let [s] = owner.sources.as_slice() else {
                        return None;
                    };
                    service_evidence::eligible(nodes, owner).then_some(*s)
                })
            } else {
                Some(r.target)
            };
            Some(
                nested_references::verify(nodes, refs, e, r, fields, |s, path| {
                    let value = nested_references::value(e[r.target].0, e[r.target].1, path);
                    if peers {
                        value.is_some_and(Value::is_array)
                            && local.is_some_and(|owner| {
                                super::scope::scope(&nodes[owner])
                                    .and_then(|s| s.region)
                                    .zip(super::scope::scope(&nodes[s]).and_then(|s| s.region))
                                    .is_some_and(|(a, b)| a != b)
                                    && cluster_arn(e, nodes, owner)
                                        .zip(cluster_arn(e, nodes, s))
                                        .is_some_and(|(a, b)| a == b)
                            })
                    } else {
                        value
                            .and_then(Value::as_str)
                            .is_some_and(|v| v != "AWS_OWNED_KMS_KEY")
                            && service_evidence::eligible(nodes, r)
                    }
                })
                .is_some(),
            )
        })
        .collect();
    for (r, v) in refs.iter_mut().zip(verified) {
        if let Some(v) = v {
            r.service_reference_verified = v;
            if v && matches!(r.attribute.as_str(), PEERS | "clusters") {
                r.service_reference_owner_account = r
                    .sources
                    .first()
                    .and_then(|&s| cluster_arn(e, nodes, s))
                    .map(|(_, a)| a);
            }
            if v {
                r.complete = true;
            }
        }
    }
}
pub(super) fn bindings(raw: &Input, r: &AttributeReference) -> bool {
    let k = &raw.nodes[r.target].resource_type;
    if label(k, &r.attribute).is_none() {
        return true;
    }
    if matches!(r.attribute.as_str(), PEERS | "clusters") {
        if r.service_reference_owner_account
            .as_ref()
            .is_none_or(|account| {
                raw.nodes[r.target]
                    .provider_configuration
                    .as_ref()
                    .and_then(|c| c.account.as_ref())
                    != Some(account)
            })
        {
            return false;
        }
        if k == PEERING
            && !lambda_core::unique(&raw.attributes, r.target, "identifier").is_some_and(|local| {
                local.service_reference_verified && lambda_core::bindings(raw, local)
            })
        {
            return false;
        }
        r.sources.iter().all(|&s| {
            peer_scopes(
                raw.nodes[r.target].provider_configuration.as_ref(),
                raw.nodes[s].provider_configuration.as_ref(),
                raw.nodes[r.target]
                    .entity
                    .scope
                    .as_ref()
                    .and_then(|s| s.region.as_ref()),
                raw.nodes[s]
                    .entity
                    .scope
                    .as_ref()
                    .and_then(|s| s.region.as_ref()),
            )
        })
    } else {
        lambda_core::bindings(raw, r)
    }
}
#[cfg(test)]
#[path = "../../../tests/unit/semantic/dsql.rs"]
mod tests;
