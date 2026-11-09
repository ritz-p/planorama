use crate::model::{AttributeReference, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;
#[cfg(test)]
#[path = "../../../tests/unit/semantic/route53_dnssec.rs"]
mod tests;

pub(super) fn resource(kind: &str) -> bool {
    matches!(
        kind,
        "aws_route53_hosted_zone_dnssec"
            | "aws_route53_key_signing_key"
            | "aws_route53domains_delegation_signer_record"
    )
}
pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    match (kind, path) {
        ("aws_route53_hosted_zone_dnssec" | "aws_route53_key_signing_key", "hosted_zone_id") => {
            Some("DNSSEC hosted zone")
        }
        ("aws_route53_key_signing_key", "key_management_service_arn") => Some("DNSSEC KMS key"),
        ("aws_route53domains_delegation_signer_record", "domain_name") => {
            Some("Registered domain owner")
        }
        _ => None,
    }
}
fn fields(_: &str, path: &str, source: &str) -> &'static [&'static str] {
    match (path, source) {
        ("hosted_zone_id", "aws_route53_zone") => &["id", "zone_id"],
        ("key_management_service_arn", kind) if super::kms::key_outputs(kind).contains(&"arn") => {
            &["arn"]
        }
        ("domain_name", "aws_route53domains_registered_domain" | "aws_route53domains_domain") => {
            &["domain_name"]
        }
        _ => &[],
    }
}
pub(crate) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    e: &[(&Value, &Value)],
) {
    let results: Vec<_> = refs
        .iter()
        .map(|r| {
            label(&nodes[r.target].resource_type, &r.attribute)?;
            Some(
                super::nested_references::verify(nodes, refs, e, r, fields, |_, path| {
                    let (v, m) = e[r.target];
                    super::nested_references::value(v, m, path).is_some_and(Value::is_string)
                })
                .is_some(),
            )
        })
        .collect();
    for (r, valid) in refs.iter_mut().zip(results) {
        if let Some(valid) = valid {
            r.service_reference_verified = valid;
        }
    }
}
pub(super) fn bindings(raw: &Input, r: &AttributeReference) -> bool {
    if label(&raw.nodes[r.target].resource_type, &r.attribute).is_none() {
        return true;
    }
    r.sources.iter().all(|&s| {
        let a = &raw.nodes[r.target];
        let b = &raw.nodes[s];
        super::iam_policies::same_account(
            a.provider_configuration.as_ref(),
            b.provider_configuration.as_ref(),
        ) && (r.attribute != "key_management_service_arn"
            || b.entity
                .scope
                .as_ref()
                .and_then(|s| s.region.as_ref())
                .is_some_and(|r| r.0 == "us-east-1"))
    })
}
