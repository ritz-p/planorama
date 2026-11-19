use super::configured_links::{self, Evidence, Rule};
use crate::model::{AttributeReference, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;

fn rule(kind: &str, path: &str) -> Option<Rule> {
    Some(match (kind, path) {
        ("aws_cloudhsm_v2_cluster", "subnet_ids") => {
            Rule::new("Cluster subnets", &[("aws_subnet", &["id"])], true, &[])
        }
        ("aws_cloudhsm_v2_hsm", "cluster_id") => Rule::new(
            "CloudHSM cluster",
            &[("aws_cloudhsm_v2_cluster", &["cluster_id", "id"])],
            false,
            &[],
        ),
        ("aws_cloudhsm_v2_hsm", "subnet_id") => {
            Rule::new("HSM subnet", &[("aws_subnet", &["id"])], false, &[])
        }
        _ => return None,
    })
}
pub(crate) fn resource(kind: &str) -> bool {
    matches!(kind, "aws_cloudhsm_v2_cluster" | "aws_cloudhsm_v2_hsm")
}
pub(crate) fn indexed(kind: &str, path: &str) -> bool {
    resource(kind) && configured_links::indexed(kind, path, rule)
}
pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    rule(kind, path).map(|r| r.label)
}
pub(super) fn bindings(raw: &Input, r: &AttributeReference) -> bool {
    configured_links::bindings(raw, r, rule)
}
pub(super) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    values: &[(&Value, &Value)],
) {
    configured_links::annotate(nodes, refs, values, rule, qualifies)
}
fn qualifies(e: &Evidence<'_>, r: &AttributeReference, source: usize, _path: &str) -> bool {
    match r.attribute.as_str() {
        "subnet_ids" => cluster_vpc(e, r.target).is_some(),
        "subnet_id" => e
            .configured_endpoint(
                r.target,
                "cluster_id",
                "aws_cloudhsm_v2_cluster",
                &["cluster_id", "id"],
            )
            .and_then(|cluster| cluster_vpc(e, cluster))
            .zip(super::lambda_core::parent(
                e.nodes, e.refs, e.values, source,
            ))
            .is_some_and(|(cluster_vpc, subnet_vpc)| cluster_vpc == subnet_vpc),
        _ => true,
    }
}

fn cluster_vpc(e: &Evidence<'_>, cluster: usize) -> Option<usize> {
    let reference = super::lambda_core::unique(e.refs, cluster, "subnet_ids")?;
    if !super::service_evidence::eligible(e.nodes, reference) {
        return None;
    }
    let (value, mask) = e.values[cluster];
    let actual = super::streaming::values(value, mask, "subnet_ids", true)?;
    let mut expected = std::collections::BTreeSet::new();
    let mut parents = std::collections::BTreeSet::new();
    for &subnet in &reference.sources {
        if e.nodes[subnet].resource_type != "aws_subnet"
            || reference
                .source_attributes_by_instance
                .get(&subnet)
                .is_none_or(|attrs| attrs.len() != 1 || !attrs.contains("id"))
            || !expected.insert(e.text(subnet, "id")?)
        {
            return None;
        }
        parents.insert(super::lambda_core::parent(
            e.nodes, e.refs, e.values, subnet,
        )?);
    }
    if actual != expected || parents.len() != 1 {
        return None;
    }
    parents.pop_first()
}

pub(super) fn present(raw: &Input, graph: &mut crate::model::Graph) {
    use crate::model::{Edge, EdgeKind};
    use crate::semantic::relationships;
    for reference in &raw.attributes {
        let index = reference.target;
        let node = &raw.nodes[index];
        if node.resource_type != "aws_cloudhsm_v2_cluster"
            || reference.attribute != "subnet_ids"
            || !super::service_references::verified(raw, reference)
        {
            continue;
        }
        let [vpc] = reference.endpoint_network_context.as_slice() else {
            continue;
        };
        let owner = &raw.nodes[*vpc];
        if !super::service_references::cluster_binding_matches(
            node.provider_configuration.as_ref(),
            owner.provider_configuration.as_ref(),
            node.entity
                .scope
                .as_ref()
                .and_then(|scope| scope.region.as_ref()),
            owner
                .entity
                .scope
                .as_ref()
                .and_then(|scope| scope.region.as_ref()),
        ) {
            continue;
        }
        let evidence = reference
            .sources
            .iter()
            .flat_map(|&subnet| {
                [
                    relationships::reference(graph, *vpc, subnet),
                    relationships::reference(graph, subnet, index),
                ]
            })
            .collect();
        relationships::record(graph, *vpc, index, EdgeKind::Containment, evidence);
        graph.edges.push(Edge {
            kind: EdgeKind::Containment,
            ..Edge::from((*vpc, index))
        });
    }
}
#[cfg(test)]
#[path = "../../../tests/unit/semantic/cloudhsm_configuration.rs"]
mod tests;
