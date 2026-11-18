use super::{eventbridge, lambda_core, nested_references, scope, service_evidence};
use crate::model::{AttributeReference, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;
use std::collections::BTreeSet;

pub(super) struct Rule {
    pub label: &'static str,
    pub outputs: &'static [(&'static str, &'static [&'static str])],
    pub list: bool,
    decoder: Option<nested_references::DecodeLeaf>,
    explicit_destination: bool,
    pub blocks: &'static [&'static str],
}

impl Rule {
    pub fn with_explicit_destination(mut self) -> Self {
        self.explicit_destination = true;
        self
    }

    pub fn decoded(mut self, decoder: nested_references::DecodeLeaf) -> Self {
        self.decoder = Some(decoder);
        self
    }
    pub fn new(
        label: &'static str,
        outputs: &'static [(&'static str, &'static [&'static str])],
        list: bool,
        blocks: &'static [&'static str],
    ) -> Self {
        Self {
            label,
            outputs,
            list,
            decoder: None,
            explicit_destination: false,
            blocks,
        }
    }
}
pub(super) type Lookup = fn(&str, &str) -> Option<Rule>;
pub(super) struct Evidence<'a> {
    pub nodes: &'a [TerraformEntity],
    pub refs: &'a [AttributeReference],
    pub values: &'a [(&'a Value, &'a Value)],
}
impl Evidence<'_> {
    pub fn value(&self, index: usize, path: &str) -> Option<&Value> {
        let (v, m) = self.values[index];
        nested_references::value(v, m, path)
    }

    pub fn text(&self, index: usize, path: &str) -> Option<&str> {
        self.value(index, path)?.as_str().filter(|s| !s.is_empty())
    }

    pub fn same_endpoint(
        &self,
        a: usize,
        a_path: &str,
        b: usize,
        b_path: &str,
        kind: &str,
        field: &str,
    ) -> bool {
        self.configured_endpoint(a, a_path, kind, &[field])
            .zip(self.configured_endpoint(b, b_path, kind, &[field]))
            .is_some_and(|(left, right)| left == right)
    }

    pub fn configured_endpoint(
        &self,
        index: usize,
        path: &str,
        kind: &str,
        fields: &[&str],
    ) -> Option<usize> {
        let reference = lambda_core::unique(self.refs, index, path)?;
        let [endpoint] = reference.sources.as_slice() else {
            return None;
        };
        let outputs = reference.source_attributes_by_instance.get(endpoint)?;
        let output = outputs.first()?;
        (outputs.len() == 1
            && fields.contains(&output.as_str())
            && self.nodes[*endpoint].resource_type == kind
            && service_evidence::eligible(self.nodes, reference)
            && self
                .text(index, path)
                .is_some_and(|value| self.text(*endpoint, output) == Some(value)))
        .then_some(*endpoint)
    }
}

pub(super) fn indexed(kind: &str, path: &str, lookup: Lookup) -> bool {
    path.split('.').any(|p| p.parse::<usize>().is_ok())
        && lookup(
            kind,
            &path
                .split('.')
                .filter(|p| p.parse::<usize>().is_err())
                .collect::<Vec<_>>()
                .join("."),
        )
        .is_some()
}

fn global(kind: &str) -> bool {
    matches!(
        kind,
        "aws_iam_role"
            | "aws_iam_user"
            | "aws_iam_policy"
            | "aws_cloudfront_distribution"
            | "aws_globalaccelerator_accelerator"
    )
}

pub(super) fn bindings(raw: &Input, r: &AttributeReference, lookup: Lookup) -> bool {
    let left = &raw.nodes[r.target];
    lookup(&left.resource_type, &r.attribute).is_none_or(|rule| {
        r.sources.iter().all(|&s| {
            let right = &raw.nodes[s];
            if rule.explicit_destination {
                return destination_bindings(
                    left.provider_configuration.as_ref(),
                    right.provider_configuration.as_ref(),
                );
            }
            eventbridge::bindings(
                left.provider_configuration.as_ref(),
                right.provider_configuration.as_ref(),
                left.entity.scope.as_ref().and_then(|c| c.region.as_ref()),
                right.entity.scope.as_ref().and_then(|c| c.region.as_ref()),
                global(&right.resource_type),
            )
        })
    })
}

