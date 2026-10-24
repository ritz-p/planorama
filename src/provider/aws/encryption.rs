use crate::model::{
    Action, AttributeReference, EntityMode, Graph, Placement, PresentationOwner, ReferenceKind,
    ResourceReference, TerraformEntity, TerraformPlan,
};
use crate::semantic::Input;
use serde_json::Value;
use std::collections::BTreeMap;

const UNKNOWN: &str = "At-rest encryption (plan): unknown";
const ENABLED: &str = "At-rest encryption (plan): enabled";
const DISABLED: &str = "At-rest encryption (plan): disabled";
const KMS: &str = "At-rest encryption (plan): enabled; AWS KMS";
const S3: &str = "At-rest encryption (plan): enabled; SSE-S3 (S3-managed key)";
const SQS: &str = "At-rest encryption (plan): enabled; SSE-SQS (SQS-managed key)";
const DSSE: &str = "At-rest encryption (plan): enabled; DSSE-KMS";

#[cfg(test)]
#[path = "../../../tests/unit/semantic/encryption.rs"]
mod tests;

fn supported(kind: &str) -> bool {
    matches!(
        kind,
        "aws_s3_bucket"
            | "aws_s3_bucket_server_side_encryption_configuration"
            | "aws_sqs_queue"
            | "aws_db_instance"
            | "aws_rds_cluster"
    )
}

fn resources<'a>(module: &'a Value, found: &mut BTreeMap<&'a str, &'a Value>) {
    for resource in module["resources"].as_array().into_iter().flatten() {
        if let Some(address) = resource["address"].as_str() {
            found.insert(address, resource);
        }
    }
    for child in module["child_modules"].as_array().into_iter().flatten() {
        resources(child, found);
    }
}

fn field<'a>(
    value: &'a Value,
    unknown: &Value,
    sensitive: &Value,
    path: &[&str],
) -> Option<&'a Value> {
    if unknown == &Value::Bool(true) || sensitive == &Value::Bool(true) {
        return None;
    }
    if let Some(values) = value.as_array() {
        let [value] = values.as_slice() else {
            return None;
        };
        if unknown.as_array().is_some_and(|v| v.len() != 1)
            || sensitive.as_array().is_some_and(|v| v.len() != 1)
        {
            return None;
        }
        return field(value, &unknown[0], &sensitive[0], path);
    }
    let Some((name, rest)) = path.split_first() else {
        return Some(value);
    };
    field(value.get(*name)?, &unknown[*name], &sensitive[*name], rest)
}

fn summary(kind: &str, value: &Value, unknown: &Value, sensitive: &Value) -> &'static str {
    let get = |path: &[&str]| field(value, unknown, sensitive, path);
    match kind {
        "aws_db_instance" | "aws_rds_cluster" => {
            match get(&["storage_encrypted"]).and_then(Value::as_bool) {
                Some(true) => ENABLED,
                Some(false) => DISABLED,
                None => UNKNOWN,
            }
        }
        "aws_sqs_queue" => {
            let managed = get(&["sqs_managed_sse_enabled"]).and_then(Value::as_bool);
            let key = get(&["kms_master_key_id"]);
            let has_key = key.and_then(Value::as_str).is_some_and(|s| !s.is_empty());
            match (managed, has_key) {
                (Some(true), true) => UNKNOWN,
                (Some(true), false) => SQS,
                (_, true) => KMS,
                (Some(false), false)
                    if key.is_some_and(|v| v.is_null() || v.as_str() == Some("")) =>
                {
                    DISABLED
                }
                _ => UNKNOWN,
            }
        }
        _ => {
            let path: &[&str] = if kind == "aws_s3_bucket" {
                &[
                    "server_side_encryption_configuration",
                    "rule",
                    "apply_server_side_encryption_by_default",
                    "sse_algorithm",
                ]
            } else {
                &[
                    "rule",
                    "apply_server_side_encryption_by_default",
                    "sse_algorithm",
                ]
            };
            match get(path).and_then(Value::as_str) {
                Some("AES256") => S3,
                Some("aws:kms") => KMS,
                Some("aws:kms:dsse") => DSSE,
                _ => UNKNOWN,
            }
        }
    }
}

