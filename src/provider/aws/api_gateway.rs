use super::service_evidence::{eligible_with_global_role, known};
use crate::model::{AttributeReference, TerraformEntity};
use serde_json::Value;
mod rest;
mod v2;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/api_gateway.rs"]
mod tests;

pub(super) struct Rule {
    pub(super) label: &'static str,
    targets: &'static [(&'static str, &'static str)],
    list: bool,
}

pub(super) fn rule(resource: &str, attribute: &str) -> Option<Rule> {
    rest::rule(resource, attribute).or_else(|| v2::rule(resource, attribute))
}

fn basic(
    nodes: &[TerraformEntity],
    r: &AttributeReference,
    evidence: &[(&Value, &Value)],
    rule: &Rule,
) -> bool {
    if !eligible_with_global_role(
        nodes,
        r,
        nodes[r.target].resource_type == "aws_api_gateway_integration"
            && r.attribute == "credentials",
    ) || r.source_attributes.len() != 1
    {
        return false;
    }
    let field = r.source_attributes.iter().next().unwrap();
    let expected: Option<Vec<_>> = r
        .sources
        .iter()
        .map(|&source| {
            rule.targets
                .contains(&(super::aliases::entity_type(&nodes[source]), field.as_str()))
                .then(|| known(evidence, source, field))
                .flatten()
        })
        .collect();
    let Some(mut expected) = expected else {
        return false;
    };
    if !rule.list {
        return expected.len() == 1
            && known(evidence, r.target, &r.attribute) == expected.first().copied();
    }
    let (value, unknown) = evidence[r.target];
    if !matches!(unknown, Value::Null | Value::Bool(false) | Value::Object(_)) {
        return false;
    }
    let Some(values) = value[&r.attribute].as_array() else {
        return false;
    };
    let mask = &unknown[&r.attribute];
    if !matches!(mask, Value::Null | Value::Bool(false))
        && !mask.as_array().is_some_and(|m| {
            m.len() == values.len()
                && m.iter()
                    .all(|v| matches!(v, Value::Null | Value::Bool(false)))
        })
    {
        return false;
    }
    let Some(mut actual) = values.iter().map(Value::as_str).collect::<Option<Vec<_>>>() else {
        return false;
    };
    actual.sort_unstable();
    expected.sort_unstable();
    !actual.is_empty() && actual == expected && expected.windows(2).all(|pair| pair[0] != pair[1])
}

struct Context<'a> {
    nodes: &'a [TerraformEntity],
    references: &'a [AttributeReference],
    evidence: &'a [(&'a Value, &'a Value)],
    base: Vec<bool>,
}

impl Context<'_> {
    fn source(&self, target: usize, attribute: &str) -> Option<usize> {
        let mut candidates = self
            .references
            .iter()
            .enumerate()
            .filter(|(_, r)| r.target == target && r.attribute == attribute);
        let (i, r) = candidates.next()?;
        if candidates.next().is_some() || !self.base[i] {
            return None;
        }
        let [source] = r.sources.as_slice() else {
            return None;
        };
        Some(*source)
    }

    fn value(&self, target: usize, attribute: &str) -> Option<&str> {
        known(self.evidence, target, attribute)
    }

    fn lambda(&self, target: usize) -> bool {
        match self.nodes[target].resource_type.as_str() {
            "aws_lambda_function" => true,
            "aws_lambda_alias" => {
                let mut owners = self
                    .references
                    .iter()
                    .filter(|r| r.target == target && r.attribute == "function_name");
                owners.next().is_some_and(|r| r.service_reference_verified)
                    && owners.next().is_none()
            }
            _ => false,
        }
    }

    fn listener(&self, target: usize) -> bool {
        let mut owners = self
            .references
            .iter()
            .filter(|r| r.target == target && r.attribute == "load_balancer_arn");
        let Some(owner) = owners.next() else {
            return false;
        };
        owners.next().is_none()
            && basic(
                self.nodes,
                owner,
                self.evidence,
                &Rule {
                    label: "Listener owner",
                    targets: &[("aws_lb", "arn")],
                    list: false,
                },
            )
            && owner.sources.iter().all(|&source| {
                matches!(
                    self.value(source, "load_balancer_type"),
                    Some("application" | "network")
                )
            })
    }

    fn known_false(&self, target: usize, key: &str) -> bool {
        let (value, unknown) = self.evidence[target];
        matches!(unknown, Value::Null | Value::Bool(false) | Value::Object(_))
            && matches!(&unknown[key], Value::Null | Value::Bool(false))
            && value[key] == Value::Bool(false)
    }
}

pub(super) fn annotate(
    nodes: &[TerraformEntity],
    references: &mut [AttributeReference],
    evidence: &[(&Value, &Value)],
) {
    let context = Context {
        nodes,
        references,
        evidence,
        base: references
            .iter()
            .map(|r| {
                rule(&nodes[r.target].resource_type, &r.attribute)
                    .is_some_and(|rule| basic(nodes, r, evidence, &rule))
            })
            .collect(),
    };
    let verified: Vec<_> = references
        .iter()
        .enumerate()
        .map(|(i, r)| context.base[i] && rest::valid(&context, r) && v2::valid(&context, r))
        .collect();
    for (r, verified) in references.iter_mut().zip(verified) {
        if rule(&nodes[r.target].resource_type, &r.attribute).is_some() {
            r.service_reference_verified = verified;
        }
    }
}
