use crate::model::{AttributeReference, Graph, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;
use std::collections::BTreeSet;
#[cfg(test)]
#[path = "../../../tests/unit/semantic/classic_elb.rs"]
mod tests;

pub(crate) fn resource(kind: &str) -> bool {
    kind == "aws_elb" || owner_path(kind).is_some()
}
pub(super) fn owner_path(kind: &str) -> Option<&'static str> {
    match kind {
        "aws_elb_attachment" => Some("elb"),
        "aws_app_cookie_stickiness_policy"
        | "aws_lb_cookie_stickiness_policy"
        | "aws_lb_ssl_negotiation_policy"
        | "aws_proxy_protocol_policy" => Some("load_balancer"),
        "aws_load_balancer_backend_server_policy"
        | "aws_load_balancer_listener_policy"
        | "aws_load_balancer_policy" => Some("load_balancer_name"),
        _ => None,
    }
}
pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    if owner_path(kind) == Some(path) {
        return Some("Classic load balancer");
    }
    match (kind, path) {
        ("aws_elb", "subnets") => Some("Classic ELB subnets"),
        ("aws_elb", "security_groups") => Some("Classic ELB security groups"),
        ("aws_elb", "instances") | ("aws_elb_attachment", "instance") => {
            Some("Classic ELB instances")
        }
        ("aws_elb", "listener.ssl_certificate_id") => Some("Classic listener certificate"),
        ("aws_elb", "access_logs.bucket") => Some("Classic ELB log bucket"),
        (
            "aws_load_balancer_backend_server_policy" | "aws_load_balancer_listener_policy",
            "policy_names",
        ) => Some("Qualified Classic ELB policies"),
        _ => None,
    }
}
fn fields(kind: &str, path: &str, source: &str) -> &'static [&'static str] {
    if owner_path(kind) == Some(path) && source == "aws_elb" {
        return &["name", "id"];
    }
    match (path, source) {
        ("subnets", "aws_subnet")
        | ("security_groups", "aws_security_group")
        | ("instances" | "instance", "aws_instance") => &["id"],
        ("listener.ssl_certificate_id", "aws_acm_certificate" | "aws_iam_server_certificate") => {
            &["arn"]
        }
        ("access_logs.bucket", "aws_s3_bucket") => &["bucket", "id"],
        ("policy_names", "aws_load_balancer_policy") => &["policy_name"],
        (
            "policy_names",
            "aws_app_cookie_stickiness_policy"
            | "aws_lb_cookie_stickiness_policy"
            | "aws_lb_ssl_negotiation_policy",
        ) => &["name"],
        _ => &[],
    }
}
pub(crate) fn indexed(kind: &str, path: &str) -> bool {
    let normal = path
        .split('.')
        .filter(|s| s.parse::<usize>().is_err())
        .collect::<Vec<_>>()
        .join(".");
    normal != path && label(kind, &normal).is_some()
}
fn value<'a>(e: &[(&'a Value, &Value)], i: usize, path: &str) -> Option<&'a Value> {
    super::nested_references::value(e[i].0, e[i].1, path)
}
fn base(
    nodes: &[TerraformEntity],
    refs: &[AttributeReference],
    e: &[(&Value, &Value)],
    r: &AttributeReference,
) -> bool {
    super::nested_references::verify(nodes, refs, e, r, fields, |_, path| {
        let collection = matches!(
            r.attribute.as_str(),
            "subnets" | "security_groups" | "instances" | "policy_names"
        );
        value(e, r.target, path).is_some_and(|v| {
            if collection {
                v.is_array()
            } else {
                v.is_string()
            }
        })
    })
    .is_some()
}
fn owner(
    nodes: &[TerraformEntity],
    refs: &[AttributeReference],
    e: &[(&Value, &Value)],
    t: usize,
) -> Option<usize> {
    let path = owner_path(&nodes[t].resource_type)?;
    let r = refs.iter().find(|r| r.target == t && r.attribute == path)?;
    let [s] = r.sources.as_slice() else {
        return None;
    };
    (base(nodes, refs, e, r) && ports(e, t, *s, &nodes[t].resource_type)).then_some(*s)
}
fn port(value: &Value) -> Option<u64> {
    value
        .as_u64()
        .or_else(|| value.as_str()?.parse().ok())
        .filter(|p| (1..=65535).contains(p))
}
fn ports(e: &[(&Value, &Value)], t: usize, s: usize, kind: &str) -> bool {
    let (input, output, protocols): (&str, &str, &[&str]) = match kind {
        "aws_app_cookie_stickiness_policy" | "aws_lb_cookie_stickiness_policy" => {
            ("lb_port", "lb_port", &["HTTP", "HTTPS"])
        }
        "aws_lb_ssl_negotiation_policy" => ("lb_port", "lb_port", &["HTTPS", "SSL"]),
        "aws_load_balancer_listener_policy" => ("load_balancer_port", "lb_port", &[]),
        "aws_load_balancer_backend_server_policy" => ("instance_port", "instance_port", &[]),
        "aws_proxy_protocol_policy" => ("instance_ports", "instance_port", &["TCP", "SSL"]),
        _ => return true,
    };
    let Some(v) = value(e, t, input) else {
        return false;
    };
    let requested: Option<BTreeSet<_>> = if let Some(a) = v.as_array() {
        a.iter().map(port).collect()
    } else {
        port(v).map(|p| [p].into())
    };
    let Some(requested) = requested.filter(|a| !a.is_empty()) else {
        return false;
    };
    let Some(listeners) = e[s].0["listener"].as_array().filter(|a| !a.is_empty()) else {
        return false;
    };
    let protocol = if output == "instance_port" {
        "instance_protocol"
    } else {
        "lb_protocol"
    };
    let available: Option<Vec<_>> = listeners
        .iter()
        .enumerate()
        .map(|(i, _)| {
            let p = port(value(e, s, &format!("listener.{i}.{output}"))?)?;
            let allowed = protocols.is_empty()
                || value(e, s, &format!("listener.{i}.{protocol}"))?
                    .as_str()
                    .is_some_and(|p| protocols.iter().any(|q| p.eq_ignore_ascii_case(q)));
            Some((p, allowed))
        })
        .collect();
    available.is_some_and(|a| {
        requested.iter().all(|p| {
            let matches: Vec<_> = a.iter().filter(|(q, _)| q == p).collect();
            !matches.is_empty()
                && (output == "instance_port" || matches.len() == 1)
                && matches.iter().all(|(_, allowed)| *allowed)
        })
    })
}
pub(crate) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    e: &[(&Value, &Value)],
) {
    let results: Vec<_> = refs
        .iter()
        .map(|r| {
            let kind = &nodes[r.target].resource_type;
            label(kind, &r.attribute)?;
            let mut context = Vec::new();
            let mut valid = base(nodes, refs, e, r);
            if valid {
                match r.attribute.as_str() {
                    "listener.ssl_certificate_id" => {
                        valid = e[r.target].0["listener"].as_array().is_some_and(|a| {
                            !a.is_empty()
                                && a.iter().enumerate().all(|(i, row)| {
                                    if row["ssl_certificate_id"].is_null() {
                                        return true;
                                    }
                                    value(e, r.target, &format!("listener.{i}.lb_protocol"))
                                        .and_then(Value::as_str)
                                        .is_some_and(|p| {
                                            p.eq_ignore_ascii_case("HTTPS")
                                                || p.eq_ignore_ascii_case("SSL")
                                        })
                                })
                        });
                    }
                    "access_logs.bucket" => {
                        valid = e[r.target].0["access_logs"]
                            .as_array()
                            .is_some_and(|a| a.len() == 1)
                            && value(e, r.target, "access_logs.0.enabled")
                                == Some(&Value::Bool(true))
                    }
                    "subnets" | "security_groups" => {
                        context = r
                            .sources
                            .iter()
                            .map(|&s| super::lambda_core::parent(nodes, refs, e, s))
                            .collect::<Option<BTreeSet<_>>>()
                            .filter(|p| p.len() == 1)
                            .map(|p| p.into_iter().collect())
                            .unwrap_or_default();
                    }
                    "policy_names" => {
                        if let Some(parent) = owner(nodes, refs, e, r.target) {
                            valid = r
                                .sources
                                .iter()
                                .all(|&s| owner(nodes, refs, e, s) == Some(parent));
                            context.push(parent);
                        } else {
                            valid = false;
                        }
                    }
                    p if owner_path(kind) == Some(p) => {
                        valid = owner(nodes, refs, e, r.target).is_some()
                    }
                    _ => {}
                }
            }
            Some((valid, context))
        })
        .collect();
    for (r, result) in refs.iter_mut().zip(results) {
        if let Some((valid, context)) = result {
            r.service_reference_verified = valid;
            r.endpoint_network_context = context;
            if valid {
                r.complete = true;
            }
        }
    }
}
pub(super) fn bindings(raw: &Input, r: &AttributeReference) -> bool {
    if label(&raw.nodes[r.target].resource_type, &r.attribute).is_none() {
        return true;
    }
    r.sources
        .iter()
        .chain(&r.endpoint_network_context)
        .all(|&s| {
            let a = &raw.nodes[r.target];
            let b = &raw.nodes[s];
            let ar = a.entity.scope.as_ref().and_then(|s| s.region.as_ref());
            let br = b.entity.scope.as_ref().and_then(|s| s.region.as_ref());
            super::iam_policies::same_account(
                a.provider_configuration.as_ref(),
                b.provider_configuration.as_ref(),
            ) && ar.is_some()
                && (b.resource_type == "aws_iam_server_certificate" || ar == br)
        })
}
pub(super) fn association(raw: &Input, t: usize) -> Option<(usize, usize)> {
    let endpoint = |path| {
        let r = raw
            .attributes
            .iter()
            .find(|r| r.target == t && r.attribute == path)?;
        let [s] = r.sources.as_slice() else {
            return None;
        };
        super::service_references::verified(raw, r).then_some(*s)
    };
    endpoint("elb").zip(endpoint("instance"))
}
pub(super) fn present(raw: &Input, graph: &mut Graph) {
    super::lambda_core::present_network(
        raw,
        graph,
        "aws_elb",
        ("subnets", "security_groups"),
        bindings,
    );
}
