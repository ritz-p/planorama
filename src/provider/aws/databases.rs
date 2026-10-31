use crate::model::{
    Action, AttributeReference, Edge, EdgeKind, EntityMode, Graph, TerraformEntity,
};
use crate::semantic::{Input, relationships};
use serde_json::Value;
use std::collections::BTreeSet;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/databases.rs"]
mod tests;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/neptune.rs"]
mod neptune_tests;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/redshift_serverless.rs"]
mod redshift_serverless_tests;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/dms.rs"]
mod dms_tests;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/database_endpoints.rs"]
mod endpoint_tests;

fn rule(
    resource: &str,
    attribute: &str,
) -> Option<(&'static str, &'static str, bool, &'static str)> {
    match (resource, attribute) {
        ("aws_db_proxy_endpoint", "db_proxy_name") => {
            Some(("aws_db_proxy", "name", false, "Proxy owner"))
        }
        ("aws_rds_cluster_endpoint", "cluster_identifier") => Some((
            "aws_rds_cluster",
            "cluster_identifier",
            false,
            "Endpoint cluster",
        )),
        ("aws_neptune_cluster_endpoint", "cluster_identifier") => Some((
            "aws_neptune_cluster",
            "cluster_identifier",
            false,
            "Endpoint cluster",
        )),
        ("aws_rds_cluster_endpoint", "static_members" | "excluded_members") => Some((
            "aws_rds_cluster_instance",
            "identifier",
            true,
            if attribute == "excluded_members" {
                "Excluded endpoint members"
            } else {
                "Static endpoint members"
            },
        )),
        ("aws_neptune_cluster_endpoint", "static_members" | "excluded_members") => Some((
            "aws_neptune_cluster_instance",
            "identifier",
            true,
            if attribute == "excluded_members" {
                "Excluded endpoint members"
            } else {
                "Static endpoint members"
            },
        )),
        ("aws_dms_replication_instance", "replication_subnet_group_id") => Some((
            "aws_dms_replication_subnet_group",
            "id",
            false,
            "Subnet group",
        )),
        ("aws_dms_replication_instance", "vpc_security_group_ids") => Some((
            "aws_security_group",
            "id",
            true,
            "Configured security groups",
        )),
        ("aws_dms_replication_subnet_group", "subnet_ids") => {
            Some(("aws_subnet", "id", true, "Configured subnets"))
        }
        ("aws_dms_replication_task", "replication_instance_arn") => Some((
            "aws_dms_replication_instance",
            "replication_instance_arn",
            false,
            "Replication instance",
        )),
        ("aws_dms_replication_task", "source_endpoint_arn") => {
            Some(("aws_dms_endpoint", "endpoint_arn", false, "Source endpoint"))
        }
        ("aws_dms_replication_task", "target_endpoint_arn") => {
            Some(("aws_dms_endpoint", "endpoint_arn", false, "Target endpoint"))
        }
        ("aws_docdb_cluster", "db_subnet_group_name") => {
            Some(("aws_docdb_subnet_group", "name", false, "Subnet group"))
        }
        ("aws_docdb_cluster", "vpc_security_group_ids") => Some((
            "aws_security_group",
            "id",
            true,
            "Configured security groups",
        )),
        ("aws_docdb_subnet_group", "subnet_ids") => {
            Some(("aws_subnet", "id", true, "Configured subnets"))
        }
        ("aws_docdb_cluster_instance", "cluster_identifier") => Some((
            "aws_docdb_cluster",
            "cluster_identifier",
            false,
            "Cluster membership",
        )),
        ("aws_neptune_cluster" | "aws_neptune_cluster_instance", "neptune_subnet_group_name") => {
            Some(("aws_neptune_subnet_group", "name", false, "Subnet group"))
        }
        ("aws_neptune_cluster", "vpc_security_group_ids") => Some((
            "aws_security_group",
            "id",
            true,
            "Configured security groups",
        )),
        ("aws_neptune_subnet_group", "subnet_ids") => {
            Some(("aws_subnet", "id", true, "Configured subnets"))
        }
        ("aws_neptune_cluster_instance", "cluster_identifier") => Some((
            "aws_neptune_cluster",
            "cluster_identifier",
            false,
            "Cluster membership",
        )),
        ("aws_redshiftserverless_workgroup", "namespace_name") => Some((
            "aws_redshiftserverless_namespace",
            "namespace_name",
            false,
            "Namespace owner",
        )),
        ("aws_redshiftserverless_endpoint_access", "workgroup_name") => Some((
            "aws_redshiftserverless_workgroup",
            "workgroup_name",
            false,
            "Workgroup owner",
        )),
        (
            "aws_redshiftserverless_workgroup" | "aws_redshiftserverless_endpoint_access",
            "subnet_ids",
        ) => Some(("aws_subnet", "id", true, "Configured subnets")),
        ("aws_redshiftserverless_workgroup", "security_group_ids")
        | ("aws_redshiftserverless_endpoint_access", "vpc_security_group_ids") => Some((
            "aws_security_group",
            "id",
            true,
            "Configured security groups",
        )),
        _ => None,
    }
}

