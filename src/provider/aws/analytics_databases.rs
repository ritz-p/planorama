use super::{lambda_core, nested_references, service_evidence};
use crate::model::{AttributeReference, Graph, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;
use std::collections::BTreeSet;

const ELASTIC: &str = "aws_docdbelastic_cluster";
const GRAPH: &str = "aws_neptunegraph_graph";
const ENDPOINT: &str = "aws_neptunegraph_private_graph_endpoint";

fn rule(kind: &str, path: &str) -> Option<(&'static str, &'static [&'static str], &'static str)> {
    Some(match (kind, path) {
        (ELASTIC | ENDPOINT, "subnet_ids") => ("aws_subnet", &["id"], "Configured subnets"),
        (ELASTIC | ENDPOINT, "vpc_security_group_ids") => {
            ("aws_security_group", &["id"], "Configured security groups")
        }
        (ELASTIC, "kms_key_id") | (GRAPH, "kms_key_identifier") => {
            ("aws_kms_key", &["arn"], "Encryption key")
        }
        (ENDPOINT, "vpc_id") => ("aws_vpc", &["id"], "Endpoint VPC"),
        (ENDPOINT, "graph_identifier") => (GRAPH, &["id"], "Private endpoint graph"),
        _ => return None,
    })
}

pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    rule(kind, path).map(|(_, _, label)| label)
}
fn fields(kind: &str, path: &str, source: &str) -> &'static [&'static str] {
    rule(kind, path)
        .filter(|(expected, _, _)| *expected == source)
        .map_or(&[], |(_, outputs, _)| outputs)
}

pub(super) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    e: &[(&Value, &Value)],
) {
    let results: Vec<_> = refs
        .iter()
        .map(|r| {
            let kind = &nodes[r.target].resource_type;
            label(kind, &r.attribute)?;
            let collection = matches!(
                r.attribute.as_str(),
                "subnet_ids" | "vpc_security_group_ids"
            );
            let valid = service_evidence::eligible(nodes, r)
                && nested_references::verify(nodes, refs, e, r, fields, |_, path| {
                    nested_references::value(e[r.target].0, e[r.target].1, path).is_some_and(|v| {
                        if collection {
                            v.is_array()
                        } else {
                            v.is_string()
                        }
                    })
                })
                .is_some();
            let parents = if collection && valid {
                r.sources
                    .iter()
                    .map(|&s| lambda_core::parent(nodes, refs, e, s))
                    .collect::<Option<BTreeSet<_>>>()
                    .filter(|p| p.len() == 1)
                    .into_iter()
                    .flatten()
                    .collect()
            } else {
                Vec::new()
            };
            Some((valid, parents))
        })
        .collect();
    for (r, result) in refs.iter_mut().zip(results) {
        if let Some((valid, parents)) = result {
            r.service_reference_verified = valid;
            r.endpoint_network_context = parents;
        }
    }
}
pub(super) fn bindings(raw: &Input, r: &AttributeReference) -> bool {
    label(&raw.nodes[r.target].resource_type, &r.attribute).is_none()
        || lambda_core::bindings(raw, r)
}
pub(super) fn present(raw: &Input, graph: &mut Graph) {
    let paths = ("subnet_ids", "vpc_security_group_ids");
    lambda_core::present_network(raw, graph, ELASTIC, paths, bindings);
    lambda_core::present_network(raw, graph, ENDPOINT, paths, |raw, r| {
        bindings(raw, r)
            && lambda_core::unique(&raw.attributes, r.target, "vpc_id").is_some_and(|vpc| {
                vpc.service_reference_verified
                    && bindings(raw, vpc)
                    && vpc.sources == r.endpoint_network_context
            })
    });
}

#[cfg(test)]
#[path = "../../../tests/unit/semantic/analytics_databases.rs"]
mod tests;
