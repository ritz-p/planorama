use crate::model::{AttributeReference, Edge, EdgeKind, Graph, TerraformEntity};
use crate::semantic::{Input, relationships};
use serde_json::Value;
#[cfg(test)]
#[path = "../../../tests/unit/semantic/network_monitors.rs"]
mod tests;
const FLOW: &str = "aws_networkflowmonitor_monitor";
const SCOPE: &str = "aws_networkflowmonitor_scope";
const MONITOR: &str = "aws_networkmonitor_monitor";
const PROBE: &str = "aws_networkmonitor_probe";
const LOCAL: &str = "local_resource.identifier";
const REMOTE: &str = "remote_resource.identifier";
pub(crate) fn resource(k: &str) -> bool {
    matches!(k, FLOW | SCOPE | MONITOR | PROBE)
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
        (FLOW, "scope_arn") => Some("Flow monitor scope"),
        (FLOW, LOCAL) => Some("Monitored local resources"),
        (FLOW, REMOTE) => Some("Monitored remote resources"),
        (PROBE, "monitor_name") => Some("Probe monitor"),
        (PROBE, "source_arn") => Some("Probe source subnet"),
        _ => None,
    }
}
fn fields(k: &str, p: &str, s: &str) -> &'static [&'static str] {
    match (k, p, s) {
        (FLOW, "scope_arn", SCOPE) => &["scope_arn"],
        (FLOW, LOCAL | REMOTE, "aws_vpc" | "aws_subnet" | "aws_eks_cluster")
        | (PROBE, "source_arn", "aws_subnet") => &["arn"],
        (PROBE, "monitor_name", MONITOR) => &["monitor_name"],
        _ => &[],
    }
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
            let shape = if matches!(r.attribute.as_str(), LOCAL | REMOTE) {
                v[r.attribute.split('.').next().unwrap()]
                    .as_array()
                    .is_some_and(|rows| {
                        !rows.is_empty() && rows.iter().all(|r| r["identifier"].is_string())
                    })
            } else {
                v[&r.attribute].is_string()
            };
            let valid = |s: usize, path: &str| {
                if !matches!(r.attribute.as_str(), LOCAL | REMOTE) {
                    return true;
                }
                let Some((row, _)) = path.rsplit_once('.') else {
                    return false;
                };
                let kind = super::nested_references::value(v, m, &format!("{row}.type"))
                    .and_then(Value::as_str);
                matches!(
                    (kind, nodes[s].resource_type.as_str()),
                    (Some("AWS::EC2::VPC"), "aws_vpc") | (Some("AWS::EC2::Subnet"), "aws_subnet")
                ) || (r.attribute == LOCAL
                    && kind == Some("AWS::EKS::Cluster")
                    && nodes[s].resource_type == "aws_eks_cluster")
            };
            Some(
                shape
                    && super::nested_references::verify_collections(
                        nodes,
                        refs,
                        e,
                        r,
                        fields,
                        valid,
                        &["local_resource", "remote_resource"],
                    )
                    .is_some(),
            )
        })
        .collect();
    for (r, ok) in refs.iter_mut().zip(results) {
        if let Some(ok) = ok {
            r.service_reference_verified = ok;
            if ok {
                r.complete = true;
            }
        }
    }
    let contexts: Vec<_> = refs
        .iter()
        .map(|r| {
            if nodes[r.target].resource_type != PROBE
                || r.attribute != "source_arn"
                || !r.service_reference_verified
            {
                return None;
            }
            let [s] = r.sources.as_slice() else {
                return None;
            };
            super::lambda_core::parent(nodes, refs, e, *s)
        })
        .collect();
    for (r, c) in refs.iter_mut().zip(contexts) {
        if let Some(c) = c {
            r.endpoint_network_context = vec![c];
        }
    }
}
pub(super) fn bindings(raw: &Input, r: &AttributeReference) -> bool {
    if label(&raw.nodes[r.target].resource_type, &r.attribute).is_none() {
        return true;
    }
    let l = &raw.nodes[r.target];
    r.sources
        .iter()
        .chain(&r.endpoint_network_context)
        .all(|&s| {
            let right = &raw.nodes[s];
            super::eventbridge::bindings(
                l.provider_configuration.as_ref(),
                right.provider_configuration.as_ref(),
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
        .filter(|(_, n)| n.resource_type == PROBE)
    {
        let Some(r) = super::lambda_core::unique(&raw.attributes, target, "source_arn")
            .filter(|r| super::service_references::verified(raw, r))
        else {
            continue;
        };
        let ([parent], [s]) = (r.endpoint_network_context.as_slice(), r.sources.as_slice()) else {
            continue;
        };
        if super::streaming::vpc(raw, *s) != Some(*parent) {
            continue;
        }
        let evidence = vec![
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
