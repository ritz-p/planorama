use crate::model::{AttributeReference, TerraformEntity};
use serde_json::Value;
use std::collections::BTreeMap;

mod eventbridge;

#[cfg(test)]
#[path = "../../tests/unit/plan/collections.rs"]
mod tests;

#[derive(PartialEq, Eq, PartialOrd, Ord)]
enum Id<'a> {
    Known(&'a str),
    Unknown,
}

fn resources<'a>(module: &'a Value, found: &mut BTreeMap<&'a str, &'a Value>) {
    for resource in module["resources"].as_array().into_iter().flatten() {
        if let Some(address) = resource["address"].as_str() {
            found.insert(address, &resource["values"]);
        }
    }
    for child in module["child_modules"].as_array().into_iter().flatten() {
        resources(child, found);
    }
}

fn sensitivities<'a>(module: &'a Value, found: &mut BTreeMap<&'a str, &'a Value>) {
    for resource in module["resources"].as_array().into_iter().flatten() {
        if let Some(address) = resource["address"].as_str() {
            found.insert(address, &resource["sensitive_values"]);
        }
    }
    for child in module["child_modules"].as_array().into_iter().flatten() {
        sensitivities(child, found);
    }
}

fn merge_masks(left: &Value, right: &Value) -> Value {
    if left == &Value::Bool(true) || right == &Value::Bool(true) {
        return Value::Bool(true);
    }
    match (left, right) {
        (Value::Object(left), Value::Object(right)) => {
            let mut merged = left.clone();
            for (key, value) in right {
                merged.insert(
                    key.clone(),
                    merge_masks(merged.get(key).unwrap_or(&Value::Null), value),
                );
            }
            Value::Object(merged)
        }
        (Value::Array(left), Value::Array(right)) if left.len() == right.len() => Value::Array(
            left.iter()
                .zip(right)
                .map(|(l, r)| merge_masks(l, r))
                .collect(),
        ),
        (Value::Null | Value::Bool(false), _) => right.clone(),
        (_, Value::Null | Value::Bool(false)) => left.clone(),
        _ => Value::Bool(true),
    }
}

fn id<'a>(value: &'a Value, unknown: &Value) -> Option<Id<'a>> {
    if unknown.as_bool() == Some(true) {
        Some(Id::Unknown)
    } else {
        value.as_str().map(Id::Known)
    }
}

fn elements<'a>(value: &'a Value, unknown: &'a Value) -> Option<Vec<(&'a Value, &'a Value)>> {
    if unknown.as_bool() == Some(true) {
        return None;
    }
    let values = value.as_array();
    let masks = unknown.as_array();
    let len = values.or(masks)?.len();
    if values.zip(masks).is_some_and(|(a, b)| a.len() != b.len()) {
        return None;
    }
    Some(
        (0..len)
            .map(|i| {
                (
                    values.and_then(|v| v.get(i)).unwrap_or(&Value::Null),
                    masks.and_then(|v| v.get(i)).unwrap_or(&Value::Null),
                )
            })
            .collect(),
    )
}

fn collection<'a>(value: &'a Value, unknown: &'a Value, path: &[&str]) -> Option<Vec<Id<'a>>> {
    if path.is_empty() {
        return elements(value, unknown)?
            .into_iter()
            .map(|(v, u)| id(v, u))
            .collect();
    }
    if value.is_array() || unknown.is_array() {
        return elements(value, unknown)?
            .into_iter()
            .map(|(v, u)| collection(v, u, path))
            .collect::<Option<Vec<_>>>()
            .map(|parts| parts.into_iter().flatten().collect());
    }
    if unknown.as_bool() == Some(true) {
        return None;
    }
    collection(&value[path[0]], &unknown[path[0]], &path[1..])
}

fn known_strings<'a>(value: &'a Value, unknown: &'a Value, path: &[&str]) -> Option<Vec<&'a str>> {
    if unknown.as_bool() == Some(true) {
        return None;
    }
    if path.is_empty() {
        return value.as_str().filter(|s| !s.is_empty()).map(|s| vec![s]);
    }
    if value.is_array() || unknown.is_array() {
        return elements(value, unknown)?
            .into_iter()
            .map(|(v, u)| known_strings(v, u, path))
            .collect::<Option<Vec<_>>>()
            .map(|parts| parts.into_iter().flatten().collect());
    }
    known_strings(&value[path[0]], &unknown[path[0]], &path[1..])
}

