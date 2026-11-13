use crate::model::{
    Action, AttributeReference, AwsAccountScope, Edge, EdgeKind, EntityMode, Graph, TerraformEntity,
};
use crate::semantic::{Input, relationships};
use serde_json::Value;
#[cfg(test)]
#[path = "../../../tests/unit/semantic/networkfirewall_extended.rs"]
mod tests;
const FIREWALL: &str = "aws_networkfirewall_firewall";
const ASSOCIATION: &str = "aws_networkfirewall_vpc_endpoint_association";
const ACCEPTER: &str = "aws_networkfirewall_firewall_transit_gateway_attachment_accepter";
const POLICY: &str = "aws_networkfirewall_firewall_policy";
const TLS: &str = "aws_networkfirewall_tls_inspection_configuration";
const RESOURCE_POLICY: &str = "aws_networkfirewall_resource_policy";
const CONTAINER: &str = "aws_networkfirewall_container_association";
const GROUP: &str = "aws_networkfirewall_rule_group";
const SUBNET: &str = "subnet_mapping.subnet_id";
const TLS_PATH: &str = "firewall_policy.tls_inspection_configuration_arn";
const CA: &str =
    "tls_inspection_configuration.server_certificate_configuration.certificate_authority_arn";
const CERT: &str =
    "tls_inspection_configuration.server_certificate_configuration.server_certificate.resource_arn";
