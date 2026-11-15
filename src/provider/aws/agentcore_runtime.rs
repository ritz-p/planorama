use crate::model::{AttributeReference, Graph, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;
use std::collections::BTreeSet;
#[cfg(test)]
#[path = "../../../tests/unit/semantic/agentcore_runtime.rs"]
mod tests;
pub(super) const RUNTIME: &str = "aws_bedrockagentcore_agent_runtime";
pub(super) const ENDPOINT: &str = "aws_bedrockagentcore_agent_runtime_endpoint";
pub(super) const BROWSER: &str = "aws_bedrockagentcore_browser";
pub(super) const INTERPRETER: &str = "aws_bedrockagentcore_code_interpreter";
pub(super) const HARNESS: &str = "aws_bedrockagentcore_harness";
const ENV: &str = "environment.agentcore_runtime_environment";
const MANY: &[&str] = &[
    "certificate",
    "enterprise_policy",
    "filesystem_configuration",
    "tool",
    "environment.agentcore_runtime_environment.filesystem_configuration",
];
pub(crate) fn resource(k: &str) -> bool {
    matches!(k, RUNTIME | ENDPOINT | BROWSER | INTERPRETER | HARNESS)
}
pub(super) fn owner(k: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (k == ENDPOINT).then_some((
        "agent_runtime_id",
        "Version-qualified runtime",
        "Runtime endpoints",
    ))
}
fn network(k: &str) -> Option<(&'static str, &'static str)> {
    match k {
        RUNTIME => Some((
            "network_configuration.network_mode_config.subnets",
            "network_configuration.network_mode_config.security_groups",
        )),
        BROWSER | INTERPRETER => Some((
            "network_configuration.vpc_config.subnets",
            "network_configuration.vpc_config.security_groups",
        )),
        HARNESS => Some((
            "environment.agentcore_runtime_environment.network_configuration.network_mode_config.subnets",
            "environment.agentcore_runtime_environment.network_configuration.network_mode_config.security_groups",
        )),
        _ => None,
    }
}
fn rule(k: &str, p: &str) -> Option<(&'static str, &'static str, &'static str)> {
    if let Some((subnets, groups)) = network(k) {
        if p == subnets {
            return Some(("aws_subnet", "id", "Runtime VPC subnets"));
        }
        if p == groups {
            return Some(("aws_security_group", "id", "Runtime VPC security groups"));
        }
    }
    let filesystem = if k == HARNESS {
        p.strip_prefix(ENV)
            .and_then(|p| p.strip_prefix('.'))
            .unwrap_or(p)
    } else {
        p
    };
    if matches!(k, RUNTIME | HARNESS) {
        match filesystem {
            "filesystem_configuration.efs_access_point.access_point_arn" => {
                return Some(("aws_efs_access_point", "arn", "Runtime EFS access points"));
            }
            "filesystem_configuration.s3_files_access_point.access_point_arn" => {
                return Some((
                    "aws_s3files_access_point",
                    "arn",
                    "Runtime S3 Files access points",
                ));
            }
            _ => {}
        }
    }
    match (k, p) {
        (RUNTIME, "role_arn") | (BROWSER | INTERPRETER | HARNESS, "execution_role_arn") => {
            Some(("aws_iam_role", "arn", "AgentCore execution role"))
        }
        (ENDPOINT, "agent_runtime_id") => {
            Some((RUNTIME, "agent_runtime_id", "Version-qualified runtime"))
        }
        (RUNTIME, "agent_runtime_artifact.code_configuration.code.s3.bucket") => {
            Some(("aws_s3_bucket", "bucket", "Runtime source bucket"))
        }
        (BROWSER | INTERPRETER, "certificate.location.secrets_manager.secret_arn") => Some((
            "aws_secretsmanager_secret",
            "arn",
            "AgentCore certificate secrets",
        )),
        (BROWSER, "recording.s3_location.bucket") => {
            Some(("aws_s3_bucket", "bucket", "Browser recording bucket"))
        }
        (BROWSER, "enterprise_policy.location.s3.bucket") => Some((
            "aws_s3_bucket",
            "bucket",
            "Browser enterprise policy buckets",
        )),
        (HARNESS, "memory.agentcore_memory_configuration.arn") => Some((
            "aws_bedrockagentcore_memory",
            "arn",
            "Existing AgentCore memory",
        )),
        (HARNESS, "tool.config.agentcore_browser.browser_arn") => {
            Some((BROWSER, "browser_arn", "Harness browser tools"))
        }
        (HARNESS, "tool.config.agentcore_code_interpreter.code_interpreter_arn") => Some((
            INTERPRETER,
            "code_interpreter_arn",
            "Harness interpreter tools",
        )),
        (HARNESS, "tool.config.agentcore_gateway.gateway_arn") => Some((
            "aws_bedrockagentcore_gateway",
            "gateway_arn",
            "Harness gateway tools",
        )),
        (
            HARNESS,
            "model.gemini_model_config.api_key_arn"
            | "model.litellm_model_config.api_key_arn"
            | "model.openai_model_config.api_key_arn",
        ) => Some((
            "aws_secretsmanager_secret",
            "arn",
            "Harness model credential",
        )),
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
    match rule(k, p).filter(|r| r.0 == s).map(|r| r.1) {
        Some("arn") => &["arn"],
        Some("id") => &["id"],
        Some("bucket") => &["bucket"],
        Some("agent_runtime_id") => &["agent_runtime_id"],
        Some("browser_arn") => &["browser_arn"],
        Some("code_interpreter_arn") => &["code_interpreter_arn"],
        Some("gateway_arn") => &["gateway_arn"],
        _ => &[],
    }
}
fn shape(v: &Value, p: &str, prefix: &str) -> bool {
    let (field, rest) = p.split_once('.').map_or((p, None), |(a, b)| (a, Some(b)));
    let v = &v[field];
    if v.is_null() {
        return true;
    }
    let Some(rest) = rest else {
        return if matches!(field, "subnets" | "security_groups") {
            v.is_array()
        } else {
            v.is_string()
        };
    };
    let prefix = if prefix.is_empty() {
        field.into()
    } else {
        format!("{prefix}.{field}")
    };
    v.as_array().is_some_and(|a| {
        let max = match prefix.as_str() {
            "certificate" => 200,
            "enterprise_policy" => 100,
            "tool" => usize::MAX,
            p if p.ends_with("filesystem_configuration") => 5,
            _ => 1,
        };
        !a.is_empty()
            && a.len() <= max
            && a.iter().all(|v| v.is_object() && shape(v, rest, &prefix))
    })
}
fn value<'a>(e: &[(&'a Value, &Value)], t: usize, p: &str) -> Option<&'a Value> {
    super::nested_references::value(e[t].0, e[t].1, p)
}
fn string<'a>(e: &[(&'a Value, &Value)], t: usize, p: &str) -> Option<&'a str> {
    value(e, t, p)?.as_str().filter(|s| !s.is_empty())
}
fn choice(e: &[(&Value, &Value)], t: usize, p: &str, selected: &str, choices: &[&str]) -> bool {
    choices.iter().all(|c| {
        value(e, t, &format!("{p}.{c}")).is_some_and(|v| {
            if *c == selected {
                v.as_array()
                    .is_some_and(|a| a.len() == 1 && a[0].is_object())
            } else {
                v.is_null() || v.as_array().is_some_and(Vec::is_empty)
            }
        })
    })
}
fn valid(
    nodes: &[TerraformEntity],
    refs: &[AttributeReference],
    e: &[(&Value, &Value)],
    r: &AttributeReference,
    s: usize,
    path: &str,
) -> bool {
    let t = r.target;
    let k = nodes[t].resource_type.as_str();
    let p = r.attribute.as_str();
    if network(k).is_some_and(|(a, b)| p == a || p == b) {
        let prefix = if k == HARNESS {
            "environment.0.agentcore_runtime_environment.0."
        } else {
            ""
        };
        return string(
            e,
            t,
            &format!("{prefix}network_configuration.0.network_mode"),
        ) == Some("VPC");
    }
    if k == ENDPOINT {
        let Some(version) = string(e, t, "agent_runtime_version") else {
            return false;
        };
        if version
            .parse::<u32>()
            .ok()
            .is_none_or(|v| !(1..=99999).contains(&v))
            || string(e, s, "agent_runtime_version") != Some(version)
        {
            return false;
        }
        return super::lambda_core::unique(refs, t, "agent_runtime_version").is_none_or(|r| {
            r.issues.is_empty()
                && (r.sources.is_empty()
                    || (r.sources == [s]
                        && r.source_attributes.len() == 1
                        && r.source_attributes.contains("agent_runtime_version")))
        });
    }
    if let Some((base, tail)) = path.split_once("filesystem_configuration.") {
        let row = tail.split('.').next().unwrap();
        let selected = tail.split('.').nth(1).unwrap();
        return choice(
            e,
            t,
            &format!("{base}filesystem_configuration.{row}"),
            selected,
            &[
                "efs_access_point",
                "s3_files_access_point",
                "session_storage",
            ],
        );
    }
    match (k, p) {
        (RUNTIME, "agent_runtime_artifact.code_configuration.code.s3.bucket") => choice(
            e,
            t,
            "agent_runtime_artifact.0",
            "code_configuration",
            &["code_configuration", "container_configuration"],
        ),
        (BROWSER, "recording.s3_location.bucket") => {
            value(e, t, "recording.0.enabled") == Some(&Value::Bool(true))
        }
        (HARNESS, "memory.agentcore_memory_configuration.arn") => choice(
            e,
            t,
            "memory.0",
            "agentcore_memory_configuration",
            &[
                "agentcore_memory_configuration",
                "managed_memory_configuration",
                "disabled",
            ],
        ),
        (HARNESS, p) if p.starts_with("tool.") => {
            let parts: Vec<_> = path.split('.').collect();
            let base = format!("tool.{}", parts[1]);
            let selected = parts[4];
            string(e, t, &format!("{base}.type")) == Some(selected)
                && choice(
                    e,
                    t,
                    &format!("{base}.config.0"),
                    selected,
                    &[
                        "agentcore_browser",
                        "agentcore_code_interpreter",
                        "agentcore_gateway",
                        "remote_mcp",
                        "inline_function",
                    ],
                )
        }
        (HARNESS, p) if p.starts_with("model.") => choice(
            e,
            t,
            "model.0",
            p.split('.').nth(1).unwrap(),
            &[
                "bedrock_model_config",
                "gemini_model_config",
                "litellm_model_config",
                "openai_model_config",
            ],
        ),
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
            label(k, &r.attribute)?;
            Some(
                shape(e[r.target].0, &r.attribute, "")
                    && super::nested_references::verify_collections(
                        nodes,
                        refs,
                        e,
                        r,
                        fields,
                        |s, path| valid(nodes, refs, e, r, s, path),
                        MANY,
                    )
                    .is_some(),
            )
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
    let contexts: Vec<_> = refs
        .iter()
        .map(|r| {
            let (a, b) = network(&nodes[r.target].resource_type)?;
            if r.attribute != a && r.attribute != b {
                return None;
            }
            let proof = || {
                let a = super::lambda_core::unique(refs, r.target, a)?;
                let b = super::lambda_core::unique(refs, r.target, b)?;
                if !a.service_reference_verified || !b.service_reference_verified {
                    return None;
                }
                let parents = a
                    .sources
                    .iter()
                    .chain(&b.sources)
                    .map(|&s| super::lambda_core::parent(nodes, refs, e, s))
                    .collect::<Option<BTreeSet<_>>>()?;
                (parents.len() == 1).then(|| *parents.first().unwrap())
            };
            Some(proof().into_iter().collect::<Vec<_>>())
        })
        .collect();
    for (r, c) in refs.iter_mut().zip(contexts) {
        if let Some(c) = c {
            r.endpoint_network_context = c;
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
            let (l, r) = (&raw.nodes[r.target], &raw.nodes[s]);
            super::eventbridge::bindings(
                l.provider_configuration.as_ref(),
                r.provider_configuration.as_ref(),
                l.entity.scope.as_ref().and_then(|s| s.region.as_ref()),
                r.entity.scope.as_ref().and_then(|s| s.region.as_ref()),
                r.resource_type == "aws_iam_role",
            )
        })
}
pub(super) fn present(raw: &Input, g: &mut Graph) {
    for k in [RUNTIME, BROWSER, INTERPRETER] {
        super::lambda_core::present_network(raw, g, k, network(k).unwrap(), bindings);
    }
}