pub(super) fn label(resource: &str, attribute: &str) -> Option<&'static str> {
    rule(resource, attribute).map(|r| r.3)
}

fn current(node: &TerraformEntity) -> bool {
    node.provider.is_aws()
        && node.mode == EntityMode::Managed
        && node.action != Action::Delete
        && node.deposed_key.is_none()
}

pub(super) fn annotate(
    nodes: &[TerraformEntity],
    references: &mut [AttributeReference],
    evidence: &[(&Value, &Value)],
) {
    for r in references.iter_mut() {
        let node = &nodes[r.target];
        let Some((kind, field, collection, _)) = rule(&node.resource_type, &r.attribute) else {
            continue;
        };
        let field = if kind == "aws_dms_replication_subnet_group"
            && r.source_attributes.contains("replication_subnet_group_id")
        {
            "replication_subnet_group_id"
        } else {
            field
        };
        r.service_reference_verified = false;
        r.service_reference_owner_account = None;
        if !current(node)
            || !r.complete
            || !r.instances_explicit
            || !r.issues.is_empty()
            || r.sources.is_empty()
            || (!collection && r.sources.len() != 1)
            || r.source_attributes.len() != 1
            || !r.source_attributes.contains(field)
        {
            continue;
        }
        let (value, mask) = evidence[r.target];
        let Some(actual) = super::streaming::values(value, mask, &r.attribute, collection) else {
            continue;
        };
        if node.resource_type == "aws_redshiftserverless_endpoint_access"
            && r.attribute == "workgroup_name"
            && (!value["owner_account"].is_null()
                || !matches!(&mask["owner_account"], Value::Null | Value::Bool(false)))
        {
            let Some(account) = super::service_references::scalar(value, mask, &["owner_account"])
                .and_then(crate::model::AwsAccountScope::new)
            else {
                continue;
            };
            r.service_reference_owner_account = Some(account);
        }
        let expected = r
            .sources
            .iter()
            .map(|&source| {
                let endpoint = &nodes[source];
                if !current(endpoint) || endpoint.resource_type != kind {
                    return None;
                }
                if !super::eventbridge::bindings(
                    node.provider_configuration.as_ref(),
                    endpoint.provider_configuration.as_ref(),
                    super::scope::scope(node)
                        .as_ref()
                        .and_then(|s| s.region.as_ref()),
                    super::scope::scope(endpoint)
                        .as_ref()
                        .and_then(|s| s.region.as_ref()),
                    false,
                ) {
                    return None;
                }
                let (value, mask) = evidence[source];
                if kind == "aws_dms_endpoint" {
                    let expected_type = if r.attribute == "source_endpoint_arn" {
                        "source"
                    } else {
                        "target"
                    };
                    if super::service_references::scalar(value, mask, &["endpoint_type"])
                        != Some(expected_type)
                    {
                        return None;
                    }
                }
                super::service_references::scalar(value, mask, &[field])
            })
            .collect::<Option<BTreeSet<_>>>();
        r.service_reference_verified = expected
            .is_some_and(|expected| expected.len() == r.sources.len() && actual == expected);
    }
    let invalid_members: BTreeSet<_> = references
        .iter()
        .enumerate()
        .filter_map(|(index, r)| {
            if !matches!(
                nodes[r.target].resource_type.as_str(),
                "aws_rds_cluster_endpoint" | "aws_neptune_cluster_endpoint"
            ) || !matches!(r.attribute.as_str(), "static_members" | "excluded_members")
            {
                return None;
            }
            let valid = r.sources.iter().all(|&source| {
                let (value, mask) = evidence[source];
                let actual =
                    super::service_references::scalar(value, mask, &["cluster_identifier"]);
                let mut refs = references
                    .iter()
                    .filter(|r| r.target == source && r.attribute == "cluster_identifier");
                let Some(member) = refs.next() else {
                    return false;
                };
                let [cluster] = member.sources.as_slice() else {
                    return false;
                };
                let (value, mask) = evidence[*cluster];
                refs.next().is_none()
                    && actual.is_some()
                    && actual
                        == super::service_references::scalar(value, mask, &["cluster_identifier"])
            });
            (!valid).then_some(index)
        })
        .collect();
    for index in invalid_members {
        references[index].service_reference_verified = false;
    }
    let groups = references
        .iter()
        .filter(|r| r.attribute == "neptune_subnet_group_name" && r.service_reference_verified)
        .filter_map(|r| r.sources.first().map(|&source| (r.target, source)))
        .collect::<std::collections::BTreeMap<_, _>>();
    for r in references.iter_mut().filter(|r| {
        nodes[r.target].resource_type == "aws_neptune_cluster_instance"
            && r.attribute == "cluster_identifier"
    }) {
        let (value, mask) = evidence[r.target];
        let field = "neptune_subnet_group_name";
        if value[field].is_null() && matches!(&mask[field], Value::Null | Value::Bool(false)) {
            continue;
        }
        r.service_reference_verified &= r.sources.first().is_some_and(|owner| {
            groups
                .get(&r.target)
                .is_some_and(|group| Some(group) == groups.get(owner))
        });
    }
}

