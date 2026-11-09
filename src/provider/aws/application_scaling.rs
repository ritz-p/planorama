use super::service_evidence::{eligible, eligible_with_global_role, known};
use crate::model::{AttributeReference, TerraformEntity};
use serde_json::Value;
use std::collections::BTreeSet;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/application_scaling.rs"]
mod tests;

pub(super) fn label(resource: &str, field: &str) -> Option<&'static str> {
    match (resource, field) {
        ("aws_appautoscaling_target", "resource_id") => Some("Scalable resource"),
        ("aws_appautoscaling_target", "role_arn") => Some("Scaling role"),
        (
            "aws_appautoscaling_policy" | "aws_appautoscaling_scheduled_action",
            "resource_id" | "service_namespace" | "scalable_dimension",
        ) => Some("Qualified scaling target"),
        ("aws_autoscalingplans_scaling_plan", "application_source.cloudformation_stack_arn") => {
            Some("Scaling application stack")
        }
        ("aws_autoscalingplans_scaling_plan", "scaling_instruction.resource_id") => {
            Some("Scaling instruction resources")
        }
        _ => None,
    }
}

fn supported(namespace: &str, dimension: &str, plan: bool) -> bool {
    matches!(
        (namespace, dimension),
        ("ecs", "ecs:service:DesiredCount")
            | ("rds", "rds:cluster:ReadReplicaCount")
            | (
                "dynamodb",
                "dynamodb:table:ReadCapacityUnits"
                    | "dynamodb:table:WriteCapacityUnits"
                    | "dynamodb:index:ReadCapacityUnits"
                    | "dynamodb:index:WriteCapacityUnits"
            )
    ) || plan
        && namespace == "autoscaling"
        && dimension == "autoscaling:autoScalingGroup:DesiredCapacity"
}

fn unknown_free(mask: &Value) -> bool {
    match mask {
        Value::Null | Value::Bool(false) => true,
        Value::Array(v) => v.iter().all(unknown_free),
        Value::Object(v) => v.values().all(unknown_free),
        _ => false,
    }
}

struct Context<'a> {
    nodes: &'a [TerraformEntity],
    refs: &'a [AttributeReference],
    evidence: &'a [(&'a Value, &'a Value)],
}

