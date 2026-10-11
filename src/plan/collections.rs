use crate::model::{AttributeReference, TerraformEntity};
use serde_json::Value;
use std::collections::BTreeMap;

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
    nodes: &[TerraformEntity],
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
    for reference in references {
        let alarm = &nodes[reference.target];
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
