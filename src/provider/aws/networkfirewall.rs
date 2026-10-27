use crate::model::{AttributeReference, Edge, EdgeKind, Graph, TerraformEntity};
use crate::semantic::{Input, relationships};
use serde_json::Value;
use std::collections::BTreeSet;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/networkfirewall.rs"]
mod tests;

const SUBNETS: &str = "subnet_mapping.subnet_id";
const STATEFUL: &str = "firewall_policy.stateful_rule_group_reference.resource_arn";
const STATELESS: &str = "firewall_policy.stateless_rule_group_reference.resource_arn";

pub(crate) fn indexed_attribute(resource: &str, attribute: &str) -> bool {
    if resource != "aws_networkfirewall_logging_configuration" {
        return false;
    }
    let p = attribute.split('.').collect::<Vec<_>>();
    matches!(p.as_slice(), ["logging_configuration", "0", "log_destination_config", index, "log_destination", key]
        if index.parse::<usize>().is_ok() && matches!(*key, "bucketName" | "logGroup" | "deliveryStream"))
}

fn rule(resource: &str, attribute: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match (resource, attribute) {
        ("aws_networkfirewall_firewall", "vpc_id") => Some(("aws_vpc", "id", "Firewall VPC")),
        ("aws_networkfirewall_firewall", SUBNETS) => Some(("aws_subnet", "id", "Firewall subnets")),
        ("aws_networkfirewall_firewall", "firewall_policy_arn") => Some((
            "aws_networkfirewall_firewall_policy",
            "arn",
            "Firewall policy",
        )),
        ("aws_networkfirewall_firewall_policy", STATEFUL) => Some((
            "aws_networkfirewall_rule_group",
            "arn",
            "Stateful rule groups",
        )),
        ("aws_networkfirewall_firewall_policy", STATELESS) => Some((
            "aws_networkfirewall_rule_group",
            "arn",
            "Stateless rule groups",
        )),
        ("aws_networkfirewall_logging_configuration", "firewall_arn") => {
            Some(("aws_networkfirewall_firewall", "arn", "Logging firewall"))
        }
        _ => None,
    }
}

pub(super) fn label(resource: &str, attribute: &str) -> Option<&'static str> {
    rule(resource, attribute)
        .map(|r| r.2)
        .or_else(|| indexed_attribute(resource, attribute).then_some("Firewall log destination"))
}

pub(super) fn aggregate(raw: &Input, reference: &AttributeReference) -> bool {
    let resource = &raw.nodes[reference.target].resource_type;
    let Some(key) = reference
        .attribute
        .strip_prefix("logging_configuration.log_destination_config.log_destination.")
    else {
        return false;
    };
    raw.attributes.iter().any(|r| {
        r.target == reference.target
            && indexed_attribute(resource, &r.attribute)
            && r.attribute.rsplit('.').next() == Some(key)
    })
}

fn leaves<'a>(
    value: &'a Value,
    unknown: &Value,
    path: &[&str],
    out: &mut Vec<&'a str>,
) -> Option<()> {
    if path.is_empty() {
        if !matches!(unknown, Value::Null | Value::Bool(false)) {
            return None;
        }
        out.push(value.as_str().filter(|s| !s.is_empty())?);
        return Some(());
    }
    if let Some(values) = value.as_array() {
        if values.is_empty() {
            return None;
        }
        let masks = match unknown {
            Value::Null | Value::Bool(false) => None,
            Value::Array(masks) if masks.len() == values.len() => Some(masks),
            _ => return None,
        };
        for (i, v) in values.iter().enumerate() {
            leaves(v, masks.map_or(&Value::Null, |m| &m[i]), path, out)?;
        }
    } else if let Some((field, rest)) = path.split_first() {
        if !value.is_object()
            || !matches!(unknown, Value::Null | Value::Bool(false) | Value::Object(_))
        {
            return None;
        }
        leaves(&value[*field], &unknown[*field], rest, out)?;
    } else {
        if !matches!(unknown, Value::Null | Value::Bool(false)) {
            return None;
        }
        out.push(value.as_str().filter(|s| !s.is_empty())?);
    }
    Some(())
}