pub(super) fn annotate(
    plan: &Value,
    nodes: &mut [TerraformEntity],
    references: &mut [AttributeReference],
) {
    let mut values = BTreeMap::new();
    let mut prior = BTreeMap::new();
    resources(&plan["prior_state"]["values"]["root_module"], &mut prior);
    resources(&plan["planned_values"]["root_module"], &mut values);
    let changes: BTreeMap<_, _> = plan["resource_changes"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|change| {
            change["address"]
                .as_str()
                .map(|address| ((address, change["deposed"].as_str()), change))
        })
        .collect();
    let evidence: Vec<_> = nodes
        .iter()
        .map(|node| {
            let change = changes
                .get(&(node.address.as_str(), node.deposed_key.as_deref()))
                .copied();
            let planned = values.get(node.address.as_str()).copied().or_else(|| {
                (node.mode == crate::model::EntityMode::Data)
                    .then(|| prior.get(node.address.as_str()).copied())
                    .flatten()
            });
            (
                change
                    .and_then(|c| c["change"].get("after"))
                    .or(planned)
                    .unwrap_or(&Value::Null),
                change
                    .map(|c| &c["change"]["after_unknown"])
                    .unwrap_or(&Value::Null),
            )
        })
        .collect();
    let before_evidence: Vec<_> = nodes
        .iter()
        .map(|node| {
            let value = changes
                .get(&(node.address.as_str(), node.deposed_key.as_deref()))
                .and_then(|c| c["change"].get("before"))
                .or_else(|| {
                    node.deposed_key
                        .is_none()
                        .then(|| prior.get(node.address.as_str()).copied())
                        .flatten()
                })
                .unwrap_or(&Value::Null);
            (value, &Value::Null)
        })
        .collect();
    let buses = eventbridge::bus_references(references);
    crate::provider::aws::waf::enrich(nodes, &evidence, &before_evidence);
    crate::provider::aws::organizations::annotate(nodes, references, &evidence);
    crate::provider::aws::organization_policies::annotate(
        nodes,
        references,
        &evidence,
        &before_evidence,
    );
    crate::provider::aws::service_references::annotate(nodes, references, &evidence);
    let mut sensitive = BTreeMap::new();
    sensitivities(&plan["planned_values"]["root_module"], &mut sensitive);
    let masks: Vec<_> = nodes
        .iter()
        .zip(&evidence)
        .map(|(node, (_, unknown))| {
            let change = changes.get(&(node.address.as_str(), node.deposed_key.as_deref()));
            let mask = merge_masks(
                unknown,
                change
                    .map(|c| &c["change"]["after_sensitive"])
                    .unwrap_or(&Value::Null),
            );
            merge_masks(
                &mask,
                sensitive
                    .get(node.address.as_str())
                    .copied()
                    .unwrap_or(&Value::Null),
            )
        })
        .collect();
    let elbv2_evidence: Vec<_> = evidence
        .iter()
        .zip(&masks)
        .map(|((value, _), mask)| (*value, mask))
        .collect();
    crate::provider::aws::route53::annotate(nodes, references, &elbv2_evidence);
    crate::provider::aws::resolver::annotate(nodes, references, &elbv2_evidence);
    crate::provider::aws::route53_records::annotate(nodes, references, &elbv2_evidence);
    crate::provider::aws::cloudfront::annotate(nodes, references, &elbv2_evidence);

    crate::provider::aws::eni::annotate(nodes, references, &elbv2_evidence);
    crate::provider::aws::peering::annotate(nodes, references, &elbv2_evidence);
    crate::provider::aws::elbv2_network::annotate(nodes, references, &elbv2_evidence);
    crate::provider::aws::cloudfront_vpc_origin::annotate(nodes, references, &elbv2_evidence);
    crate::provider::aws::elbv2::annotate(nodes, references, &elbv2_evidence);
    crate::provider::aws::elbv2_cognito::annotate(nodes, references, &elbv2_evidence);
    crate::provider::aws::lambda_config::annotate(nodes, references, &elbv2_evidence);
    crate::provider::aws::lambda_qualified::annotate(nodes, references, &elbv2_evidence);
    crate::provider::aws::lambda_events::annotate(nodes, references, &elbv2_evidence);
    for reference in references {
        let consumer = &nodes[reference.target];
        let evidence = if consumer.provider.is_aws()
            && matches!(
                consumer.resource_type.as_str(),
                "aws_sns_topic_subscription"
                    | "aws_cloudwatch_event_target"
                    | "aws_lambda_event_source_mapping"
                    | "aws_rds_cluster_instance"
            )
            && consumer.action == crate::model::Action::Delete
        {
            &before_evidence
        } else {
            &evidence
        };
        let alarm = &nodes[reference.target];
        if alarm.provider.is_aws()
            && alarm.resource_type == "aws_cloudwatch_event_target"
            && reference.attribute == "rule"
        {
            reference.messaging_endpoint_verified =
                eventbridge::rule_verified(reference, nodes, evidence, &buses);
        }
        if alarm.provider.is_aws()
            && alarm.resource_type == "aws_cloudwatch_metric_alarm"
            && matches!(
                reference.attribute.as_str(),
                "dimensions" | "dimensions.DBInstanceIdentifier"
            )
            && reference.complete
            && reference.instances_explicit
            && !reference
                .issues
                .contains(&crate::model::DiagnosticReason::DynamicInstanceSelection)
        {
            if let [source] = reference.sources.as_slice() {
                let resource = &nodes[*source];
                let (value, unknown) = evidence[reference.target];
                let (target, target_unknown) = evidence[*source];
                let namespace = known_strings(value, unknown, &["namespace"]);
                let dimension =
                    known_strings(value, unknown, &["dimensions", "DBInstanceIdentifier"]);
                let identifier = known_strings(target, target_unknown, &["identifier"]);
                reference.monitoring_verified = resource.provider.is_aws()
                    && resource.resource_type == "aws_db_instance"
                    && resource.provider_configuration == alarm.provider_configuration
                    && resource.deposed_key.is_none()
                    && namespace == Some(vec!["AWS/RDS"])
                    && value["dimensions"]
                        .as_object()
                        .is_some_and(|d| d.len() == 1)
                    && dimension.is_some()
                    && dimension == identifier
                    && unknown["dimensions"]
                        .as_object()
                        .is_none_or(|d| d.values().all(|v| v == &Value::Bool(false)))
                    && known_strings(value, unknown, &["metric_name"]).is_some()
                    && (matches!(
                        unknown.get("metric_query"),
                        None | Some(Value::Null) | Some(Value::Bool(false))
                    ) || unknown["metric_query"]
                        .as_array()
                        .is_some_and(Vec::is_empty))
                    && value
                        .get("metric_query")
                        .is_none_or(|q| q.is_null() || q.as_array().is_some_and(Vec::is_empty));
            }
        }
        if reference.complete && reference.instances_explicit && !reference.sources.is_empty() {
            let expected = reference
                .sources
                .iter()
                .map(|&source| {
                    let (value, unknown) = evidence[source];
                    known_strings(value, unknown, &["arn"])
                })
                .collect::<Option<Vec<_>>>()
                .map(|parts| parts.into_iter().flatten().collect::<Vec<_>>());
            let (value, unknown) = evidence[reference.target];
            let observed = known_strings(
                value,
                unknown,
                &reference.attribute.split('.').collect::<Vec<_>>(),
            );
            if let (Some(mut expected), Some(mut observed)) = (expected, observed) {
                expected.sort_unstable();
                expected.dedup();
                observed.sort_unstable();
                observed.dedup();
                reference.arn_values_match = !expected.is_empty() && expected == observed;
            }
        }
        if reference.complete
            && reference.instances_explicit
            && reference.sources.len() == 1
            && !reference
                .issues
                .contains(&crate::model::DiagnosticReason::DynamicInstanceSelection)
            && !reference.attribute.contains('.')
        {
            let source = reference.sources[0];
            let (value, unknown) = evidence[reference.target];
            let (source_value, source_unknown) = evidence[source];
            if let (Some(actual), Some(expected)) = (
                known_strings(value, unknown, &[reference.attribute.as_str()]),
                known_strings(source_value, source_unknown, &["name"]),
            ) {
                reference.scalar_name_matches =
                    actual.len() == 1 && expected.len() == 1 && actual == expected;
            }
        }
        if alarm.provider.is_aws()
            && alarm.resource_type == "aws_sns_topic_subscription"
            && reference.attribute == "endpoint"
            && reference.arn_values_match
        {
            if let [source] = reference.sources.as_slice() {
                let (value, unknown) = evidence[reference.target];
                reference.messaging_endpoint_verified = matches!(
                    (
                        known_strings(value, unknown, &["protocol"]).as_deref(),
                        nodes[*source].resource_type.as_str()
                    ),
                    (Some(["sqs"]), "aws_sqs_queue") | (Some(["lambda"]), "aws_lambda_function")
                );
            }
        }
        if alarm.provider.is_aws()
            && alarm.resource_type == "aws_lambda_event_source_mapping"
            && reference.attribute == "function_name"
            && reference.complete
            && reference.instances_explicit
        {
            if let [source] = reference.sources.as_slice() {
                let (value, unknown) = evidence[reference.target];
                let (function, function_unknown) = evidence[*source];
                let actual = known_strings(value, unknown, &["function_name"]);
                let expected = known_strings(function, function_unknown, &["function_name"]);
                reference.messaging_endpoint_verified =
                    reference.arn_values_match || (actual.is_some() && actual == expected);
            }
        }
        if !reference.complete || !reference.ids_only || reference.sources.is_empty() {
            continue;
        }
        let expected: Option<Vec<_>> = reference
            .sources
            .iter()
            .map(|&source| {
                let (value, unknown) = evidence[source];
                id(&value["id"], &unknown["id"])
            })
            .collect();
        let (value, unknown) = evidence[reference.target];
        if let [source] = reference.sources.as_slice() {
            let (source_value, source_unknown) = evidence[*source];
            if !reference.attribute.contains('.')
                && unknown != &Value::Bool(true)
                && source_unknown != &Value::Bool(true)
            {
                if let (Some(Id::Known(actual)), Some(Id::Known(expected))) = (
                    id(&value[&reference.attribute], &unknown[&reference.attribute]),
                    id(&source_value["id"], &source_unknown["id"]),
                ) {
                    reference.scalar_id_matches = !actual.is_empty() && actual == expected;
                }
            }
        }
        let observed = collection(
            value,
            unknown,
            &reference.attribute.split('.').collect::<Vec<_>>(),
        );
        if let (Some(mut expected), Some(mut observed)) = (expected, observed) {
            expected.sort();
            observed.sort();
            reference.collection_ids_complete = expected == observed;
        }
    }
}