pub(super) fn bindings(raw: &Input, reference: &AttributeReference) -> bool {
    super::streaming::bindings(raw, reference)
        && endpoint_members(raw, reference)
        && reference
            .service_reference_owner_account
            .as_ref()
            .is_none_or(|account| {
                reference.sources.iter().all(|&source| {
                    raw.nodes[source]
                        .provider_configuration
                        .as_ref()
                        .and_then(|binding| binding.account.as_ref())
                        == Some(account)
                })
            })
}

fn endpoint_members(raw: &Input, reference: &AttributeReference) -> bool {
    let kind = match raw.nodes[reference.target].resource_type.as_str() {
        "aws_rds_cluster_endpoint" => "aws_rds_cluster",
        "aws_neptune_cluster_endpoint" => "aws_neptune_cluster",
        _ => return true,
    };
    if !matches!(
        reference.attribute.as_str(),
        "static_members" | "excluded_members"
    ) {
        return true;
    }
    let Some(owner) = proof(raw, reference.target, "cluster_identifier") else {
        return false;
    };
    reference.sources.iter().all(|&source| {
        let mut refs = raw
            .attributes
            .iter()
            .filter(|r| r.target == source && r.attribute == "cluster_identifier");
        let Some(member) = refs.next() else {
            return false;
        };
        refs.next().is_none()
            && member.complete
            && member.instances_explicit
            && member.issues.is_empty()
            && member.sources == owner.sources
            && member.source_attributes.len() == 1
            && member.source_attributes.contains("cluster_identifier")
            && member.sources.iter().all(|&i| {
                raw.nodes[i].provider.is_aws()
                    && raw.nodes[i].mode == EntityMode::Managed
                    && raw.nodes[i].action != Action::Delete
                    && raw.nodes[i].deposed_key.is_none()
                    && raw.nodes[i].resource_type == kind
            })
            && super::streaming::bindings(raw, member)
    })
}