fn absent(value: &Value, unknown: &Value, field: &str) -> bool {
    matches!(unknown, Value::Null | Value::Bool(false) | Value::Object(_))
        && matches!(&unknown[field], Value::Null | Value::Bool(false))
        && (value[field].is_null()
            || value[field].as_str() == Some("")
            || value[field].as_array().is_some_and(Vec::is_empty))
}

fn log_destination(
    nodes: &[TerraformEntity],
    r: &AttributeReference,
    evidence: &[(&Value, &Value)],
) -> bool {
    let [source] = r.sources.as_slice() else {
        return false;
    };
    let (kind, key, field) = match nodes[*source].resource_type.as_str() {
        "aws_s3_bucket" => ("S3", "bucketName", "bucket"),
        "aws_cloudwatch_log_group" => ("CloudWatchLogs", "logGroup", "name"),
        "aws_kinesis_firehose_delivery_stream" => ("KinesisDataFirehose", "deliveryStream", "name"),
        _ => return false,
    };
    if r.attribute.rsplit('.').next() != Some(key)
        || r.source_attributes.len() != 1
        || !r.source_attributes.contains(field)
    {
        return false;
    }
    let Some(expected) = super::service_evidence::known(evidence, *source, field) else {
        return false;
    };
    let (value, unknown) = evidence[r.target];
    let Some(config) = value["logging_configuration"]
        .as_array()
        .filter(|v| v.len() == 1)
    else {
        return false;
    };
    let Some(blocks) = config[0]["log_destination_config"].as_array() else {
        return false;
    };
    if r.indexed_block_lengths != [1, blocks.len()] {
        return false;
    }
    let mut kinds = Vec::new();
    if leaves(
        value,
        unknown,
        &[
            "logging_configuration",
            "log_destination_config",
            "log_destination_type",
        ],
        &mut kinds,
    )
    .is_none()
    {
        return false;
    }
    let mask = match &unknown["logging_configuration"] {
        Value::Null | Value::Bool(false) => &Value::Null,
        Value::Array(v) if v.len() == 1 => &v[0]["log_destination_config"],
        _ => return false,
    };
    let mut matches = 0;
    for (i, block) in blocks.iter().enumerate() {
        let m = match mask {
            Value::Null | Value::Bool(false) => &Value::Null,
            Value::Array(v) if v.len() == blocks.len() => &v[i],
            _ => return false,
        };
        let mut destination = Vec::new();
        if kinds.get(i) == Some(&kind) {
            if leaves(block, m, &["log_destination", key], &mut destination).is_none() {
                return false;
            }
            if destination == [expected] {
                matches += 1;
            }
        }
    }
    matches == 1
}

