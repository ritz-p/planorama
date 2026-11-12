use crate::model::{Action, AttributeReference, EntityMode, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;
use std::collections::BTreeSet;
#[cfg(test)]
#[path = "../../../tests/unit/semantic/agentcore_services.rs"]
mod tests;
pub(super) const EVALUATOR: &str = "aws_bedrockagentcore_evaluator";
pub(super) const MEMORY: &str = "aws_bedrockagentcore_memory";
pub(super) const STRATEGY: &str = "aws_bedrockagentcore_memory_strategy";
pub(super) const ONLINE: &str = "aws_bedrockagentcore_online_evaluation_config";
pub(super) const POLICY: &str = "aws_bedrockagentcore_policy";
pub(super) const ENGINE: &str = "aws_bedrockagentcore_policy_engine";
pub(super) const RESOURCE_POLICY: &str = "aws_bedrockagentcore_resource_policy";
pub(super) const VAULT: &str = "aws_bedrockagentcore_token_vault_cmk";
const MODEL: &str =
    "evaluator_config.llm_as_a_judge.model_config.bedrock_evaluator_model_config.model_id";
pub(crate) fn resource(k: &str) -> bool {
    matches!(
        k,
        EVALUATOR | MEMORY | STRATEGY | ONLINE | POLICY | ENGINE | RESOURCE_POLICY | VAULT
    )
}
pub(super) fn owner(k: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match k {
        STRATEGY => Some(("memory_id", "AgentCore memory", "Memory strategies")),
        POLICY => Some(("policy_engine_id", "AgentCore policy engine", "Policies")),
        RESOURCE_POLICY => Some((
            "resource_arn",
            "AgentCore policy target",
            "Resource policies",
        )),
        _ => None,
    }
}
fn rule(k: &str, p: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match (k, p) {
        (EVALUATOR, "kms_key_arn")
        | (MEMORY | ENGINE, "encryption_key_arn")
        | (VAULT, "kms_configuration.kms_key_arn") => {
            Some(("aws_kms_key", "arn", "AgentCore encryption key"))
        }
        (EVALUATOR, "evaluator_config.code_based.lambda_config.lambda_arn") => {
            Some(("aws_lambda_function", "lambda", "Evaluator Lambda"))
        }
        (EVALUATOR, MODEL) => Some(("", "", "Foundation model selector")),
        (MEMORY | STRATEGY, "memory_execution_role_arn")
        | (ONLINE, "evaluation_execution_role_arn") => {
            Some(("aws_iam_role", "arn", "AgentCore execution role"))
        }
        (MEMORY, "stream_delivery_resources.resource.kinesis.data_stream_arn") => {
            Some(("aws_kinesis_stream", "arn", "Memory delivery stream"))
        }
        (STRATEGY, "memory_id") => Some((MEMORY, "id", "AgentCore memory")),
        (
            STRATEGY,
            "configuration.self_managed_configuration.invocation_configuration.topic_arn",
        ) => Some(("aws_sns_topic", "arn", "Self-managed memory topic")),
        (
            STRATEGY,
            "configuration.self_managed_configuration.invocation_configuration.payload_delivery_bucket_name",
        ) => Some(("aws_s3_bucket", "bucket", "Self-managed memory bucket")),
        (ONLINE, "evaluator.evaluator_id") => {
            Some((EVALUATOR, "evaluator_id", "Custom evaluators"))
        }
        (ONLINE, "data_source_config.cloudwatch_logs.log_group_names") => {
            Some(("aws_cloudwatch_log_group", "name", "Evaluation log groups"))
        }
        (POLICY, "policy_engine_id") => {
            Some((ENGINE, "policy_engine_id", "AgentCore policy engine"))
        }
        (RESOURCE_POLICY, "resource_arn") => Some(("target", "", "AgentCore policy target")),
        _ => None,
    }
}
pub(super) fn label(k: &str, p: &str) -> Option<&'static str> {
    rule(k, p).map(|r| r.2)
}
pub(crate) fn indexed(k: &str, p: &str) -> bool {
    p.split('.').any(|s| s.parse::<usize>().is_ok())
        && label(
            k,
            &p.split('.')
                .filter(|s| s.parse::<usize>().is_err())
                .collect::<Vec<_>>()
                .join("."),
        )
        .is_some()
}
fn fields(k: &str, p: &str, s: &str) -> &'static [&'static str] {
    if k == RESOURCE_POLICY && p == "resource_arn" {
        return match s {
            super::agentcore_runtime::RUNTIME => &["agent_runtime_arn"],
            super::agentcore_gateway::GATEWAY => &["gateway_arn"],
            _ => &[],
        };
    }
    match rule(k, p).filter(|r| r.0 == s).map(|r| r.1) {
        Some("arn") => &["arn"],
        Some("lambda") => &["arn", "qualified_arn"],
        Some("id") => &["id"],
        Some("bucket") => &["bucket", "id"],
        Some("name") => &["name"],
        Some("evaluator_id") => &["evaluator_id"],
        Some("policy_engine_id") => &["policy_engine_id"],
        _ => &[],
    }
}
fn value<'a>(e: &[(&'a Value, &Value)], t: usize, p: &str) -> Option<&'a Value> {
    super::nested_references::value(e[t].0, e[t].1, p)
}
fn string<'a>(e: &[(&'a Value, &Value)], t: usize, p: &str) -> Option<&'a str> {
    value(e, t, p)?.as_str()
}
fn inactive(e: &[(&Value, &Value)], t: usize, p: &str) -> bool {
    value(e, t, p).is_some_and(|v| v.is_null() || v.as_array().is_some_and(Vec::is_empty))
}
fn choice(e: &[(&Value, &Value)], t: usize, p: &str, selected: &str, choices: &[&str]) -> bool {
    choices.iter().all(|c| {
        if *c == selected {
            super::nested_references::singleton(e[t].0, e[t].1, &format!("{p}.{c}"))
        } else {
            inactive(e, t, &format!("{p}.{c}"))
        }
    })
}
fn shape(v: &Value, p: &str) -> bool {
    let (field, rest) = p.split_once('.').map_or((p, None), |(a, b)| (a, Some(b)));
    let v = &v[field];
    if let Some(rest) = rest {
        v.as_array()
            .is_some_and(|a| a.len() == 1 && a[0].is_object() && shape(&a[0], rest))
    } else if field == "log_group_names" {
        v.is_array()
    } else {
        v.is_string()
    }
}
fn valid(nodes: &[TerraformEntity], e: &[(&Value, &Value)], r: &AttributeReference) -> bool {
    let t = r.target;
    match (nodes[t].resource_type.as_str(), r.attribute.as_str()) {
        (EVALUATOR, p) if p.starts_with("evaluator_config.code_based.") => choice(
            e,
            t,
            "evaluator_config.0",
            "code_based",
            &["code_based", "llm_as_a_judge"],
        ),
        (EVALUATOR, MODEL) => false,
        (STRATEGY, p) if p.starts_with("configuration.") => {
            string(e, t, "type") == Some("CUSTOM")
                && string(e, t, "configuration.0.type") == Some("SELF_MANAGED")
                && choice(
                    e,
                    t,
                    "configuration.0",
                    "self_managed_configuration",
                    &[
                        "self_managed_configuration",
                        "consolidation",
                        "extraction",
                        "reflection",
                    ],
                )
                && inactive(e, t, "reflection_configuration")
        }
        (VAULT, _) => string(e, t, "kms_configuration.0.key_type") == Some("CustomerManagedKey"),
        _ => true,
    }
}
fn current(n: &TerraformEntity) -> bool {
    n.provider.is_aws()
        && n.mode == EntityMode::Managed
        && n.action != Action::Delete
        && n.deposed_key.is_none()
}
fn builtin(s: &str) -> bool {
    s.strip_prefix("Builtin.").is_some_and(|v| {
        !v.is_empty()
            && v.bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
    })
}
fn custom(s: &str) -> bool {
    s.rsplit_once('-').is_some_and(|(name, id)| {
        (1..=100).contains(&name.len())
            && name.as_bytes()[0].is_ascii_alphabetic()
            && name
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
            && id.len() == 10
            && id.bytes().all(|b| b.is_ascii_alphanumeric())
    })
}
fn evaluators(
    nodes: &[TerraformEntity],
    refs: &[AttributeReference],
    e: &[(&Value, &Value)],
    r: &AttributeReference,
) -> Option<()> {
    if !current(&nodes[r.target])
        || !r.issues.is_empty()
        || !r.instances_explicit
        || super::lambda_core::unique(refs, r.target, &r.attribute).is_none()
    {
        return None;
    }
    let rows = value(e, r.target, "evaluator")?.as_array()?;
    if !(1..=10).contains(&rows.len()) {
        return None;
    }
    let mut actual = BTreeSet::new();
    let mut builtins = 0;
    for row in rows {
        let id = row.as_object()?.get("evaluator_id")?.as_str()?;
        if !actual.insert(id) {
            return None;
        }
        if builtin(id) {
            builtins += 1;
        } else if !custom(id) {
            return None;
        }
    }
    let mut expected = BTreeSet::new();
    let mut sources = BTreeSet::new();
    let mut literals = 0;
    for i in 0..rows.len() {
        let row =
            super::lambda_core::unique(refs, r.target, &format!("evaluator.{i}.evaluator_id"))?;
        if row.indexed_block_lengths != [rows.len()] || !row.issues.is_empty() {
            return None;
        }
        if row.sources.is_empty() {
            if row.has_references {
                return None;
            }
            literals += 1;
            continue;
        }
        if !row.complete || !row.instances_explicit {
            return None;
        }
        let [s] = row.sources.as_slice() else {
            return None;
        };
        let endpoint = &nodes[*s];
        if *s == r.target
            || !current(endpoint)
            || endpoint.resource_type != EVALUATOR
            || !sources.insert(*s)
        {
            return None;
        }
        let outputs = row.source_attributes_by_instance.get(s)?;
        if outputs.len() != 1 || outputs.first()?.as_str() != "evaluator_id" {
            return None;
        }
        let id = super::service_evidence::known(e, *s, "evaluator_id")?;
        if !custom(id) || !expected.insert(id) {
            return None;
        }
    }
    (literals == builtins
        && !sources.is_empty()
        && expected == actual.into_iter().filter(|id| !builtin(id)).collect()
        && sources == r.sources.iter().copied().collect())
    .then_some(())
}
pub(super) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    e: &[(&Value, &Value)],
) {
    let results: Vec<_> = refs
        .iter()
        .map(|r| {
            let k = nodes[r.target].resource_type.as_str();
            label(k, &r.attribute)?;
            Some(if k == ONLINE && r.attribute == "evaluator.evaluator_id" {
                evaluators(nodes, refs, e, r).is_some()
            } else {
                shape(e[r.target].0, &r.attribute)
                    && valid(nodes, e, r)
                    && super::nested_references::verify(nodes, refs, e, r, fields, |_, _| true)
                        .is_some()
            })
        })
        .collect();
    for (r, ok) in refs.iter_mut().zip(results) {
        if let Some(ok) = ok {
            r.service_reference_verified = ok;
            if ok {
                r.complete = true;
            }
        }
    }
}
fn compatible(raw: &Input, t: usize, s: usize) -> bool {
    let (l, r) = (&raw.nodes[t], &raw.nodes[s]);
    super::eventbridge::bindings(
        l.provider_configuration.as_ref(),
        r.provider_configuration.as_ref(),
        l.entity.scope.as_ref().and_then(|s| s.region.as_ref()),
        r.entity.scope.as_ref().and_then(|s| s.region.as_ref()),
        r.resource_type == "aws_iam_role",
    )
}
pub(super) fn bindings(raw: &Input, r: &AttributeReference) -> bool {
    let k = raw.nodes[r.target].resource_type.as_str();
    if label(k, &r.attribute).is_none() {
        return true;
    }
    r.sources.iter().all(|&s| compatible(raw, r.target, s))
        && (k != STRATEGY
            || r.attribute == "memory_id"
            || super::lambda_core::unique(&raw.attributes, r.target, "memory_id").is_some_and(
                |p| {
                    p.service_reference_verified
                        && p.sources.len() == 1
                        && compatible(raw, r.target, p.sources[0])
                },
            ))
}