fn destination_bindings(
    left: Option<&crate::model::ProviderConfiguration>,
    right: Option<&crate::model::ProviderConfiguration>,
) -> bool {
    left.zip(right)
        .is_some_and(|(left, right)| !left.account_conflict && !right.account_conflict)
}

pub(super) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    values: &[(&Value, &Value)],
    lookup: Lookup,
    validate: fn(&Evidence<'_>, &AttributeReference, usize, &str) -> bool,
) {
    let e = Evidence {
        nodes,
        refs,
        values,
    };
    let results: Vec<_> = refs
        .iter()
        .map(|r| {
            let node = &nodes[r.target];
            let rule = lookup(&node.resource_type, &r.attribute)?;
            let fields = |_: &str, _: &str, src: &str| {
                rule.outputs
                    .iter()
                    .find(|(k, _)| *k == src)
                    .map_or(&[][..], |(_, v)| *v)
            };
            if rule
                .outputs
                .iter()
                .any(|(kind, _)| *kind == "aws_ssoadmin_instances")
            {
                let path = r.attribute.split('.').collect::<Vec<_>>().join(".0.");
                let depth = r.attribute.split('.').count() - 1;
                let verified = lambda_core::unique(refs, r.target, &r.attribute).is_some()
                    && r.instances_explicit
                    && r.issues.is_empty()
                    && lambda_core::unique(refs, r.target, &path).is_some_and(|leaf| {
                        let [s] = leaf.sources.as_slice() else {
                            return false;
                        };
                        leaf.sources == r.sources
                            && leaf.indexed_block_lengths == vec![1; depth]
                            && validate(&e, r, *s, &path)
                            && eventbridge::bindings(
                                node.provider_configuration.as_ref(),
                                nodes[*s].provider_configuration.as_ref(),
                                scope::scope(node).as_ref().and_then(|v| v.region.as_ref()),
                                scope::scope(&nodes[*s])
                                    .as_ref()
                                    .and_then(|v| v.region.as_ref()),
                                false,
                            )
                            && e.text(r.target, &path).is_some_and(|v| {
                                super::identity_center_instance::verify(nodes, leaf, values, v)
                            })
                    });
                return Some((verified, Vec::new()));
            }
            let verified = nested_references::verify_values(
                nodes,
                refs,
                values,
                r,
                fields,
                |s, path| {
                    let endpoint = &nodes[s];
                    let shape = path.split('.').count() == 2 * r.attribute.split('.').count() - 1
                        && path
                            .split('.')
                            .skip(1)
                            .step_by(2)
                            .all(|part| part.parse::<usize>().is_ok());
                    shape
                        && e.value(r.target, path).is_some_and(|v| {
                            if let Some(decode) = rule.decoder {
                                decode(&r.attribute, v).is_some()
                            } else if rule.list {
                                v.is_array()
                            } else {
                                v.is_string()
                            }
                        })
                        && if rule.explicit_destination {
                            destination_bindings(
                                node.provider_configuration.as_ref(),
                                endpoint.provider_configuration.as_ref(),
                            )
                        } else {
                            eventbridge::bindings(
                                node.provider_configuration.as_ref(),
                                endpoint.provider_configuration.as_ref(),
                                scope::scope(node).as_ref().and_then(|c| c.region.as_ref()),
                                scope::scope(endpoint)
                                    .as_ref()
                                    .and_then(|c| c.region.as_ref()),
                                global(&endpoint.resource_type),
                            )
                        }
                        && validate(&e, r, s, path)
                },
                (
                    rule.blocks,
                    rule.decoder.unwrap_or(nested_references::leaf_values),
                ),
            )
            .is_some();
            let context = if verified
                && rule
                    .outputs
                    .iter()
                    .all(|(k, _)| matches!(*k, "aws_subnet" | "aws_security_group"))
            {
                r.sources
                    .iter()
                    .map(|&s| lambda_core::parent(nodes, refs, values, s))
                    .collect::<Option<BTreeSet<_>>>()
                    .unwrap_or_default()
                    .into_iter()
                    .collect()
            } else {
                Vec::new()
            };
            Some((verified, context))
        })
        .collect();
    for (r, result) in refs.iter_mut().zip(results) {
        if let Some((valid, parents)) = result {
            r.service_reference_verified = valid;
            if valid {
                r.complete = true;
            }
            r.endpoint_network_context = parents;
        }
    }
}