pub(crate) fn enrich(
    plan: &Value,
    nodes: &mut [TerraformEntity],
    references: &mut [AttributeReference],
) {
    let mut planned = BTreeMap::new();
    let mut prior = BTreeMap::new();
    resources(&plan["planned_values"]["root_module"], &mut planned);
    resources(&plan["prior_state"]["values"]["root_module"], &mut prior);
    let changes: BTreeMap<_, _> = plan["resource_changes"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|c| {
            Some((
                (c["address"].as_str()?, c["deposed"].as_str()),
                &c["change"],
            ))
        })
        .collect();
    let evidence: Vec<_> = nodes
        .iter()
        .map(|node| {
            let historical = node.action == Action::Delete || node.deposed_key.is_some();
            let change = changes.get(&(node.address.as_str(), node.deposed_key.as_deref()));
            if let Some(change) = change {
                if historical {
                    (&change["before"], &Value::Null, &change["before_sensitive"])
                } else {
                    (
                        &change["after"],
                        &change["after_unknown"],
                        &change["after_sensitive"],
                    )
                }
            } else {
                let resource = if historical { &prior } else { &planned }
                    .get(node.address.as_str())
                    .copied()
                    .unwrap_or(&Value::Null);
                (
                    &resource["values"],
                    &Value::Null,
                    &resource["sensitive_values"],
                )
            }
        })
        .collect();
    for (index, node) in nodes.iter_mut().enumerate() {
        if !node.provider.is_aws() || !supported(&node.resource_type) {
            continue;
        }
        let historical = node.action == Action::Delete || node.deposed_key.is_some();
        let (value, unknown, sensitive) = evidence[index];
        node.encryption = Some(summary(&node.resource_type, value, unknown, sensitive));
        for reference in references.iter_mut().filter(|r| r.target == index) {
            if super::service_references::label(&node.resource_type, &reference.attribute)
                != Some("Encryption key reference")
                && !(node.resource_type == "aws_s3_bucket_server_side_encryption_configuration"
                    && reference.attribute == "bucket")
            {
                continue;
            }
            if historical
                || field(
                    value,
                    unknown,
                    sensitive,
                    &reference.attribute.split('.').collect::<Vec<_>>(),
                )
                .is_none()
                || reference.sources.iter().any(|&source| {
                    let (value, unknown, sensitive) = evidence[source];
                    reference.source_attributes.iter().any(|attribute| {
                        field(
                            value,
                            unknown,
                            sensitive,
                            &attribute.split('.').collect::<Vec<_>>(),
                        )
                        .is_none()
                    })
                })
            {
                reference.complete = false;
                reference.service_reference_verified = false;
                reference.sources.clear();
            }
        }
    }
}

pub(crate) fn facts(plan: &TerraformPlan, graph: &mut Graph) {
    for (fact, node) in plan.nodes.iter().zip(&mut graph.nodes) {
        if let Some(label) = fact.encryption {
            node.references.push(ResourceReference {
                kind: ReferenceKind::Information,
                preserve_spatial: false,
                show_spatial: true,
                label,
                attribute: "at_rest_encryption".into(),
                targets: Vec::new(),
                complete: true,
                dependency: false,
                issues: Default::default(),
            });
        }
    }
}

pub(super) fn present(raw: &Input, graph: &mut Graph) {
    for (index, node) in raw.nodes.iter().enumerate() {
        if !node.provider.is_aws()
            || node.mode != EntityMode::Managed
            || node.resource_type != "aws_s3_bucket_server_side_encryption_configuration"
        {
            continue;
        }
        let mut candidates = raw
            .attributes
            .iter()
            .filter(|r| r.target == index && r.attribute == "bucket");
        let Some(reference) = candidates.next() else {
            continue;
        };
        if candidates.next().is_some() || !super::service_references::verified(raw, reference) {
            continue;
        }
        let [owner] = reference.sources.as_slice() else {
            continue;
        };
        let parent = &raw.nodes[*owner];
        if !node.presentation_scope_matches(parent)
            || raw
                .attributes
                .iter()
                .filter(|r| {
                    r.attribute == "bucket"
                        && r.sources.contains(owner)
                        && raw.nodes[r.target].resource_type == node.resource_type
                })
                .count()
                != 1
        {
            continue;
        }
        graph.nodes[index].placement = Placement::Spatial;
        graph.nodes[index].member_owner = Some(PresentationOwner {
            target: parent.entity.provenance[0].clone(),
            attribute: "bucket",
            label: "Bucket",
            section: "At-rest encryption (plan)",
        });
    }
    refresh(graph);
}

pub(crate) fn refresh(graph: &mut Graph) {
    let summaries: BTreeMap<_, _> = graph
        .nodes
        .iter()
        .filter(|node| node.resource_type == "aws_s3_bucket_server_side_encryption_configuration")
        .filter_map(|node| {
            let owner = crate::view::presentation::owner(graph, node)?;
            let summary = node
                .references
                .iter()
                .find(|r| r.attribute == "at_rest_encryption")?;
            Some((owner, summary.label))
        })
        .collect();
    for (index, node) in graph.nodes.iter_mut().enumerate() {
        if node.resource_type != "aws_s3_bucket" {
            continue;
        }
        for reference in &mut node.references {
            if reference.attribute == "at_rest_encryption_configuration"
                || (reference.attribute == "at_rest_encryption" && reference.label == UNKNOWN)
            {
                if let Some(label) = summaries.get(&index) {
                    reference.label = label;
                    reference.attribute = "at_rest_encryption_configuration".into();
                } else {
                    reference.label = UNKNOWN;
                    reference.attribute = "at_rest_encryption".into();
                }
            }
        }
    }
}
