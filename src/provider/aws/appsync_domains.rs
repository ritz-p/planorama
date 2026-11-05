use crate::model::{AttributeReference, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/appsync_domains.rs"]
mod tests;

const DOMAIN: &str = "aws_appsync_domain_name";
const ASSOCIATION: &str = "aws_appsync_domain_name_api_association";

pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    match (kind, path) {
        (DOMAIN, "certificate_arn") => Some("Custom domain certificate"),
        (ASSOCIATION, "api_id") => Some("Associated GraphQL API"),
        (ASSOCIATION, "domain_name") => Some("Associated custom domain"),
        _ => None,
    }
}

pub(super) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    evidence: &[(&Value, &Value)],
) {
    let mut counts = std::collections::BTreeMap::new();
    for r in refs.iter() {
        *counts.entry((r.target, r.attribute.clone())).or_insert(0) += 1;
    }
    for r in refs {
        if label(&nodes[r.target].resource_type, &r.attribute).is_none() {
            continue;
        }
        r.service_reference_verified = false;
        if counts[&(r.target, r.attribute.clone())] != 1 {
            continue;
        }
        let (kind, output) = match r.attribute.as_str() {
            "certificate_arn" => ("aws_acm_certificate", "arn"),
            "api_id" => ("aws_appsync_graphql_api", "id"),
            "domain_name" => (DOMAIN, "domain_name"),
            _ => continue,
        };
        let source = super::route53::endpoint(nodes, r, kind, &[output]).or_else(|| {
            (r.attribute == "certificate_arn")
                .then(|| super::route53::endpoint(nodes, r, "aws_iam_server_certificate", &["arn"]))
                .flatten()
        });
        let Some(s) = source else { continue };
        let own_region = super::scope::scope(&nodes[r.target]).and_then(|s| s.region);
        let source_region = super::scope::scope(&nodes[s]).and_then(|s| s.region);
        if own_region.is_none() || !super::route53::configured(&nodes[r.target], &nodes[s]) {
            continue;
        }
        if r.attribute == "certificate_arn" {
            if nodes[s].resource_type == "aws_acm_certificate"
                && source_region.as_ref().is_none_or(|r| r.0 != "us-east-1")
            {
                continue;
            }
        } else if own_region != source_region {
            continue;
        }
        let actual = super::service_evidence::known(evidence, r.target, &r.attribute);
        r.service_reference_verified =
            actual.is_some() && actual == super::service_evidence::known(evidence, s, output);
    }
}

pub(super) fn bindings(raw: &Input, r: &AttributeReference) -> bool {
    super::route53::bindings(raw, r)
}

pub(super) fn association(raw: &Input, target: usize) -> Option<(usize, usize)> {
    let endpoint = |path| {
        let mut found = raw
            .attributes
            .iter()
            .filter(|r| r.target == target && r.attribute == path);
        let r = found.next()?;
        let [source] = r.sources.as_slice() else {
            return None;
        };
        (found.next().is_none() && super::service_references::verified(raw, r)).then_some(*source)
    };
    endpoint("domain_name").zip(endpoint("api_id"))
}