fn proof<'a>(raw: &'a Input, target: usize, attribute: &str) -> Option<&'a AttributeReference> {
    let mut refs = raw
        .attributes
        .iter()
        .filter(|r| r.target == target && r.attribute == attribute);
    let r = refs.next()?;
    (refs.next().is_none() && r.service_reference_verified && bindings(raw, r)).then_some(r)
}

fn network(raw: &Input, target: usize) -> Option<(usize, Vec<(usize, usize)>)> {
    let node = &raw.nodes[target];
    let (subnet_target, group_attribute, mut edges) = match node.resource_type.as_str() {
        "aws_docdb_cluster" | "aws_neptune_cluster" | "aws_dms_replication_instance" => {
            let attribute = if node.resource_type == "aws_docdb_cluster" {
                "db_subnet_group_name"
            } else if node.resource_type == "aws_dms_replication_instance" {
                "replication_subnet_group_id"
            } else {
                "neptune_subnet_group_name"
            };
            let group_ref = proof(raw, target, attribute)?;
            let [group] = group_ref.sources.as_slice() else {
                return None;
            };
            (*group, "vpc_security_group_ids", vec![(*group, target)])
        }
        "aws_redshiftserverless_workgroup" => (target, "security_group_ids", Vec::new()),
        "aws_redshiftserverless_endpoint_access" => (target, "vpc_security_group_ids", Vec::new()),
        _ => return None,
    };
    let subnets = proof(raw, subnet_target, "subnet_ids")?;
    let groups = proof(raw, target, group_attribute)?;
    let members = subnets.sources.iter().chain(&groups.sources);
    let parents = members
        .map(|&i| super::streaming::vpc(raw, i))
        .collect::<Option<BTreeSet<_>>>()?;
    let [parent] = parents.iter().copied().collect::<Vec<_>>()[..] else {
        return None;
    };
    for &subnet in &subnets.sources {
        edges.extend([(parent, subnet), (subnet, subnet_target)]);
    }
    for &sg in &groups.sources {
        edges.extend([(parent, sg), (sg, target)]);
    }
    Some((parent, edges))
}

pub(super) fn present(raw: &Input, graph: &mut Graph) {
    for (target, node) in raw.nodes.iter().enumerate() {
        if let Some((parent, edges)) = network(raw, target) {
            let evidence = edges
                .into_iter()
                .map(|(a, b)| relationships::reference(graph, a, b))
                .collect();
            relationships::record(graph, parent, target, EdgeKind::Containment, evidence);
            graph.edges.push(Edge {
                kind: EdgeKind::Containment,
                ..Edge::from((parent, target))
            });
        }
        if !matches!(
            node.resource_type.as_str(),
            "aws_docdb_cluster_instance" | "aws_neptune_cluster_instance"
        ) {
            continue;
        }
        let Some(r) = proof(raw, target, "cluster_identifier") else {
            continue;
        };
        let [owner] = r.sources.as_slice() else {
            continue;
        };
        let parent = &raw.nodes[*owner];
        if parent.entity.scope != node.entity.scope {
            continue;
        }
        graph.nodes[target].member_owner =
            parent.entity.provenance.first().cloned().map(|target| {
                crate::model::PresentationOwner {
                    target,
                    attribute: "cluster_identifier",
                    label: "Cluster",
                    section: "Cluster instances",
                }
            });
    }
}
