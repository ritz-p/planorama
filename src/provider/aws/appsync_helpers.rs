use super::appsync::scalar;
use crate::model::{AttributeReference, TerraformEntity};
use serde_json::Value;

const KEY: &str = "aws_appsync_api_key";

pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    if path != "api_id" {
        return None;
    }
    match kind {
        KEY => Some("AppSync key API"),
        "aws_appsync_api_cache" => Some("Cache GraphQL API"),
        "aws_appsync_type" => Some("Type GraphQL API"),
        _ => None,
    }
}

pub(super) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    evidence: &[(&Value, &Value)],
) {
    for r in refs
        .iter_mut()
        .filter(|r| label(&nodes[r.target].resource_type, &r.attribute).is_some())
    {
        r.service_reference_verified = false;
        let kind = nodes[r.target].resource_type.as_str();
        let endpoint = super::route53::endpoint(nodes, r, super::appsync::API, &["id"])
            .map(|s| (s, "id"))
            .or_else(|| {
                (kind == KEY)
                    .then(|| super::route53::endpoint(nodes, r, "aws_appsync_api", &["api_id"]))
                    .flatten()
                    .map(|s| (s, "api_id"))
            });
        let Some((s, output)) = endpoint else {
            continue;
        };
        let region = super::scope::scope(&nodes[r.target]).and_then(|s| s.region);
        r.service_reference_verified = region.is_some()
            && region == super::scope::scope(&nodes[s]).and_then(|s| s.region)
            && super::route53::configured(&nodes[r.target], &nodes[s])
            && scalar(evidence, r.target, "api_id").is_some()
            && scalar(evidence, r.target, "api_id") == scalar(evidence, s, output);
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/semantic/appsync_helpers.rs"]
mod tests;
