use super::{lambda_core, nested_references};
use crate::model::{AttributeReference, Edge, EdgeKind, Graph, TerraformEntity};
use crate::semantic::{Input, relationships};
use serde_json::Value;
use std::collections::BTreeSet;
const CLUSTER: &str = "aws_emr_cluster";
const STUDIO: &str = "aws_emr_studio";
const MAPPING: &str = "aws_emr_studio_session_mapping";
const VIRTUAL: &str = "aws_emrcontainers_virtual_cluster";
const TEMPLATE: &str = "aws_emrcontainers_job_template";
const TEMPLATE_LOG: &str = "job_template_data.configuration_overrides.monitoring_configuration.cloud_watch_monitoring_configuration.log_group_name";
const TEMPLATE_BUCKET: &str = "job_template_data.configuration_overrides.monitoring_configuration.s3_monitoring_configuration.log_uri";
const SERVERLESS: &str = "aws_emrserverless_application";
const NET_SUBNETS: &str = "network_configuration.subnet_ids";
const NET_GROUPS: &str = "network_configuration.security_group_ids";
const CW_LOG: &str = "monitoring_configuration.cloudwatch_logging_configuration.log_group_name";
const CW_KEY: &str = "monitoring_configuration.cloudwatch_logging_configuration.encryption_key_arn";
const S3_URI: &str = "monitoring_configuration.s3_monitoring_configuration.log_uri";
const S3_KEY: &str = "monitoring_configuration.s3_monitoring_configuration.encryption_key_arn";
const PERSIST_KEY: &str =
    "monitoring_configuration.managed_persistence_monitoring_configuration.encryption_key_arn";
const PROFILE: &str = "ec2_attributes.instance_profile";
const SINGLE: &str = "ec2_attributes.subnet_id";
const SUBNETS: &str = "ec2_attributes.subnet_ids";
const MASTER: &str = "ec2_attributes.emr_managed_master_security_group";
const SLAVE: &str = "ec2_attributes.emr_managed_slave_security_group";
const ACCESS: &str = "ec2_attributes.service_access_security_group";
const ADD_MASTER: &str = "ec2_attributes.additional_master_security_groups";
const ADD_SLAVE: &str = "ec2_attributes.additional_slave_security_groups";

