use super::{AGENT, ALIAS, compatible};
use crate::model::{AttributeReference, TerraformEntity};
use crate::provider::aws::{lambda_core, nested_references as nested, route53, service_evidence};
use serde_json::Value;
fn known<'a>(e: &[(&'a Value, &Value)], t: usize, p: &str) -> Option<&'a str> {
    nested::value(e[t].0, e[t].1, p)?
        .as_str()
        .filter(|s| !s.is_empty())
}
fn published(v: &str) -> bool {
    v.bytes().all(|b| b.is_ascii_digit()) && v.parse::<u64>().is_ok_and(|n| n > 0)
}
fn endpoint(
    nodes: &[TerraformEntity],
    e: &[(&Value, &Value)],
    r: &AttributeReference,
    kind: &str,
    fields: &[&str],
    actual: &str,
) -> Option<usize> {
    let s = route53::endpoint(nodes, r, kind, fields)?;
    let field = r.source_attributes.iter().next()?;
    (compatible(&nodes[r.target], &nodes[s])
        && service_evidence::known(e, s, field) == Some(actual))
    .then_some(s)
}
pub(super) fn verify(
    nodes: &[TerraformEntity],
    refs: &[AttributeReference],
    e: &[(&Value, &Value)],
    r: &AttributeReference,
) -> bool {
    if lambda_core::unique(refs, r.target, &r.attribute).is_none()
        || !nested::singleton(e[r.target].0, e[r.target].1, &r.attribute)
    {
        return false;
    }
    match (nodes[r.target].resource_type.as_str(), r.attribute.as_str()) {
        (AGENT, "prompt_override_configuration") => {
            let Some(arn) = known(
                e,
                r.target,
                "prompt_override_configuration.0.override_lambda",
            ) else {
                return false;
            };
            if endpoint(nodes, e, r, "aws_lambda_function", &["arn"], arn).is_none() {
                return false;
            }
            let path = "prompt_override_configuration.0.prompt_configurations";
            if !nested::block(e[r.target].0, e[r.target].1, path, 5) {
                return false;
            }
            let Some(rows) =
                e[r.target].0["prompt_override_configuration"][0]["prompt_configurations"]
                    .as_array()
            else {
                return false;
            };
            let mut overridden = false;
            for i in 0..rows.len() {
                let base = format!("{path}.{i}");
                match known(e, r.target, &format!("{base}.parser_mode")) {
                    Some("OVERRIDDEN") => overridden = true,
                    Some("DEFAULT") => {}
                    _ => return false,
                }
                if !matches!(
                    known(e, r.target, &format!("{base}.prompt_creation_mode")),
                    Some("DEFAULT" | "OVERRIDDEN")
                ) || !matches!(
                    known(e, r.target, &format!("{base}.prompt_state")),
                    Some("ENABLED" | "DISABLED")
                ) || !matches!(
                    known(e, r.target, &format!("{base}.prompt_type")),
                    Some(
                        "PRE_PROCESSING"
                            | "ORCHESTRATION"
                            | "POST_PROCESSING"
                            | "KNOWLEDGE_BASE_RESPONSE_GENERATION"
                            | "MEMORY_SUMMARIZATION"
                    )
                ) {
                    return false;
                }
            }
            overridden
        }
        (ALIAS, "routing_configuration") => {
            let Some(_version) = known(e, r.target, "routing_configuration.0.agent_version")
                .filter(|v| published(v))
            else {
                return false;
            };
            known(
                e,
                r.target,
                "routing_configuration.0.provisioned_throughput",
            )
            .is_some_and(|v| {
                endpoint(
                    nodes,
                    e,
                    r,
                    "aws_bedrock_provisioned_model_throughput",
                    &["provisioned_model_arn"],
                    v,
                )
                .is_some()
            })
        }
        (AGENT, "guardrail_configuration") => guardrail(nodes, refs, e, r),
        _ => false,
    }
}
fn guardrail(
    nodes: &[TerraformEntity],
    refs: &[AttributeReference],
    e: &[(&Value, &Value)],
    r: &AttributeReference,
) -> bool {
    let Some(identifier) = known(
        e,
        r.target,
        "guardrail_configuration.0.guardrail_identifier",
    ) else {
        return false;
    };
    let Some(version) =
        known(e, r.target, "guardrail_configuration.0.guardrail_version").filter(|v| published(v))
    else {
        return false;
    };
    if !r.complete || !r.instances_explicit || !r.issues.is_empty() || r.sources.len() != 2 {
        return false;
    }
    let mut parent = None;
    let mut published_version = None;
    for &s in &r.sources {
        let Some(attrs) = r.source_attributes_by_instance.get(&s) else {
            return false;
        };
        let mut single = r.clone();
        single.sources = vec![s];
        single.source_attributes = attrs.clone();
        match nodes[s].resource_type.as_str() {
            "aws_bedrock_guardrail" => {
                parent = endpoint(
                    nodes,
                    e,
                    &single,
                    "aws_bedrock_guardrail",
                    &["guardrail_id", "guardrail_arn"],
                    identifier,
                )
            }
            "aws_bedrock_guardrail_version" => {
                published_version = endpoint(
                    nodes,
                    e,
                    &single,
                    "aws_bedrock_guardrail_version",
                    &["version"],
                    version,
                )
            }
            _ => return false,
        }
    }
    let (Some(parent), Some(v)) = (parent, published_version) else {
        return false;
    };
    let Some(owner) = lambda_core::unique(refs, v, "guardrail_arn") else {
        return false;
    };
    let Some(arn) = service_evidence::known(e, parent, "guardrail_arn") else {
        return false;
    };
    if service_evidence::known(e, v, "guardrail_arn") != Some(arn) {
        return false;
    }
    endpoint(
        nodes,
        e,
        owner,
        "aws_bedrock_guardrail",
        &["guardrail_arn"],
        arn,
    ) == Some(parent)
}
