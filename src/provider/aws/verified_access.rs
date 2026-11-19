use super::configured_links::{self, Evidence, Rule};
use crate::model::{AttributeReference, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;

fn rule(kind: &str, path: &str) -> Option<Rule> {
    Some(match (kind, path) {
        (
            "aws_verifiedaccess_group"
            | "aws_verifiedaccess_instance_logging_configuration"
            | "aws_verifiedaccess_instance_trust_provider_attachment",
            "verifiedaccess_instance_id",
        ) => Rule::new(
            "Verified Access instance",
            &[("aws_verifiedaccess_instance", &["id"])],
            false,
            &[],
        ),
        ("aws_verifiedaccess_endpoint", "verified_access_group_id") => Rule::new(
            "Verified Access group",
            &[(
                "aws_verifiedaccess_group",
                &["verifiedaccess_group_id", "id"],
            )],
            false,
            &[],
        ),
        ("aws_verifiedaccess_endpoint", "domain_certificate_arn") => Rule::new(
            "Application certificate",
            &[("aws_acm_certificate", &["arn"])],
            false,
            &[],
        ),
        ("aws_verifiedaccess_endpoint", "security_group_ids") => Rule::new(
            "Endpoint security groups",
            &[("aws_security_group", &["id"])],
            true,
            &[],
        ),
        ("aws_verifiedaccess_endpoint", "load_balancer_options.load_balancer_arn") => Rule::new(
            "Application load balancer",
            &[("aws_lb", &["arn"])],
            false,
            &[],
        ),
        ("aws_verifiedaccess_endpoint", "network_interface_options.network_interface_id") => {
            Rule::new(
                "Application network interface",
                &[("aws_network_interface", &["id"])],
                false,
                &[],
            )
        }
        ("aws_verifiedaccess_endpoint", "rds_options.rds_db_proxy_arn") => Rule::new(
            "Application database",
            &[("aws_db_proxy", &["arn"])],
            false,
            &[],
        ),
        ("aws_verifiedaccess_endpoint", "rds_options.rds_db_instance_arn") => Rule::new(
            "Application database",
            &[("aws_db_instance", &["arn"])],
            false,
            &[],
        ),
        ("aws_verifiedaccess_endpoint", "rds_options.rds_db_cluster_arn") => Rule::new(
            "Application database",
            &[("aws_rds_cluster", &["arn"])],
            false,
            &[],
        ),
        ("aws_verifiedaccess_endpoint", "rds_options.subnet_ids") => {
            Rule::new("Endpoint subnets", &[("aws_subnet", &["id"])], true, &[])
        }
        ("aws_verifiedaccess_endpoint", "load_balancer_options.subnet_ids") => {
            Rule::new("Endpoint subnets", &[("aws_subnet", &["id"])], true, &[])
        }
        ("aws_verifiedaccess_endpoint", "cidr_options.subnet_ids") => {
            Rule::new("Endpoint subnets", &[("aws_subnet", &["id"])], true, &[])
        }
        (
            "aws_verifiedaccess_endpoint" | "aws_verifiedaccess_trust_provider",
            "sse_specification.kms_key_arn",
        ) => Rule::new(
            "Policy encryption key",
            &[("aws_kms_key", &["arn"])],
            false,
            &[],
        ),
        ("aws_verifiedaccess_group", "sse_configuration.kms_key_arn") => Rule::new(
            "Policy encryption key",
            &[("aws_kms_key", &["arn"])],
            false,
            &[],
        ),
        ("aws_verifiedaccess_instance_logging_configuration", "access_logs.s3.bucket_name") => {
            Rule::new(
                "Access log bucket",
                &[("aws_s3_bucket", &["bucket", "id"])],
                false,
                &[],
            )
            .with_explicit_destination()
        }
        (
            "aws_verifiedaccess_instance_logging_configuration",
            "access_logs.cloudwatch_logs.log_group",
        ) => Rule::new(
            "Access log group",
            &[("aws_cloudwatch_log_group", &["name"])],
            false,
            &[],
        ),
        (
            "aws_verifiedaccess_instance_logging_configuration",
            "access_logs.kinesis_data_firehose.delivery_stream",
        ) => Rule::new(
            "Access log delivery stream",
            &[("aws_kinesis_firehose_delivery_stream", &["name"])],
            false,
            &[],
        ),
        (
            "aws_verifiedaccess_instance_trust_provider_attachment",
            "verifiedaccess_trust_provider_id",
        ) => Rule::new(
            "Trust provider",
            &[("aws_verifiedaccess_trust_provider", &["id"])],
            false,
            &[],
        ),
        _ => return None,
    })
}
pub(crate) fn resource(kind: &str) -> bool {
    matches!(
        kind,
        "aws_verifiedaccess_group"
            | "aws_verifiedaccess_instance_logging_configuration"
            | "aws_verifiedaccess_instance_trust_provider_attachment"
            | "aws_verifiedaccess_endpoint"
            | "aws_verifiedaccess_trust_provider"
    )
}
pub(crate) fn indexed(kind: &str, path: &str) -> bool {
    resource(kind) && configured_links::indexed(kind, path, rule)
}
pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    rule(kind, path).map(|r| r.label)
}
pub(super) fn bindings(raw: &Input, r: &AttributeReference) -> bool {
    configured_links::bindings(raw, r, rule)
        && (raw.nodes[r.target].resource_type
            != "aws_verifiedaccess_instance_logging_configuration"
            || r.attribute != "access_logs.s3.bucket_name"
            || {
                let Some(&source) = r.sources.first() else {
                    return false;
                };
                let region = |i: usize| {
                    raw.nodes[i]
                        .entity
                        .scope
                        .as_ref()
                        .and_then(|s| s.region.as_ref())
                };
                region(source).is_some()
                    && region(source) == region(r.target)
                    && r.service_reference_owner_account.as_ref().map_or_else(
                        || configured_links::destination_in_same_account(raw, r),
                        |owner| {
                            raw.nodes[r.target]
                                .provider_configuration
                                .as_ref()
                                .is_some_and(|p| p.account.is_some())
                                && raw.nodes[source]
                                    .provider_configuration
                                    .as_ref()
                                    .is_some_and(|p| p.account.as_ref() == Some(owner))
                        },
                    )
            })
}
pub(super) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    values: &[(&Value, &Value)],
) {
    configured_links::annotate(nodes, refs, values, rule, qualifies);
    let evidence = Evidence {
        nodes,
        refs,
        values,
    };
    let contexts: Vec<_> = refs
        .iter()
        .enumerate()
        .filter(|(_, r)| {
            r.service_reference_verified
                && r.attribute == "network_interface_options.network_interface_id"
                && nodes[r.target].resource_type == "aws_verifiedaccess_endpoint"
        })
        .filter_map(|(i, r)| {
            endpoint_network(&evidence, r.target, "network_interface_options").map(|vpc| (i, vpc))
        })
        .collect();
    for (i, vpc) in contexts {
        refs[i].endpoint_network_context = vec![vpc];
    }
    for r in refs.iter_mut().filter(|r| {
        nodes[r.target].resource_type == "aws_verifiedaccess_instance_logging_configuration"
            && r.attribute == "access_logs.s3.bucket_name"
            && r.service_reference_verified
    }) {
        let (v, m) = values[r.target];
        r.service_reference_owner_account =
            super::nested_references::value(v, m, "access_logs.0.s3.0.bucket_owner")
                .and_then(Value::as_str)
                .and_then(crate::model::AwsAccountScope::new);
    }
}
fn qualifies(e: &Evidence<'_>, r: &AttributeReference, _source: usize, path: &str) -> bool {
    let node = r.target;
    let field = r.attribute.as_str();
    if field.ends_with(".kms_key_arn") {
        return path.rsplit_once('.').is_some_and(|(p, _)| {
            e.value(node, &format!("{p}.customer_managed_key_enabled"))
                .and_then(Value::as_bool)
                == Some(true)
        });
    }
    if field.starts_with("access_logs.") {
        return path.rsplit_once('.').is_some_and(|(p, _)| {
            e.value(node, &format!("{p}.enabled"))
                .and_then(Value::as_bool)
                == Some(true)
        }) && (field != "access_logs.s3.bucket_name"
            || (e.absent(node, "access_logs.0.s3.0.bucket_owner")
                || e.text(node, "access_logs.0.s3.0.bucket_owner")
                    .and_then(crate::model::AwsAccountScope::new)
                    .is_some()));
    }
    if e.nodes[node].resource_type != "aws_verifiedaccess_endpoint"
        || field == "verified_access_group_id"
    {
        return true;
    }
    let Some(block) = selected_options(e, node) else {
        return false;
    };
    if field == "domain_certificate_arn" {
        return matches!(block, "load_balancer_options" | "network_interface_options");
    }
    if field != "security_group_ids" && !field.starts_with(&format!("{block}.")) {
        return false;
    }
    if field.starts_with("rds_options.rds_db_") {
        return [
            "rds_db_instance_arn",
            "rds_db_proxy_arn",
            "rds_db_cluster_arn",
        ]
        .iter()
        .all(|other| field.ends_with(other) || e.absent(node, &format!("rds_options.0.{other}")));
    }
    if field == "security_group_ids"
        || field.ends_with(".subnet_ids")
        || field.ends_with(".network_interface_id")
    {
        return endpoint_network(e, node, block).is_some();
    }
    true
}
#[cfg(test)]
#[path = "../../../tests/unit/semantic/verified_access.rs"]
mod tests;