impl Context<'_> {
    fn tuple(&self, index: usize) -> Option<(&str, &str, &str)> {
        Some((
            known(self.evidence, index, "service_namespace")?,
            known(self.evidence, index, "scalable_dimension")?,
            known(self.evidence, index, "resource_id")?,
        ))
    }

    fn cluster(&self, service: usize, cluster: usize) -> bool {
        let refs: Vec<_> = self
            .refs
            .iter()
            .filter(|r| r.target == service && r.attribute == "cluster")
            .collect();
        let [r] = refs.as_slice() else {
            return false;
        };
        if !eligible(self.nodes, r) || r.sources != [cluster] || r.source_attributes.len() != 1 {
            return false;
        }
        let field = r.source_attributes.iter().next().unwrap();
        matches!(field.as_str(), "name" | "id" | "arn")
            && known(self.evidence, cluster, field)
                .is_some_and(|v| known(self.evidence, service, "cluster") == Some(v))
    }

    fn formatted(
        &self,
        r: &AttributeReference,
        namespace: &str,
        dimension: &str,
        id: &str,
        plan: bool,
    ) -> Option<(BTreeSet<usize>, BTreeSet<String>)> {
        if !supported(namespace, dimension, plan) {
            return None;
        }
        let mut matches = Vec::new();
        for &source in &r.sources {
            let kind = self.nodes[source].resource_type.as_str();
            let fields: &[&str] = match (namespace, kind) {
                ("dynamodb", "aws_dynamodb_table")
                | ("ecs", "aws_ecs_service")
                | ("autoscaling", "aws_autoscaling_group") => &["name"],
                ("rds", "aws_rds_cluster") => &["id", "cluster_identifier"],
                _ => continue,
            };
            let mut used = BTreeSet::new();
            let mut sources = BTreeSet::from([source]);
            for &field in fields {
                if !r.source_attributes.contains(field) {
                    continue;
                }
                let Some(name) = known(self.evidence, source, field) else {
                    continue;
                };
                let matched = match namespace {
                    "dynamodb" if dimension.starts_with("dynamodb:table:") => {
                        id == format!("table/{name}")
                    }
                    "dynamodb" => {
                        let (value, unknown) = self.evidence[source];
                        matches!(unknown, Value::Null | Value::Bool(false) | Value::Object(_))
                            && unknown_free(&unknown["global_secondary_index"])
                            && value["global_secondary_index"]
                                .as_array()
                                .is_some_and(|indexes| {
                                    indexes
                                        .iter()
                                        .filter(|i| {
                                            i["name"].as_str().is_some_and(|index| {
                                                id == format!("table/{name}/index/{index}")
                                            })
                                        })
                                        .count()
                                        == 1
                                })
                    }
                    "rds" => id == format!("cluster:{name}"),
                    "autoscaling" => id == format!("autoScalingGroup/{name}"),
                    "ecs" => {
                        let clusters: Vec<_> = r
                            .sources
                            .iter()
                            .copied()
                            .filter(|&cluster| {
                                self.nodes[cluster].resource_type == "aws_ecs_cluster"
                                    && known(self.evidence, cluster, "name")
                                        .is_some_and(|c| id == format!("service/{c}/{name}"))
                                    && self.cluster(source, cluster)
                            })
                            .collect();
                        if let [cluster] = clusters.as_slice() {
                            sources.insert(*cluster);
                            true
                        } else {
                            false
                        }
                    }
                    _ => false,
                };
                if matched {
                    used.insert(field.to_string());
                }
            }
            if !used.is_empty() {
                matches.push((sources, used));
            }
        }
        if matches.len() == 1 {
            matches.pop()
        } else {
            None
        }
    }

    fn valid(&self, r: &AttributeReference) -> bool {
        let kind = self.nodes[r.target].resource_type.as_str();
        if !eligible_with_global_role(self.nodes, r, r.attribute == "role_arn") {
            return false;
        }
        if r.attribute == "role_arn" || r.attribute == "application_source.cloudformation_stack_arn"
        {
            let [source] = r.sources.as_slice() else {
                return false;
            };
            if r.source_attributes.len() != 1 {
                return false;
            }
            let field = r.source_attributes.iter().next().unwrap();
            let valid = if r.attribute == "role_arn" {
                self.nodes[*source].resource_type == "aws_iam_role" && field == "arn"
            } else {
                self.nodes[*source].resource_type == "aws_cloudformation_stack"
                    && matches!(field.as_str(), "arn" | "id")
            };
            return valid
                && known(self.evidence, *source, field).is_some_and(|v| {
                    super::streaming::values(
                        self.evidence[r.target].0,
                        self.evidence[r.target].1,
                        &r.attribute,
                        false,
                    )
                    .is_some_and(|actual| actual == BTreeSet::from([v]))
                });
        }
        if matches!(
            kind,
            "aws_appautoscaling_policy" | "aws_appautoscaling_scheduled_action"
        ) {
            let [source] = r.sources.as_slice() else {
                return false;
            };
            if self.nodes[*source].resource_type != "aws_appautoscaling_target"
                || r.source_attributes.len() != 1
                || !r.source_attributes.contains(&r.attribute)
            {
                return false;
            }
            return self.tuple(r.target).is_some_and(|tuple| {
                supported(tuple.0, tuple.1, false) && self.tuple(*source) == Some(tuple)
            });
        }
        let mut sources = BTreeSet::new();
        let mut fields = BTreeSet::new();
        if kind == "aws_appautoscaling_target" {
            let Some((ns, dim, id)) = self.tuple(r.target) else {
                return false;
            };
            let Some((matched, used)) = self.formatted(r, ns, dim, id, false) else {
                return false;
            };
            sources.extend(matched);
            fields.extend(used);
        } else {
            let (value, unknown) = self.evidence[r.target];
            if !matches!(unknown, Value::Null | Value::Bool(false) | Value::Object(_))
                || !unknown_free(&unknown["scaling_instruction"])
            {
                return false;
            }
            let Some(blocks) = value["scaling_instruction"]
                .as_array()
                .filter(|b| !b.is_empty())
            else {
                return false;
            };
            let mut tuples = BTreeSet::new();
            for block in blocks {
                let (Some(ns), Some(dim), Some(id)) = (
                    block["service_namespace"].as_str(),
                    block["scalable_dimension"].as_str(),
                    block["resource_id"].as_str(),
                ) else {
                    return false;
                };
                if !tuples.insert((ns, dim, id)) {
                    return false;
                }
                let Some((matched, used)) = self.formatted(r, ns, dim, id, true) else {
                    return false;
                };
                sources.extend(matched);
                fields.extend(used);
            }
        }
        sources == r.sources.iter().copied().collect() && fields == r.source_attributes
    }
}

pub(super) fn annotate(
    nodes: &[TerraformEntity],
    references: &mut [AttributeReference],
    evidence: &[(&Value, &Value)],
) {
    let context = Context {
        nodes,
        refs: references,
        evidence,
    };
    let verified: Vec<_> = references
        .iter()
        .map(|r| label(&nodes[r.target].resource_type, &r.attribute).map(|_| context.valid(r)))
        .collect();
    for (r, result) in references.iter_mut().zip(verified) {
        if let Some(valid) = result {
            r.service_reference_verified = valid;
        }
    }
}
