use crate::model::{
    AttributeReference, AwsAccountScope, Edge, EdgeKind, Graph, RegionScope, TerraformEntity,
};
use crate::semantic::{Input, relationships};
use serde_json::Value;
#[cfg(test)]
#[path = "../../../tests/unit/semantic/directory_service.rs"]
mod tests;
const DIRECTORY: &str = "aws_directory_service_directory";
const REGION: &str = "aws_directory_service_region";
const SHARED: &str = "aws_directory_service_shared_directory";
const ACCEPT: &str = "aws_directory_service_shared_directory_accepter";
const LOG: &str = "aws_directory_service_log_subscription";
const HELPERS: &[&str] = &[
    "aws_directory_service_conditional_forwarder",
    "aws_directory_service_directory_setting",
    "aws_directory_service_ip_route",
    "aws_directory_service_ip_routes_exclusive",
    "aws_directory_service_radius_settings",
    "aws_directory_service_trust",
    LOG,
    REGION,
    "aws_directoryservicedata_group",
    "aws_directoryservicedata_user",
];
const VPC: &str = "vpc_settings.vpc_id";
const SUBNETS: &str = "vpc_settings.subnet_ids";
const CONNECT_VPC: &str = "connect_settings.vpc_id";
const CONNECT_SUBNETS: &str = "connect_settings.subnet_ids";
pub(crate) fn resource(k: &str) -> bool {
    matches!(k, DIRECTORY | SHARED | ACCEPT) || HELPERS.contains(&k)
}
pub(super) fn owner(k: &str) -> Option<(&'static str, &'static str, &'static str)> {
    HELPERS.contains(&k).then_some((
        "directory_id",
        "Directory owner",
        if matches!(
            k,
            "aws_directoryservicedata_group" | "aws_directoryservicedata_user"
        ) {
            "Directory identities"
        } else if k == REGION {
            "Additional regions"
        } else {
            "Directory configuration"
        },
    ))
}
fn canonical(p: &str) -> String {
    p.split('.')
        .filter(|s| s.parse::<usize>().is_err())
        .collect::<Vec<_>>()
        .join(".")
}
pub(crate) fn indexed(k: &str, p: &str) -> bool {
    p.split('.').any(|s| s.parse::<usize>().is_ok()) && label(k, &canonical(p)).is_some()
}
pub(super) fn label(k: &str, p: &str) -> Option<&'static str> {
    match (k, p) {
        (DIRECTORY | REGION, VPC) | (DIRECTORY, CONNECT_VPC) => Some("Directory VPC"),
        (DIRECTORY | REGION, SUBNETS) | (DIRECTORY, CONNECT_SUBNETS) => Some("Directory subnets"),
        (SHARED, "directory_id") => Some("Shared directory"),
        (SHARED, "target.id") => Some("Directory recipient account"),
        (ACCEPT, "shared_directory_id") => Some("Directory share source"),
        (LOG, "log_group_name") => Some("Directory log group"),
        (k, "directory_id") if HELPERS.contains(&k) => Some("Directory owner"),
        _ => None,
    }
}
fn fields(k: &str, p: &str, s: &str) -> &'static [&'static str] {
    match (k, p, s) {
        (DIRECTORY | REGION, VPC | CONNECT_VPC, "aws_vpc")
        | (DIRECTORY | REGION, SUBNETS | CONNECT_SUBNETS, "aws_subnet")
        | (SHARED, "target.id", "aws_organizations_account") => &["id"],
        (ACCEPT, "shared_directory_id", SHARED) => &["shared_directory_id"],
        (LOG, "log_group_name", "aws_cloudwatch_log_group") => &["name"],
        (k, "directory_id", DIRECTORY) if k == SHARED || HELPERS.contains(&k) => &["id"],
        _ => &[],
    }
}
fn shape(v: &Value, p: &str) -> bool {
    if let Some((block, leaf)) = p.split_once('.') {
        v[block].as_array().is_some_and(|a| {
            a.len() == 1
                && if leaf == "subnet_ids" {
                    a[0][leaf]
                        .as_array()
                        .is_some_and(|a| !a.is_empty() && a.iter().all(Value::is_string))
                } else {
                    a[0][leaf].is_string()
                }
        })
    } else {
        v[p].is_string()
    }
}
fn network_paths(k: &str, v: &Value, m: &Value) -> Option<(&'static str, &'static str)> {
    if k == REGION {
        return Some((VPC, SUBNETS));
    }
    if k != DIRECTORY {
        return None;
    }
    let known = |p| super::nested_references::value(v, m, p);
    let (paths, other) = match known("type")?.as_str()? {
        "SimpleAD" | "MicrosoftAD" => ((VPC, SUBNETS), "connect_settings"),
        "ADConnector" => ((CONNECT_VPC, CONNECT_SUBNETS), "vpc_settings"),
        _ => return None,
    };
    known(other)
        .is_some_and(|v| v.is_null() || v.as_array().is_some_and(Vec::is_empty))
        .then_some(paths)
}
fn recipient(v: &Value, m: &Value) -> Option<AwsAccountScope> {
    if v["target"].as_array().is_none_or(|a| a.len() != 1)
        || super::nested_references::value(v, m, "target.0.type")?.as_str() != Some("ACCOUNT")
    {
        return None;
    }
    AwsAccountScope::new(super::nested_references::value(v, m, "target.0.id")?.as_str()?)
}
pub(super) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    e: &[(&Value, &Value)],
) {
    let results: Vec<_> = refs
        .iter()
        .map(|r| {
            let k = nodes[r.target].resource_type.as_str();
            label(k, &r.attribute)?;
            let (v, m) = e[r.target];
            let region = if k == REGION && matches!(r.attribute.as_str(), VPC | SUBNETS) {
                super::service_evidence::known(e, r.target, "region_name")
                    .map(|s| RegionScope(s.into()))
            } else {
                None
            };
            let valid = |s: usize, _path: &str| {
                if matches!(
                    r.attribute.as_str(),
                    VPC | SUBNETS | CONNECT_VPC | CONNECT_SUBNETS
                ) {
                    return network_paths(k, v, m)
                        .is_some_and(|(vp, sp)| r.attribute == vp || r.attribute == sp)
                        && (k != REGION || region.is_some());
                }
                if k == SHARED && r.attribute == "target.id" {
                    return recipient(v, m).is_some();
                }
                if k == ACCEPT {
                    let (v, m) = e[s];
                    return recipient(v, m).is_some();
                }
                if matches!(k, REGION | SHARED) && r.attribute == "directory_id" {
                    return super::service_evidence::known(e, s, "type") == Some("MicrosoftAD");
                }
                true
            };
            let ok = shape(v, &r.attribute)
                && super::nested_references::verify(nodes, refs, e, r, fields, valid).is_some();
            let account = if k == ACCEPT {
                r.sources.first().and_then(|&s| {
                    let (v, m) = e[s];
                    recipient(v, m)
                })
            } else {
                None
            };
            Some((ok, region, account))
        })
        .collect();
    for (r, result) in refs.iter_mut().zip(results) {
        if let Some((ok, region, account)) = result {
            r.service_reference_verified = ok;
            if ok {
                r.complete = true;
            }
            r.service_reference_region = region;
            r.service_reference_owner_account = account;
        }
    }
    let contexts: Vec<_> = refs
        .iter()
        .map(|r| {
            let (v, m) = e[r.target];
            let (vp, sp) = network_paths(&nodes[r.target].resource_type, v, m)?;
            if r.attribute != vp && r.attribute != sp {
                return None;
            }
            let proof = |p| {
                super::lambda_core::unique(refs, r.target, p)
                    .filter(|r| r.service_reference_verified)
            };
            let (vpc, subnets) = proof(vp).zip(proof(sp))?;
            let [parent] = vpc.sources.as_slice() else {
                return None;
            };
            subnets
                .sources
                .iter()
                .all(|&s| super::lambda_core::parent(nodes, refs, e, s) == Some(*parent))
                .then_some(*parent)
        })
        .collect();
    for (r, c) in refs.iter_mut().zip(contexts) {
        if let Some(c) = c {
            r.endpoint_network_context = vec![c];
        }
    }
}
pub(super) fn bindings(raw: &Input, r: &AttributeReference) -> bool {
    let l = &raw.nodes[r.target];
    let k = l.resource_type.as_str();
    if label(k, &r.attribute).is_none() {
        return true;
    }
    let own_region = l.entity.scope.as_ref().and_then(|s| s.region.as_ref());
    if own_region.is_none() {
        return false;
    }
    if k == ACCEPT {
        let [s] = r.sources.as_slice() else {
            return false;
        };
        let right = &raw.nodes[*s];
        let (Some(left), Some(right_binding), Some(recipient)) = (
            l.provider_configuration.as_ref(),
            right.provider_configuration.as_ref(),
            r.service_reference_owner_account.as_ref(),
        ) else {
            return false;
        };
        return !left.account_conflict
            && !right_binding.account_conflict
            && left.account.as_ref() == Some(recipient)
            && right_binding
                .account
                .as_ref()
                .is_some_and(|owner| owner != recipient)
            && own_region == right.entity.scope.as_ref().and_then(|s| s.region.as_ref());
    }
    let region = if k == REGION && matches!(r.attribute.as_str(), VPC | SUBNETS) {
        r.service_reference_region.as_ref()
    } else {
        own_region
    };
    r.sources
        .iter()
        .chain(&r.endpoint_network_context)
        .all(|&s| {
            let right = &raw.nodes[s];
            super::eventbridge::bindings(
                l.provider_configuration.as_ref(),
                right.provider_configuration.as_ref(),
                region,
                right.entity.scope.as_ref().and_then(|s| s.region.as_ref()),
                right.resource_type == "aws_organizations_account",
            )
        })
}
pub(super) fn present(raw: &Input, g: &mut Graph) {
    for (target, n) in raw
        .nodes
        .iter()
        .enumerate()
        .filter(|(_, n)| matches!(n.resource_type.as_str(), DIRECTORY | REGION))
    {
        for (vp, sp) in [(VPC, SUBNETS), (CONNECT_VPC, CONNECT_SUBNETS)] {
            if n.resource_type == REGION && vp == CONNECT_VPC {
                continue;
            }
            let proof = |p| {
                super::lambda_core::unique(&raw.attributes, target, p)
                    .filter(|r| super::service_references::verified(raw, r))
            };
            let Some((vpc, subnets)) = proof(vp).zip(proof(sp)) else {
                continue;
            };
            let [parent] = vpc.endpoint_network_context.as_slice() else {
                continue;
            };
            if subnets.endpoint_network_context != [*parent]
                || !subnets
                    .sources
                    .iter()
                    .all(|&s| super::streaming::vpc(raw, s) == Some(*parent))
            {
                continue;
            }
            let mut evidence = vec![relationships::reference(g, *parent, target)];
            for &s in &subnets.sources {
                evidence.extend([
                    relationships::reference(g, *parent, s),
                    relationships::reference(g, s, target),
                ]);
            }
            relationships::record(g, *parent, target, EdgeKind::Containment, evidence);
            g.edges.push(Edge {
                kind: EdgeKind::Containment,
                ..Edge::from((*parent, target))
            });
        }
    }
}