fn selected_options(e: &Evidence<'_>, node: usize) -> Option<&'static str> {
    if e.text(node, "attachment_type") != Some("vpc") {
        return None;
    }
    let selected = match e.text(node, "endpoint_type")? {
        "cidr" => "cidr_options",
        "rds" => "rds_options",
        "load-balancer" => "load_balancer_options",
        "network-interface" => "network_interface_options",
        _ => return None,
    };
    let (value, mask) = e.values[node];
    (super::nested_references::singleton(value, mask, selected)
        && [
            "cidr_options",
            "rds_options",
            "load_balancer_options",
            "network_interface_options",
        ]
        .iter()
        .all(|&b| b == selected || e.absent(node, b)))
    .then_some(selected)
}

fn endpoint_network(e: &Evidence<'_>, node: usize, block: &str) -> Option<usize> {
    let parent = if block == "network_interface_options" {
        let eni = e.configured_endpoint(
            node,
            "network_interface_options.0.network_interface_id",
            "aws_network_interface",
            &["id"],
        )?;
        let subnet = e.configured_endpoint(eni, "subnet_id", "aws_subnet", &["id"])?;
        super::lambda_core::parent(e.nodes, e.refs, e.values, subnet)?
    } else {
        configured_links::network_context(
            e,
            node,
            &[(&format!("{block}.subnet_ids"), "aws_subnet")],
        )?
    };
    (e.absent(node, "security_group_ids")
        || configured_links::network_context(
            e,
            node,
            &[("security_group_ids", "aws_security_group")],
        ) == Some(parent))
    .then_some(parent)
}

pub(super) fn present(raw: &Input, graph: &mut crate::model::Graph) {
    use crate::model::{Edge, EdgeKind};
    use crate::semantic::relationships;
    for r in raw.attributes.iter().filter(|r| {
        raw.nodes[r.target].resource_type == "aws_verifiedaccess_endpoint"
            && (r.attribute.ends_with(".subnet_ids")
                || r.attribute == "network_interface_options.network_interface_id")
            && super::service_references::verified(raw, r)
    }) {
        let [vpc] = r.endpoint_network_context.as_slice() else {
            continue;
        };
        let mut members = r.sources.clone();
        if let Some(groups) =
            super::lambda_core::unique(&raw.attributes, r.target, "security_group_ids")
        {
            if !super::service_references::verified(raw, groups)
                || groups.endpoint_network_context != [*vpc]
            {
                continue;
            }
            members.extend_from_slice(&groups.sources);
        }
        let mut evidence = Vec::new();
        let mut valid = true;
        for member in members {
            evidence.push(relationships::reference(graph, member, r.target));
            let subnet = if raw.nodes[member].resource_type == "aws_network_interface" {
                let Some(sub) = super::lambda_core::unique(&raw.attributes, member, "subnet_id")
                    .filter(|r| super::service_references::verified(raw, r))
                    .and_then(|r| r.sources.first())
                else {
                    valid = false;
                    break;
                };
                evidence.push(relationships::reference(graph, *sub, member));
                *sub
            } else {
                member
            };
            if !super::lambda_core::unique(&raw.attributes, subnet, "vpc_id")
                .is_some_and(|r| super::apprunner::bindings(raw, r))
            {
                valid = false;
                break;
            }
            evidence.push(relationships::reference(graph, *vpc, subnet));
        }
        if valid {
            relationships::record(graph, *vpc, r.target, EdgeKind::Containment, evidence);
            graph.edges.push(Edge {
                kind: EdgeKind::Containment,
                ..Edge::from((*vpc, r.target))
            });
        }
    }
}