pub(super) fn annotate(
    nodes: &[TerraformEntity],
    references: &mut [AttributeReference],
    evidence: &[(&Value, &Value)],
) {
    for r in references {
        let node = &nodes[r.target];
        if label(&node.resource_type, &r.attribute).is_none() {
            continue;
        }
        r.service_reference_verified = false;
        if !super::service_evidence::eligible(nodes, r) {
            continue;
        }
        if indexed_attribute(&node.resource_type, &r.attribute) {
            r.service_reference_verified = log_destination(nodes, r, evidence);
            continue;
        }
        let Some((kind, field, _)) = rule(&node.resource_type, &r.attribute) else {
            continue;
        };
        if r.source_attributes.len() != 1 || !r.source_attributes.contains(field) {
            continue;
        }
        let (value, unknown) = evidence[r.target];
        if r.attribute == SUBNETS && !value["subnet_mapping"].is_array() {
            continue;
        }
        if matches!(r.attribute.as_str(), STATEFUL | STATELESS)
            && !value["firewall_policy"].as_array().is_some_and(|blocks| {
                blocks.len() == 1
                    && blocks[0][if r.attribute == STATEFUL {
                        "stateful_rule_group_reference"
                    } else {
                        "stateless_rule_group_reference"
                    }]
                    .is_array()
            })
        {
            continue;
        }
        if node.resource_type == "aws_networkfirewall_firewall"
            && matches!(r.attribute.as_str(), "vpc_id" | SUBNETS)
            && (!absent(value, unknown, "transit_gateway_id")
                || !absent(value, unknown, "availability_zone_mapping"))
        {
            continue;
        }
        let mut actual = Vec::new();
        if leaves(
            value,
            unknown,
            &r.attribute.split('.').collect::<Vec<_>>(),
            &mut actual,
        )
        .is_none()
        {
            continue;
        }
        let collection = matches!(r.attribute.as_str(), SUBNETS | STATEFUL | STATELESS);
        if !collection && actual.len() != 1 {
            continue;
        }
        let expected = r
            .sources
            .iter()
            .map(|&source| {
                if nodes[source].resource_type != kind {
                    return None;
                }
                if matches!(r.attribute.as_str(), STATEFUL | STATELESS)
                    && super::service_evidence::known(evidence, source, "type")
                        != Some(if r.attribute == STATEFUL {
                            "STATEFUL"
                        } else {
                            "STATELESS"
                        })
                {
                    return None;
                }
                super::service_evidence::known(evidence, source, field)
            })
            .collect::<Option<BTreeSet<_>>>();
        let unique = actual.iter().copied().collect::<BTreeSet<_>>();
        r.service_reference_verified = expected.is_some_and(|expected| {
            expected.len() == r.sources.len() && unique.len() == actual.len() && expected == unique
        });
    }
}

fn proof<'a>(raw: &'a Input, target: usize, attribute: &str) -> Option<&'a AttributeReference> {
    let mut refs = raw
        .attributes
        .iter()
        .filter(|r| r.target == target && r.attribute == attribute);
    let r = refs.next()?;
    (refs.next().is_none() && super::service_references::verified(raw, r)).then_some(r)
}

fn network(raw: &Input, target: usize) -> Option<(usize, &[usize])> {
    let vpc = proof(raw, target, "vpc_id")?;
    let [parent] = vpc.sources.as_slice() else {
        return None;
    };
    let subnets = proof(raw, target, SUBNETS)?;
    subnets
        .sources
        .iter()
        .all(|&s| super::streaming::vpc(raw, s) == Some(*parent))
        .then_some((*parent, subnets.sources.as_slice()))
}

pub(super) fn diagnostics(raw: &Input) -> Vec<crate::model::Diagnostic> {
    raw.nodes
        .iter()
        .enumerate()
        .filter(|(target, node)| {
            node.provider.is_aws()
                && node.resource_type == "aws_networkfirewall_firewall"
                && node.mode == crate::model::EntityMode::Managed
                && node.deposed_key.is_none()
                && network(raw, *target).is_none()
        })
        .map(|(_, node)| crate::model::Diagnostic {
            address: node.address.clone(),
            deposed_key: None,
            attribute: SUBNETS.into(),
            reason: crate::model::DiagnosticReason::UnprovenEndpointValue,
        })
        .collect()
}

pub(super) fn present(raw: &Input, graph: &mut Graph) {
    for (target, node) in raw.nodes.iter().enumerate() {
        if node.resource_type != "aws_networkfirewall_firewall" {
            continue;
        }
        let Some((parent, subnets)) = network(raw, target) else {
            continue;
        };
        let mut evidence = vec![relationships::reference(graph, parent, target)];
        for &s in subnets {
            evidence.extend([
                relationships::reference(graph, parent, s),
                relationships::reference(graph, s, target),
            ]);
        }
        relationships::record(graph, parent, target, EdgeKind::Containment, evidence);
        if let Some(edge) = graph
            .edges
            .iter_mut()
            .find(|e| e.from == parent && e.to == target && e.kind == EdgeKind::Dependency)
        {
            edge.kind = EdgeKind::Containment;
        } else {
            graph.edges.push(Edge {
                kind: EdgeKind::Containment,
                ..Edge::from((parent, target))
            });
        }
    }
}
