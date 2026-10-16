use crate::model::{AttributeReference, TerraformEntity};
use serde_json::Value;
use std::collections::BTreeMap;

pub(super) fn bus_references(references: &[AttributeReference]) -> BTreeMap<usize, Option<usize>> {
    references
        .iter()
        .filter(|r| r.attribute == "event_bus_name")
        .map(|r| {
            let source = match r.sources.as_slice() {
                [source] if r.complete && r.instances_explicit && r.issues.is_empty() => {
                    Some(*source)
                }
                _ => None,
            };
            (r.target, source)
        })
        .collect()
}

pub(super) fn rule_verified(
    reference: &AttributeReference,
    nodes: &[TerraformEntity],
    evidence: &[(&Value, &Value)],
    buses: &BTreeMap<usize, Option<usize>>,
) -> bool {
    let [rule] = reference.sources.as_slice() else {
        return false;
    };
    if !reference.complete
        || !reference.instances_explicit
        || !reference.issues.is_empty()
        || !nodes[*rule].provider.is_aws()
        || nodes[*rule].resource_type != "aws_cloudwatch_event_rule"
        || nodes[*rule].deposed_key.is_some()
    {
        return false;
    }
    let strings = |index: usize, attribute: &str| {
        let (value, unknown) = evidence[index];
        super::known_strings(value, unknown, &[attribute])
    };
    let name = strings(*rule, "name");
    if name.is_none() || strings(reference.target, "rule") != name {
        return false;
    }
    let actual = strings(reference.target, "event_bus_name");
    let rule_bus = strings(*rule, "event_bus_name");
    match (buses.get(&reference.target), buses.get(rule)) {
        (Some(Some(bus)), Some(Some(rule_bus_index))) if bus == rule_bus_index => {
            let node = &nodes[*bus];
            let name = strings(*bus, "name");
            let arn = strings(*bus, "arn");
            let matches =
                |value: &Option<Vec<&str>>| value.is_some() && (value == &name || value == &arn);
            node.provider.is_aws()
                && node.resource_type == "aws_cloudwatch_event_bus"
                && node.deposed_key.is_none()
                && matches(&actual)
                && matches(&rule_bus)
        }
        (None, None) => {
            actual.as_deref() == Some(["default"].as_slice())
                && rule_bus.as_deref() == Some(["default"].as_slice())
        }
        _ => false,
    }
}