struct Rule {
    source: &'static str,
    outputs: &'static [&'static str],
    label: &'static str,
}
fn rule(kind: &str, p: &str) -> Option<Rule> {
    let (source, outputs, label): (&str, &[&str], &str) = match (kind, p) {
        (STUDIO, "vpc_id") => ("aws_vpc", &["id"], "Studio VPC"),
        (STUDIO, "subnet_ids") => ("aws_subnet", &["id"], "Studio subnets"),
        (STUDIO, "engine_security_group_id" | "workspace_security_group_id") => {
            ("aws_security_group", &["id"], "Studio security group")
        }
        (STUDIO, "service_role" | "user_role") => ("aws_iam_role", &["arn"], "Studio role"),
        (STUDIO, "encryption_key_arn") => ("aws_kms_key", &["arn"], "Studio encryption key"),
        (STUDIO, "default_s3_location") => {
            ("aws_s3_bucket", &["bucket", "id"], "Studio backup bucket")
        }
        (MAPPING, "studio_id") => (STUDIO, &["id"], "EMR Studio"),
        (MAPPING, "session_policy_arn") => ("aws_iam_policy", &["arn"], "Session policy"),
        (VIRTUAL, "container_provider.id") => {
            ("aws_eks_cluster", &["name"], "Registered EKS cluster")
        }
        (TEMPLATE, "kms_key_arn") => ("aws_kms_key", &["arn"], "Template encryption key"),
        (TEMPLATE, "job_template_data.execution_role_arn") => {
            ("aws_iam_role", &["arn"], "Job execution role")
        }
        (TEMPLATE, TEMPLATE_LOG) => ("aws_cloudwatch_log_group", &["name"], "Job log group"),
        (TEMPLATE, TEMPLATE_BUCKET) => ("aws_s3_bucket", &["bucket", "id"], "Job log bucket"),
        (SERVERLESS, NET_SUBNETS) => ("aws_subnet", &["id"], "Application subnets"),
        (SERVERLESS, NET_GROUPS) => ("aws_security_group", &["id"], "Application security groups"),
        (SERVERLESS, CW_LOG) => (
            "aws_cloudwatch_log_group",
            &["name"],
            "Application log group",
        ),
        (SERVERLESS, CW_KEY | PERSIST_KEY | S3_KEY) => {
            ("aws_kms_key", &["arn"], "Application log encryption key")
        }
        (SERVERLESS, S3_URI) => ("aws_s3_bucket", &["bucket", "id"], "Application log bucket"),
        (CLUSTER, SINGLE | SUBNETS) => ("aws_subnet", &["id"], "Cluster subnets"),
        (CLUSTER, MASTER | SLAVE | ACCESS | ADD_MASTER | ADD_SLAVE) => {
            ("aws_security_group", &["id"], "Cluster security groups")
        }
        (CLUSTER, PROFILE) => (
            "aws_iam_instance_profile",
            &["arn", "name"],
            "EC2 instance profile",
        ),
        (CLUSTER, "service_role" | "autoscaling_role") => {
            ("aws_iam_role", &["arn", "name"], "EMR role")
        }
        (CLUSTER, "security_configuration") => (
            "aws_emr_security_configuration",
            &["name"],
            "EMR security configuration",
        ),
        (CLUSTER, "log_encryption_kms_key_id") => (
            "aws_kms_key",
            &["arn", "id", "key_id"],
            "Log encryption key",
        ),
        (CLUSTER, "log_uri") => ("aws_s3_bucket", &["bucket", "id"], "Log bucket"),
        (CLUSTER, "custom_ami_id") => ("aws_ami", &["id"], "Custom machine image"),
        (
            "aws_emr_instance_fleet" | "aws_emr_instance_group" | "aws_emr_managed_scaling_policy",
            "cluster_id",
        ) => (CLUSTER, &["id"], "EMR cluster"),
        _ => return None,
    };
    Some(Rule {
        source,
        outputs,
        label,
    })
}
pub(crate) fn resource(kind: &str) -> bool {
    matches!(kind, CLUSTER | VIRTUAL | TEMPLATE | SERVERLESS)
}
pub(crate) fn indexed(kind: &str, path: &str) -> bool {
    path.contains(".0.") && rule(kind, &path.replace(".0.", ".")).is_some()
}
pub(super) fn label(kind: &str, p: &str) -> Option<&'static str> {
    Some(rule(kind, p)?.label)
}
fn fields(kind: &str, p: &str, s: &str) -> &'static [&'static str] {
    rule(kind, p)
        .filter(|r| r.source == s)
        .map_or(&[], |r| r.outputs)
}
fn values<'a>(path: &str, v: &'a Value) -> Option<BTreeSet<&'a str>> {
    if matches!(path, ADD_MASTER | ADD_SLAVE) {
        let parts: Vec<_> = v.as_str()?.split(',').map(str::trim).collect();
        let set: BTreeSet<_> = parts.iter().copied().collect();
        return (set.len() == parts.len() && !set.contains("")).then_some(set);
    }
    if path.ends_with("log_uri") || path == "default_s3_location" {
        let uri = v.as_str()?;
        if uri.contains(['$', '{', '}', '?', '#']) {
            return None;
        }
        let bucket = uri.strip_prefix("s3://")?.split('/').next()?;
        return (!bucket.is_empty()).then(|| [bucket].into());
    }
    if matches!(path, SUBNETS | "subnet_ids" | NET_SUBNETS | NET_GROUPS) {
        return nested_references::leaf_values(path, v).filter(|_| v.is_array());
    }
    let text = v.as_str()?;
    (!text.is_empty() && !text.contains(['$', '{', '}'])).then(|| [text].into())
}
fn compatible(l: &TerraformEntity, s: &TerraformEntity) -> bool {
    super::eventbridge::bindings(
        l.provider_configuration.as_ref(),
        s.provider_configuration.as_ref(),
        super::scope::scope(l)
            .and_then(|scope| scope.region)
            .as_ref(),
        super::scope::scope(s)
            .and_then(|scope| scope.region)
            .as_ref(),
        s.resource_type.starts_with("aws_iam_"),
    )
}
pub(super) fn bindings(raw: &Input, r: &AttributeReference) -> bool {
    label(&raw.nodes[r.target].resource_type, &r.attribute).is_none()
        || r.sources.iter().all(|&s| {
            let left = &raw.nodes[r.target];
            let right = &raw.nodes[s];
            super::eventbridge::bindings(
                left.provider_configuration.as_ref(),
                right.provider_configuration.as_ref(),
                left.entity.scope.as_ref().and_then(|s| s.region.as_ref()),
                right.entity.scope.as_ref().and_then(|s| s.region.as_ref()),
                right.resource_type.starts_with("aws_iam_"),
            )
        })
}
fn mode_allows(kind: &str, attribute: &str, (value, mask): (&Value, &Value)) -> bool {
    let known = |path| nested_references::value(value, mask, path);
    match (kind, attribute) {
        (STUDIO, "user_role") => known("auth_mode").and_then(Value::as_str) == Some("SSO"),
        (VIRTUAL, "container_provider.id") => {
            known("container_provider.0.type").and_then(Value::as_str) == Some("EKS")
        }
        (SERVERLESS, CW_LOG | CW_KEY) => {
            known("monitoring_configuration.0.cloudwatch_logging_configuration.0.enabled")
                .and_then(Value::as_bool)
                == Some(true)
        }
        (SERVERLESS, PERSIST_KEY) => {
            known(
                "monitoring_configuration.0.managed_persistence_monitoring_configuration.0.enabled",
            )
            .and_then(Value::as_bool)
                == Some(true)
        }
        _ => true,
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
            let k = &nodes[r.target].resource_type;
            let rule = rule(k, &r.attribute)?;
            let ok = mode_allows(k, &r.attribute, e[r.target])
                && nested_references::verify_values(
                    nodes,
                    refs,
                    e,
                    r,
                    fields,
                    |s, _| compatible(&nodes[r.target], &nodes[s]),
                    (&[], values),
                )
                .is_some();
            let parents = if ok && matches!(rule.source, "aws_subnet" | "aws_security_group") {
                r.sources
                    .iter()
                    .map(|&s| lambda_core::parent(nodes, refs, e, s))
                    .collect::<Option<BTreeSet<_>>>()
                    .filter(|s| s.len() == 1)
                    .into_iter()
                    .flatten()
                    .collect()
            } else {
                Vec::new()
            };
            Some((ok, parents))
        })
        .collect();
    for (r, result) in refs.iter_mut().zip(results) {
        if let Some((ok, parents)) = result {
            r.service_reference_verified = ok;
            if ok {
                r.complete = true
            }
            r.endpoint_network_context = parents;
        }
    }
    for (i, _) in nodes
        .iter()
        .enumerate()
        .filter(|(_, n)| n.resource_type == CLUSTER)
    {
        let paths = [
            SINGLE, SUBNETS, MASTER, SLAVE, ACCESS, ADD_MASTER, ADD_SLAVE,
        ];
        let configured: Vec<_> = refs
            .iter()
            .filter(|r| r.target == i && paths.contains(&r.attribute.as_str()))
            .collect();
        let valid = configured
            .iter()
            .filter(|r| matches!(r.attribute.as_str(), SINGLE | SUBNETS))
            .count()
            == 1
            && configured.len() >= 2
            && configured
                .iter()
                .all(|r| r.service_reference_verified && r.endpoint_network_context.len() == 1);
        if !valid {
            for r in refs
                .iter_mut()
                .filter(|r| r.target == i && paths.contains(&r.attribute.as_str()))
            {
                r.endpoint_network_context.clear()
            }
        }
    }
}
pub(super) fn present(raw: &Input, graph: &mut Graph) {
    lambda_core::present_network(raw, graph, SERVERLESS, (NET_SUBNETS, NET_GROUPS), bindings);
    for (t, _) in raw
        .nodes
        .iter()
        .enumerate()
        .filter(|(_, n)| n.resource_type == STUDIO)
    {
        let proof = |p| {
            lambda_core::unique(&raw.attributes, t, p)
                .filter(|r| r.service_reference_verified && bindings(raw, r))
        };
        let Some(vpc) = proof("vpc_id") else { continue };
        let [parent] = vpc.sources.as_slice() else {
            continue;
        };
        let Some(network) = [
            "subnet_ids",
            "engine_security_group_id",
            "workspace_security_group_id",
        ]
        .into_iter()
        .map(proof)
        .collect::<Option<Vec<_>>>() else {
            continue;
        };
        if !network.iter().all(|r| {
            r.endpoint_network_context == [*parent]
                && r.sources.iter().all(|&s| {
                    lambda_core::unique(&raw.attributes, s, "vpc_id")
                        .is_some_and(|p| super::apprunner::bindings(raw, p))
                })
        }) {
            continue;
        }
        let mut evidence = vec![relationships::reference(graph, *parent, t)];
        for &member in network.iter().flat_map(|r| &r.sources) {
            evidence.extend([
                relationships::reference(graph, *parent, member),
                relationships::reference(graph, member, t),
            ]);
        }
        relationships::record(graph, *parent, t, EdgeKind::Containment, evidence);
        graph.edges.push(Edge {
            kind: EdgeKind::Containment,
            ..Edge::from((*parent, t))
        });
    }
    for (t, _) in raw
        .nodes
        .iter()
        .enumerate()
        .filter(|(_, n)| n.resource_type == CLUSTER)
    {
        let network: Vec<_> = raw
            .attributes
            .iter()
            .filter(|r| r.target == t && !r.endpoint_network_context.is_empty())
            .collect();
        let Some(parent) = network
            .first()
            .and_then(|r| r.endpoint_network_context.first())
            .copied()
        else {
            continue;
        };
        if !network
            .iter()
            .any(|r| matches!(r.attribute.as_str(), SINGLE | SUBNETS))
            || !network.iter().any(|r| {
                matches!(
                    r.attribute.as_str(),
                    MASTER | SLAVE | ACCESS | ADD_MASTER | ADD_SLAVE
                )
            })
        {
            continue;
        }
        if !network.iter().all(|r| {
            r.endpoint_network_context == [parent]
                && bindings(raw, r)
                && r.sources.iter().all(|&s| {
                    lambda_core::unique(&raw.attributes, s, "vpc_id")
                        .is_some_and(|p| super::apprunner::bindings(raw, p))
                })
        }) {
            continue;
        }
        let proof = network
            .iter()
            .flat_map(|r| r.sources.iter())
            .flat_map(|&s| {
                [
                    relationships::reference(graph, parent, s),
                    relationships::reference(graph, s, t),
                ]
            })
            .collect();
        relationships::record(graph, parent, t, EdgeKind::Containment, proof);
        graph.edges.push(Edge {
            kind: EdgeKind::Containment,
            ..Edge::from((parent, t))
        });
    }
}
#[cfg(test)]
#[path = "../../../tests/unit/semantic/emr.rs"]
mod tests;
