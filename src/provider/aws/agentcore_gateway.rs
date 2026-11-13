use crate::model::{AttributeReference, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;
use std::collections::BTreeSet;
#[cfg(test)]
#[path = "../../../tests/unit/semantic/agentcore_gateway.rs"]
mod tests;
pub(super) const REGISTRY: &str = "aws_agentregistry_registry";
pub(super) const LEGACY: &str = "aws_bedrockagentcore_registry";
pub(super) const GATEWAY: &str = "aws_bedrockagentcore_gateway";
pub(super) const RULE: &str = "aws_bedrockagentcore_gateway_rule";
pub(super) const TARGET: &str = "aws_bedrockagentcore_gateway_target";
pub(super) const APIKEY: &str = "aws_bedrockagentcore_api_key_credential_provider";
pub(super) const OAUTH: &str = "aws_bedrockagentcore_oauth2_credential_provider";
const JWT: &str = "authorizer_configuration.custom_jwt_authorizer";
const DISCOVERY: &str = "discovery_configuration.authorizer_configuration.custom_jwt_authorizer";
const CUSTOM: &str = "oauth2_provider_config.custom_oauth2_provider_config";
const VENDORS: &[(&str, &str)] = &[
    ("GoogleOauth2", "google_oauth2_provider_config"),
    ("GithubOauth2", "github_oauth2_provider_config"),
    ("SlackOauth2", "slack_oauth2_provider_config"),
    ("SalesforceOauth2", "salesforce_oauth2_provider_config"),
    ("MicrosoftOauth2", "microsoft_oauth2_provider_config"),
    ("CustomOauth2", "custom_oauth2_provider_config"),
    ("AtlassianOauth2", "atlassian_oauth2_provider_config"),
    ("LinkedinOauth2", "linkedin_oauth2_provider_config"),
];
const CONFIGS: &[&str] = &[
    "google_oauth2_provider_config",
    "github_oauth2_provider_config",
    "slack_oauth2_provider_config",
    "salesforce_oauth2_provider_config",
    "microsoft_oauth2_provider_config",
    "custom_oauth2_provider_config",
    "atlassian_oauth2_provider_config",
    "linkedin_oauth2_provider_config",
    "included_oauth2_provider_config",
];
const MANY: &[&str] = &[
    "interceptor_configuration",
    "action",
    "action.route_to_target.weighted_route.traffic_split",
    "authorizer_configuration.custom_jwt_authorizer.allowed_workload_configuration.hosting_environment",
    "authorizer_configuration.custom_jwt_authorizer.private_endpoint_overrides",
    "discovery_configuration.authorizer_configuration.custom_jwt_authorizer.private_endpoint_override",
    "oauth2_provider_config.custom_oauth2_provider_config.private_endpoint_override",
];
pub(crate) fn resource(k: &str) -> bool {
    matches!(
        k,
        REGISTRY | LEGACY | GATEWAY | RULE | TARGET | APIKEY | OAUTH
    )
}
pub(super) fn owner(k: &str) -> Option<(&'static str, &'static str, &'static str)> {
    matches!(k, RULE | TARGET).then_some((
        "gateway_identifier",
        "AgentCore gateway",
        "Gateway configuration",
    ))
}
fn private_path(k: &str, p: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let (prefix, overrides) = match k {
        REGISTRY => (DISCOVERY, "private_endpoint_override"),
        LEGACY | GATEWAY => (JWT, "private_endpoint_overrides"),
        OAUTH => (CUSTOM, "private_endpoint_override"),
        TARGET => ("", ""),
        _ => return None,
    };
    let tail = if prefix.is_empty() {
        p
    } else {
        p.strip_prefix(prefix)?.strip_prefix('.')?
    };
    let leaf = tail.strip_prefix("private_endpoint.").or_else(|| {
        tail.strip_prefix(overrides)?
            .strip_prefix(".private_endpoint.")
    })?;
    match leaf {
        "managed_vpc_resource.vpc_identifier" => Some(("aws_vpc", "id", "Private endpoint VPC")),
        "managed_vpc_resource.subnet_ids" => Some(("aws_subnet", "id", "Private endpoint subnets")),
        "managed_vpc_resource.security_group_ids" => Some((
            "aws_security_group",
            "id",
            "Private endpoint security groups",
        )),
        "self_managed_lattice_resource.resource_configuration_identifier" => Some((
            "aws_vpclattice_resource_configuration",
            "id-or-arn",
            "Private endpoint Lattice configuration",
        )),
        _ => None,
    }
}
fn rule(k: &str, p: &str) -> Option<(&'static str, &'static str, &'static str)> {
    if let Some(r) = private_path(k, p) {
        return Some(r);
    }
    if k == OAUTH {
        let tail = p.strip_prefix("oauth2_provider_config.")?;
        let (config, leaf) = tail.split_once('.')?;
        if CONFIGS.contains(&config) && leaf == "client_secret_config.secret_id" {
            return Some((
                "aws_secretsmanager_secret",
                "id-or-arn",
                "External OAuth client secret",
            ));
        }
        if config == "custom_oauth2_provider_config"
            && leaf == "private_key_jwt_config.private_key_source.kms_key_source.kms_key_arn"
        {
            return Some(("aws_kms_key", "arn", "OAuth JWT signing key"));
        }
    }
    match (k, p) {
        (REGISTRY, "encryption_configuration.kms_key_arn") | (GATEWAY, "kms_key_arn") => {
            Some(("aws_kms_key", "arn", "AgentCore encryption key"))
        }
        (APIKEY, "api_key_secret_config.secret_id") => Some((
            "aws_secretsmanager_secret",
            "id-or-arn",
            "External API key secret",
        )),
        (GATEWAY, "role_arn") => Some(("aws_iam_role", "arn", "Gateway execution role")),
        (GATEWAY, "interceptor_configuration.interceptor.lambda.arn") => Some((
            "aws_lambda_function",
            "lambda-arn",
            "Gateway interceptor Lambda",
        )),
        (GATEWAY, "policy_engine_configuration.arn") => Some((
            "aws_bedrockagentcore_policy_engine",
            "policy_engine_arn",
            "Gateway policy engine",
        )),
        (
            GATEWAY | LEGACY,
            "authorizer_configuration.custom_jwt_authorizer.allowed_workload_configuration.hosting_environment.arn",
        ) => Some((GATEWAY, "gateway_arn", "JWT hosting gateway")),
        (RULE | TARGET, "gateway_identifier") => Some((GATEWAY, "gateway_id", "AgentCore gateway")),
        (
            RULE,
            "action.route_to_target.static_route.target_name"
            | "action.route_to_target.weighted_route.traffic_split.target_name",
        ) => Some((TARGET, "name", "Gateway-qualified route targets")),
        (TARGET, "target_configuration.http.agentcore_runtime.arn") => Some((
            super::agentcore_runtime::RUNTIME,
            "agent_runtime_arn",
            "Gateway runtime target",
        )),
        (TARGET, "target_configuration.mcp.lambda.lambda_arn") => {
            Some(("aws_lambda_function", "lambda-arn", "Gateway Lambda target"))
        }
        (TARGET, "target_configuration.mcp.api_gateway.rest_api_id") => {
            Some(("aws_api_gateway_rest_api", "id", "Gateway REST API target"))
        }
        (TARGET, "target_configuration.mcp.api_gateway.stage") => Some((
            "aws_api_gateway_stage",
            "stage_name",
            "REST API-qualified stage",
        )),
        (TARGET, "credential_provider_configuration.api_key.provider_arn") => Some((
            APIKEY,
            "credential_provider_arn",
            "Gateway API key provider",
        )),
        (TARGET, "credential_provider_configuration.oauth.provider_arn") => {
            Some((OAUTH, "credential_provider_arn", "Gateway OAuth provider"))
        }
        _ => None,
    }
}
pub(super) fn label(k: &str, p: &str) -> Option<&'static str> {
    rule(k, p).map(|r| r.2)
}
fn canonical(p: &str) -> String {
    p.split('.')
        .filter(|s| s.parse::<usize>().is_err())
        .collect::<Vec<_>>()
        .join(".")
}
pub(crate) fn indexed(k: &str, p: &str) -> bool {
    p.split('.').any(|s| s.parse::<usize>().is_ok()) && label(k, &canonical(p)).is_some()
}
fn fields(k: &str, p: &str, s: &str) -> &'static [&'static str] {
    match rule(k, p).filter(|r| r.0 == s).map(|r| r.1) {
        Some("arn") => &["arn"],
        Some("id") => &["id"],
        Some("id-or-arn") => &["id", "arn"],
        Some("name") => &["name"],
        Some("lambda-arn") => &["arn", "qualified_arn"],
        Some("policy_engine_arn") => &["policy_engine_arn"],
        Some("gateway_arn") => &["gateway_arn"],
        Some("gateway_id") => &["gateway_id"],
        Some("agent_runtime_arn") => &["agent_runtime_arn"],
        Some("stage_name") => &["stage_name"],
        Some("credential_provider_arn") => &["credential_provider_arn"],
        _ => &[],
    }
}
fn value<'a>(e: &[(&'a Value, &Value)], t: usize, p: &str) -> Option<&'a Value> {
    super::nested_references::value(e[t].0, e[t].1, p)
}
fn string<'a>(e: &[(&'a Value, &Value)], t: usize, p: &str) -> Option<&'a str> {
    value(e, t, p)?.as_str().filter(|s| !s.is_empty())
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
fn shape(v: &Value, p: &str, prefix: &str) -> bool {
    let (field, rest) = p.split_once('.').map_or((p, None), |(a, b)| (a, Some(b)));
    let v = &v[field];
    if v.is_null() {
        return true;
    }
    let Some(rest) = rest else {
        return if matches!(field, "subnet_ids" | "security_group_ids") {
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
    let (min, max) = match field {
        "interceptor_configuration" | "action" => (1, 2),
        "traffic_split" => (2, 2),
        "hosting_environment" => (1, 10),
        "private_endpoint_override" | "private_endpoint_overrides" => (1, 5),
        _ => (1, 1),
    };
    v.as_array().is_some_and(|a| {
        (min..=max).contains(&a.len()) && a.iter().all(|v| v.is_object() && shape(v, rest, &prefix))
    })
}
fn exact<'a>(
    nodes: &[TerraformEntity],
    refs: &'a [AttributeReference],
    e: &[(&Value, &Value)],
    t: usize,
    p: &str,
    kind: &str,
    outputs: &[&str],
) -> Option<&'a AttributeReference> {
    let r = super::lambda_core::unique(refs, t, p)?;
    if !super::service_evidence::eligible(nodes, r) {
        return None;
    }
    let actual = value(e, t, p)?;
    let actual: BTreeSet<_> = if matches!(
        p.rsplit('.').next(),
        Some("subnet_ids" | "security_group_ids")
    ) {
        let a = actual.as_array()?;
        let s: BTreeSet<_> = a.iter().map(Value::as_str).collect::<Option<_>>()?;
        if s.len() != a.len() {
            return None;
        }
        s
    } else {
        [actual.as_str()?].into()
    };
    if actual.is_empty() || actual.contains("") {
        return None;
    }
    let expected = r
        .sources
        .iter()
        .map(|&s| {
            if super::aliases::entity_type(&nodes[s]) != kind {
                return None;
            }
            let attrs = r.source_attributes_by_instance.get(&s)?;
            if attrs.len() != 1 {
                return None;
            }
            let out = attrs.first()?.as_str();
            if !outputs.contains(&out) {
                return None;
            }
            super::service_evidence::known(e, s, out)
        })
        .collect::<Option<BTreeSet<_>>>()?;
    (actual == expected && expected.len() == r.sources.len()).then_some(r)
}
fn gateway(
    nodes: &[TerraformEntity],
    refs: &[AttributeReference],
    e: &[(&Value, &Value)],
    t: usize,
) -> Option<usize> {
    let r = exact(
        nodes,
        refs,
        e,
        t,
        "gateway_identifier",
        GATEWAY,
        &["gateway_id"],
    )?;
    let [s] = r.sources.as_slice() else {
        return None;
    };
    Some(*s)
}
fn oauth(e: &[(&Value, &Value)], t: usize, config: &str) -> bool {
    let Some(vendor) = string(e, t, "credential_provider_vendor") else {
        return false;
    };
    let selected = VENDORS
        .iter()
        .find(|v| v.0 == vendor)
        .map(|v| v.1)
        .or_else(|| {
            matches!(
                vendor,
                "XOauth2"
                    | "OktaOauth2"
                    | "OneLoginOauth2"
                    | "PingOneOauth2"
                    | "FacebookOauth2"
                    | "YandexOauth2"
                    | "RedditOauth2"
                    | "ZoomOauth2"
                    | "TwitchOauth2"
                    | "SpotifyOauth2"
                    | "DropboxOauth2"
                    | "NotionOauth2"
                    | "HubspotOauth2"
                    | "CyberArkOauth2"
                    | "FusionAuthOauth2"
                    | "Auth0Oauth2"
                    | "CognitoOauth2"
            )
            .then_some("included_oauth2_provider_config")
        });
    selected == Some(config) && choice(e, t, "oauth2_provider_config.0", config, CONFIGS)
}
fn private_gate(k: &str, e: &[(&Value, &Value)], t: usize, path: &str) -> bool {
    match k {
        REGISTRY => string(e, t, "discovery_configuration.0.authorizer_type") == Some("CUSTOM_JWT"),
        LEGACY | GATEWAY => string(e, t, "authorizer_type") == Some("CUSTOM_JWT"),
        OAUTH => {
            oauth(e, t, "custom_oauth2_provider_config")
                && (!path.contains("private_endpoint_override.")
                    || super::nested_references::singleton(
                        e[t].0,
                        e[t].1,
                        "oauth2_provider_config.0.custom_oauth2_provider_config.0.private_endpoint",
                    ))
        }
        TARGET => true,
        _ => false,
    }
}
fn private_row(
    nodes: &[TerraformEntity],
    refs: &[AttributeReference],
    e: &[(&Value, &Value)],
    r: &AttributeReference,
    path: &str,
) -> bool {
    let t = r.target;
    let k = nodes[t].resource_type.as_str();
    if !private_gate(k, e, t, path) {
        return false;
    }
    let Some((base, tail)) = path
        .rsplit_once(".private_endpoint.")
        .or_else(|| path.strip_prefix("private_endpoint.").map(|s| ("", s)))
    else {
        return false;
    };
    let endpoint = if base.is_empty() {
        "private_endpoint.0".into()
    } else {
        format!("{base}.private_endpoint.0")
    };
    let branch = tail.split('.').nth(1).unwrap_or("");
    if !choice(
        e,
        t,
        &endpoint,
        branch,
        &["managed_vpc_resource", "self_managed_lattice_resource"],
    ) {
        return false;
    }
    if branch == "self_managed_lattice_resource" {
        return true;
    }
    let base = format!("{endpoint}.managed_vpc_resource.0");
    if !matches!(
        string(e, t, &format!("{base}.endpoint_ip_address_type")),
        Some("IPV4" | "IPV6")
    ) {
        return false;
    }
    let proof = || {
        let vpc = exact(
            nodes,
            refs,
            e,
            t,
            &format!("{base}.vpc_identifier"),
            "aws_vpc",
            &["id"],
        )?;
        let [parent] = vpc.sources.as_slice() else {
            return None;
        };
        let subnets = exact(
            nodes,
            refs,
            e,
            t,
            &format!("{base}.subnet_ids"),
            "aws_subnet",
            &["id"],
        )?;
        if !value(e, t, &format!("{base}.subnet_ids"))?.is_array()
            || vpc.indexed_block_lengths != subnets.indexed_block_lengths
        {
            return None;
        }
        let mut members = subnets.sources.clone();
        let groups_path = format!("{base}.security_group_ids");
        let groups = value(e, t, &groups_path)?;
        if !groups.is_null() && !groups.as_array().is_some_and(Vec::is_empty) {
            if groups.as_array().is_none_or(|a| a.len() > 5) {
                return None;
            }
            let groups = exact(
                nodes,
                refs,
                e,
                t,
                &groups_path,
                "aws_security_group",
                &["id"],
            )?;
            if groups.indexed_block_lengths != vpc.indexed_block_lengths {
                return None;
            }
            members.extend(&groups.sources);
        }
        members
            .iter()
            .all(|&s| super::lambda_core::parent(nodes, refs, e, s) == Some(*parent))
            .then_some(())
    };
    proof().is_some()
}
fn route(
    nodes: &[TerraformEntity],
    refs: &[AttributeReference],
    e: &[(&Value, &Value)],
    r: &AttributeReference,
    s: usize,
    path: &str,
) -> bool {
    let t = r.target;
    let Some(parent) = gateway(nodes, refs, e, t) else {
        return false;
    };
    if gateway(nodes, refs, e, s) != Some(parent) {
        return false;
    }
    let parts: Vec<_> = path.split('.').collect();
    let action = format!("action.{}", parts[1]);
    if !choice(
        e,
        t,
        &action,
        "route_to_target",
        &["configuration_bundle", "route_to_target"],
    ) {
        return false;
    }
    let root = format!("{action}.route_to_target.0");
    let selected = parts[4];
    if !choice(e, t, &root, selected, &["static_route", "weighted_route"]) {
        return false;
    }
    if selected == "static_route" {
        return true;
    }
    let base = format!("{root}.weighted_route.0.traffic_split");
    let proof = || {
        let rows = e[t]
            .0
            .pointer(&format!("/{}", base.replace('.', "/")))?
            .as_array()?;
        if rows.len() != 2 {
            return None;
        }
        let mut weights = 0;
        let mut names = BTreeSet::new();
        let mut targets = BTreeSet::new();
        for i in 0..2 {
            let p = format!("{base}.{i}");
            let w = value(e, t, &format!("{p}.weight"))?.as_u64()?;
            if !(1..=99).contains(&w) {
                return None;
            }
            weights += w;
            if !names.insert(string(e, t, &format!("{p}.name"))?)
                || !targets.insert(string(e, t, &format!("{p}.target_name"))?)
            {
                return None;
            }
        }
        (weights == 100).then_some(())
    };
    proof().is_some()
}
fn target(
    nodes: &[TerraformEntity],
    refs: &[AttributeReference],
    e: &[(&Value, &Value)],
    r: &AttributeReference,
    s: usize,
) -> bool {
    let t = r.target;
    let p = r.attribute.as_str();
    let Some(gw) = gateway(nodes, refs, e, t) else {
        return false;
    };
    if p.starts_with("credential_provider_configuration.") {
        return choice(
            e,
            t,
            "credential_provider_configuration.0",
            p.split('.').nth(1).unwrap(),
            &[
                "api_key",
                "caller_iam_credentials",
                "gateway_iam_role",
                "jwt_passthrough",
                "oauth",
            ],
        );
    }
    let protocol = if p.starts_with("target_configuration.http.") {
        "http"
    } else {
        "mcp"
    };
    if !choice(
        e,
        t,
        "target_configuration.0",
        protocol,
        &["http", "inference", "mcp"],
    ) {
        return false;
    }
    if protocol == "http" {
        return value(e, gw, "protocol_type").is_some_and(Value::is_null)
            && choice(
                e,
                t,
                "target_configuration.0.http.0",
                "agentcore_runtime",
                &["agentcore_runtime", "passthrough"],
            );
    }
    if string(e, gw, "protocol_type") != Some("MCP") {
        return false;
    }
    let selected = p.split('.').nth(2).unwrap();
    if !choice(
        e,
        t,
        "target_configuration.0.mcp.0",
        selected,
        &[
            "api_gateway",
            "connector",
            "lambda",
            "mcp_server",
            "open_api_schema",
            "smithy_model",
        ],
    ) {
        return false;
    }
    if !p.ends_with(".stage") {
        return true;
    }
    let proof = || {
        let a = exact(
            nodes,
            refs,
            e,
            t,
            "target_configuration.0.mcp.0.api_gateway.0.rest_api_id",
            "aws_api_gateway_rest_api",
            &["id"],
        )?;
        let b = exact(
            nodes,
            refs,
            e,
            s,
            "rest_api_id",
            "aws_api_gateway_rest_api",
            &["id"],
        )?;
        (a.sources.len() == 1 && a.sources == b.sources).then_some(())
    };
    proof().is_some()
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
    if private_path(k, p).is_some() {
        return (k != TARGET || gateway(nodes, refs, e, t).is_some())
            && private_row(nodes, refs, e, r, path);
    }
    match (k, p) {
        (APIKEY, _) => string(e, t, "api_key_secret_source") == Some("EXTERNAL"),
        (OAUTH, _) => {
            let config = p.split('.').nth(1).unwrap();
            oauth(e, t, config)
                && (!p.ends_with("secret_id")
                    || string(
                        e,
                        t,
                        &format!("oauth2_provider_config.0.{config}.0.client_secret_source"),
                    ) == Some("EXTERNAL"))
        }
        (
            GATEWAY | LEGACY,
            "authorizer_configuration.custom_jwt_authorizer.allowed_workload_configuration.hosting_environment.arn",
        ) => string(e, t, "authorizer_type") == Some("CUSTOM_JWT"),
        (RULE | TARGET, "gateway_identifier") => true,
        (RULE, _) => route(nodes, refs, e, r, s, path),
        (TARGET, _) => target(nodes, refs, e, r, s),
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
            let k = nodes[r.target].resource_type.as_str();
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
fn gateway_bound(raw: &Input, t: usize) -> bool {
    super::lambda_core::unique(&raw.attributes, t, "gateway_identifier").is_some_and(|r| {
        r.service_reference_verified && r.sources.len() == 1 && compatible(raw, t, r.sources[0])
    })
}
pub(super) fn bindings(raw: &Input, r: &AttributeReference) -> bool {
    let k = raw.nodes[r.target].resource_type.as_str();
    if label(k, &r.attribute).is_none() {
        return true;
    }
    r.sources.iter().all(|&s| compatible(raw, r.target, s))
        && (r.attribute == "gateway_identifier"
            || (!matches!(k, RULE | TARGET) || gateway_bound(raw, r.target)))
        && (k != RULE
            || r.attribute == "gateway_identifier"
            || r.sources.iter().all(|&s| gateway_bound(raw, s)))
}
