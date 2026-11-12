use crate::model::{AttributeReference, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;
#[cfg(test)]
#[path = "../../../tests/unit/semantic/transfer_as2.rs"]
mod tests;
const LATTICE: &str = "egress_config.vpc_lattice.resource_configuration_arn";
pub(crate) fn resource(kind: &str) -> bool {
    matches!(
        kind,
        "aws_transfer_profile" | "aws_transfer_agreement" | "aws_transfer_connector"
    )
}
pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    match (kind, path) {
        ("aws_transfer_profile", "certificate_ids") => Some("AS2 certificates"),
        ("aws_transfer_agreement", "server_id") => Some("Agreement server"),
        ("aws_transfer_agreement" | "aws_transfer_connector", "access_role") => {
            Some("Transfer access role")
        }
        ("aws_transfer_connector", "logging_role") => Some("Connector logging role"),
        ("aws_transfer_agreement", "local_profile_id")
        | ("aws_transfer_connector", "as2_config.local_profile_id") => Some("Local AS2 profile"),
        ("aws_transfer_agreement", "partner_profile_id")
        | ("aws_transfer_connector", "as2_config.partner_profile_id") => {
            Some("Partner AS2 profile")
        }
        ("aws_transfer_connector", "sftp_config.user_secret_id") => Some("SFTP user secret"),
        ("aws_transfer_connector", LATTICE) => Some("Connector Lattice resource configuration"),
        _ => None,
    }
}
pub(crate) fn indexed(kind: &str, path: &str) -> bool {
    kind == "aws_transfer_connector"
        && matches!(
            path,
            "as2_config.0.local_profile_id"
                | "as2_config.0.partner_profile_id"
                | "sftp_config.0.user_secret_id"
                | "egress_config.0.vpc_lattice.0.resource_configuration_arn"
        )
}
fn fields(kind: &str, path: &str, source: &str) -> &'static [&'static str] {
    if label(kind, path).is_none() {
        return &[];
    }
    match (path, source) {
        ("certificate_ids", "aws_transfer_certificate") => &["certificate_id"],
        ("server_id", "aws_transfer_server") => &["id"],
        ("access_role" | "logging_role", "aws_iam_role") => &["arn"],
        (
            "local_profile_id"
            | "partner_profile_id"
            | "as2_config.local_profile_id"
            | "as2_config.partner_profile_id",
            "aws_transfer_profile",
        ) => &["profile_id"],
        ("sftp_config.user_secret_id", "aws_secretsmanager_secret") => &["arn", "name"],
        (LATTICE, "aws_vpclattice_resource_configuration") => &["arn"],
        _ => &[],
    }
}
fn singleton(v: &Value) -> bool {
    v.as_array().is_some_and(|a| a.len() == 1)
}
fn value<'a>(e: &[(&'a Value, &Value)], t: usize, path: &str) -> Option<&'a Value> {
    super::nested_references::value(e[t].0, e[t].1, path)
}
fn known<'a>(e: &[(&'a Value, &Value)], t: usize, path: &str) -> Option<&'a str> {
    value(e, t, path)?.as_str()
}
fn mode(e: &[(&Value, &Value)], t: usize, selected: &str, other: &str) -> bool {
    let mask = e[t].1;
    matches!(mask, Value::Null | Value::Bool(false) | Value::Object(_))
        && (matches!(&mask[selected], Value::Null | Value::Bool(false))
            || mask[selected].as_array().is_some_and(|a| {
                a.len() == 1 && matches!(a[0], Value::Null | Value::Bool(false) | Value::Object(_))
            }))
        && singleton(&e[t].0[selected])
        && e[t].0[selected][0].is_object()
        && value(e, t, other)
            .is_some_and(|v| v.is_null() || v.as_array().is_some_and(Vec::is_empty))
}
fn verify(
    nodes: &[TerraformEntity],
    refs: &[AttributeReference],
    e: &[(&Value, &Value)],
    r: &AttributeReference,
) -> bool {
    let t = r.target;
    let p = r.attribute.as_str();
    let v = e[t].0;
    if p == "certificate_ids" {
        if !v[p].is_array() || !matches!(known(e, t, "profile_type"), Some("LOCAL" | "PARTNER")) {
            return false;
        }
    } else if p.starts_with("as2_config.") {
        if !mode(e, t, "as2_config", "sftp_config") {
            return false;
        }
    } else if p == "sftp_config.user_secret_id" || p == LATTICE {
        if !mode(e, t, "sftp_config", "as2_config") {
            return false;
        }
        if p == LATTICE
            && (!singleton(&v["egress_config"])
                || !singleton(&v["egress_config"][0]["vpc_lattice"])
                || !value(e, t, "url").is_some_and(Value::is_null))
        {
            return false;
        }
    }
    super::nested_references::verify(nodes, refs, e, r, fields, |s, path| {
        let shape = value(e, t, path).is_some_and(|v| {
            if p == "certificate_ids" {
                v.is_array()
            } else {
                v.is_string()
            }
        });
        shape
            && match p {
                "certificate_ids" => matches!(known(e, s, "usage"), Some("SIGNING" | "ENCRYPTION")),
                "local_profile_id" | "as2_config.local_profile_id" => {
                    known(e, s, "profile_type") == Some("LOCAL")
                }
                "partner_profile_id" | "as2_config.partner_profile_id" => {
                    known(e, s, "profile_type") == Some("PARTNER")
                }
                _ => true,
            }
    })
    .is_some()
}
pub(super) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    e: &[(&Value, &Value)],
) {
    let results: Vec<_> = refs
        .iter()
        .map(|r| {
            label(&nodes[r.target].resource_type, &r.attribute).map(|_| verify(nodes, refs, e, r))
        })
        .collect();
    for (r, result) in refs.iter_mut().zip(results) {
        if let Some(valid) = result {
            r.service_reference_verified = valid;
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
    r.sources.iter().all(|&s| {
        let l = &raw.nodes[r.target];
        let right = &raw.nodes[s];
        super::eventbridge::bindings(
            l.provider_configuration.as_ref(),
            right.provider_configuration.as_ref(),
            l.entity.scope.as_ref().and_then(|s| s.region.as_ref()),
            right.entity.scope.as_ref().and_then(|s| s.region.as_ref()),
            right.resource_type == "aws_iam_role",
        )
    })
}