const CLUSTER: &str = "container_monitoring_configuration.cluster_arn";
const IPSET: &str = "rule_group.reference_sets.ip_set_references.ip_set_reference.reference_arn";
const ATTACHMENT: &str = "firewall_status.0.transit_gateway_attachment_sync_states.0.attachment_id";
const MANY: &[&str] = &[
    "tls_inspection_configuration.server_certificate_configuration.server_certificate",
    "container_monitoring_configuration",
    "rule_group.reference_sets.ip_set_references",
    "rule_group.reference_sets.ip_set_references.ip_set_reference",
];
pub(crate) fn resource(k: &str) -> bool {
    matches!(
        k,
        FIREWALL | ASSOCIATION | ACCEPTER | POLICY | TLS | RESOURCE_POLICY | CONTAINER | GROUP
    )
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
        (FIREWALL, "transit_gateway_id") => Some("Firewall transit gateway"),
        (ASSOCIATION, "firewall_arn") => Some("Associated firewall"),
        (ASSOCIATION, "vpc_id") => Some("Endpoint association VPC"),
        (ASSOCIATION, SUBNET) => Some("Endpoint association subnet"),
        (ACCEPTER, "transit_gateway_attachment_id") => Some("Firewall attachment source"),
        (POLICY, TLS_PATH) => Some("TLS inspection configuration"),
        (TLS, CA) => Some("Inspection CA certificate"),
        (TLS, CERT) => Some("Inspection server certificates"),
        (RESOURCE_POLICY, "resource_arn") => Some("Firewall policy target"),
        (CONTAINER, CLUSTER) => Some("Monitored container clusters"),
        (GROUP, IPSET) => Some("Firewall IP set resources"),
        _ => None,
    }
}
fn fields(k: &str, p: &str, s: &str) -> &'static [&'static str] {
    match (k, p, s) {
        (FIREWALL, "transit_gateway_id", "aws_ec2_transit_gateway")
        | (ASSOCIATION, "vpc_id", "aws_vpc")
        | (ASSOCIATION, SUBNET, "aws_subnet") => &["id"],
        (ASSOCIATION, "firewall_arn", FIREWALL)
        | (POLICY, TLS_PATH, TLS)
        | (TLS, CA | CERT, "aws_acm_certificate")
        | (RESOURCE_POLICY, "resource_arn", POLICY | GROUP)
        | (CONTAINER, CLUSTER, "aws_ecs_cluster" | "aws_eks_cluster")
        | (GROUP, IPSET, "aws_ec2_managed_prefix_list" | "aws_resourcegroups_group") => &["arn"],
        (GROUP, IPSET, CONTAINER) => &["container_association_arn"],
        _ => &[],
    }
}
fn shape(v: &Value, p: &str, prefix: &str) -> bool {
    let (field, rest) = p.split_once('.').map_or((p, None), |(a, b)| (a, Some(b)));
    let v = &v[field];
    if v.is_null() {
        return true;
    }
    let Some(rest) = rest else {
        return v.is_string();
    };
    let prefix = if prefix.is_empty() {
        field.into()
    } else {
        format!("{prefix}.{field}")
    };
    v.as_array().is_some_and(|rows| {
        !rows.is_empty()
            && (MANY.contains(&prefix.as_str()) || rows.len() == 1)
            && rows.iter().all(|r| shape(r, rest, &prefix))
    })
}
fn tgw_mode(v: &Value, m: &Value) -> bool {
    let known = |p| super::nested_references::value(v, m, p);
    let absent = |p| {
        known(p).is_some_and(|v| {
            v.is_null() || v.as_array().is_some_and(Vec::is_empty) || v.as_str() == Some("")
        })
    };
    absent("vpc_id")
        && absent("subnet_mapping")
        && known("transit_gateway_id")
            .and_then(Value::as_str)
            .is_some_and(|s| !s.is_empty())
        && v["availability_zone_mapping"]
            .as_array()
            .is_some_and(|rows| {
                !rows.is_empty() && {
                    let zones = rows
                        .iter()
                        .enumerate()
                        .map(|(i, _)| {
                            super::nested_references::value(
                                v,
                                m,
                                &format!("availability_zone_mapping.{i}.availability_zone_id"),
                            )
                            .and_then(Value::as_str)
                            .filter(|z| !z.is_empty())
                        })
                        .collect::<Option<std::collections::BTreeSet<_>>>();
                    zones.is_some_and(|z| z.len() == rows.len())
                }
            })
}
fn current(n: &TerraformEntity) -> bool {
    n.provider.is_aws()
        && n.mode == EntityMode::Managed
        && n.action != Action::Delete
        && n.deposed_key.is_none()
}
fn attachment(
    nodes: &[TerraformEntity],
    refs: &[AttributeReference],
    e: &[(&Value, &Value)],
    r: &AttributeReference,
) -> bool {
    let [s] = r.sources.as_slice() else {
        return false;
    };
    let source = &nodes[*s];
    if !current(&nodes[r.target])
        || !current(source)
        || source.resource_type != FIREWALL
        || !r.complete
        || !r.instances_explicit
        || !r.issues.is_empty()
        || refs
            .iter()
            .filter(|x| x.target == r.target && x.attribute == r.attribute)
            .count()
            != 1
    {
        return false;
    }
    let Some(attrs) = r.source_attributes_by_instance.get(s) else {
        return false;
    };
    if attrs.len() != 1 {
        return false;
    }
    let path = attrs.first().unwrap().replace('[', ".").replace(']', "");
    if path != ATTACHMENT {
        return false;
    }
    let (v, m) = e[*s];
    if !tgw_mode(v, m)
        || !v["firewall_status"].as_array().is_some_and(|a| {
            a.len() == 1
                && a[0]["transit_gateway_attachment_sync_states"]
                    .as_array()
                    .is_some_and(|a| a.len() == 1)
        })
    {
        return false;
    }
    super::service_evidence::known(e, r.target, "transit_gateway_attachment_id").is_some_and(
        |actual| {
            super::nested_references::value(v, m, ATTACHMENT).and_then(Value::as_str)
                == Some(actual)
        },
    )
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
            let valid = |s: usize, _path: &str| match k {
                FIREWALL => tgw_mode(v, m),
                CONTAINER => {
                    super::service_evidence::known(e, r.target, "type")
                        == Some(if nodes[s].resource_type == "aws_ecs_cluster" {
                            "ECS"
                        } else {
                            "EKS"
                        })
                }
                GROUP => {
                    super::service_evidence::known(e, r.target, "type") == Some("STATEFUL")
                        && r.sources.iter().all(|&other| {
                            (nodes[other].resource_type == CONTAINER)
                                == (nodes[s].resource_type == CONTAINER)
                        })
                }
                _ => true,
            };
            let ok = if k == ACCEPTER {
                attachment(nodes, refs, e, r)
            } else {
                shape(v, &r.attribute, "")
                    && super::nested_references::verify_collections(
                        nodes, refs, e, r, fields, valid, MANY,
                    )
                    .is_some()
            };
            let owner = if k == FIREWALL {
                super::service_evidence::known(e, r.target, "transit_gateway_owner_account_id")
            } else if k == ACCEPTER {
                r.sources.first().and_then(|&s| {
                    super::service_evidence::known(e, s, "transit_gateway_owner_account_id")
                })
            } else {
                None
            };
            let owner = owner.and_then(AwsAccountScope::new);
            Some((ok, owner))
        })
        .collect();
    for (r, result) in refs.iter_mut().zip(results) {
        if let Some((ok, owner)) = result {
            r.service_reference_verified = ok;
            if ok {
                r.complete = true;
            }
            r.service_reference_owner_account = owner;
        }
    }
    let contexts: Vec<_> = refs
        .iter()
        .map(|r| {
            if nodes[r.target].resource_type != ASSOCIATION
                || !matches!(r.attribute.as_str(), "vpc_id" | SUBNET)
                || !r.service_reference_verified
            {
                return Vec::new();
            }
            let proof = |p| {
                super::lambda_core::unique(refs, r.target, p)
                    .filter(|r| r.service_reference_verified)
            };
            let Some((vpc, subnet)) = proof("vpc_id").zip(proof(SUBNET)) else {
                return Vec::new();
            };
            let ([parent], [s]) = (vpc.sources.as_slice(), subnet.sources.as_slice()) else {
                return Vec::new();
            };
            if super::lambda_core::parent(nodes, refs, e, *s) == Some(*parent) {
                vec![*parent]
            } else {
                Vec::new()
            }
        })
        .collect();
    for (r, c) in refs.iter_mut().zip(contexts) {
        if !c.is_empty() {
            r.endpoint_network_context = c;
        }
    }
}
pub(super) fn bindings(raw: &Input, r: &AttributeReference) -> bool {
    let l = &raw.nodes[r.target];
    let k = l.resource_type.as_str();
    if label(k, &r.attribute).is_none() {
        return true;
    }
    r.sources
        .iter()
        .chain(&r.endpoint_network_context)
        .all(|&s| {
            let right = &raw.nodes[s];
            let mut left_binding = l.provider_configuration.clone();
            let mut right_binding = right.provider_configuration.clone();
            if let Some(owner) = &r.service_reference_owner_account {
                if k == FIREWALL {
                    let Some(b) = left_binding.as_mut() else {
                        return false;
                    };
                    if b.account_conflict {
                        return false;
                    }
                    b.account = Some(owner.clone());
                } else if k == ACCEPTER {
                    let Some(b) = right_binding.as_mut() else {
                        return false;
                    };
                    if b.account_conflict || b.account.is_none() {
                        return false;
                    }
                    b.account = Some(owner.clone());
                }
            }
            super::eventbridge::bindings(
                left_binding.as_ref(),
                right_binding.as_ref(),
                l.entity.scope.as_ref().and_then(|s| s.region.as_ref()),
                right.entity.scope.as_ref().and_then(|s| s.region.as_ref()),
                false,
            )
        })
}
pub(super) fn present(raw: &Input, g: &mut Graph) {
    for (target, _) in raw
        .nodes
        .iter()
        .enumerate()
        .filter(|(_, n)| n.resource_type == ASSOCIATION)
    {
        let proof = |p| {
            super::lambda_core::unique(&raw.attributes, target, p)
                .filter(|r| super::service_references::verified(raw, r))
        };
        let Some((vpc, subnet)) = proof("vpc_id").zip(proof(SUBNET)) else {
            continue;
        };
        let ([parent], [s]) = (
            vpc.endpoint_network_context.as_slice(),
            subnet.sources.as_slice(),
        ) else {
            continue;
        };
        if subnet.endpoint_network_context != [*parent]
            || super::streaming::vpc(raw, *s) != Some(*parent)
        {
            continue;
        }
        let evidence = vec![
            relationships::reference(g, *parent, target),
            relationships::reference(g, *parent, *s),
            relationships::reference(g, *s, target),
        ];
        relationships::record(g, *parent, target, EdgeKind::Containment, evidence);
        g.edges.push(Edge {
            kind: EdgeKind::Containment,
            ..Edge::from((*parent, target))
        });
    }
}
