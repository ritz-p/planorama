use crate::model::{
    Action, AttributeReference, EntityMode, Graph, PresentationOwner, RegionScope, TerraformEntity,
};
use crate::semantic::Input;
use serde_json::Value;
use std::collections::BTreeSet;
#[cfg(test)]
#[path = "../../../tests/unit/semantic/cognito_extended.rs"]
mod tests;
const POOL: &str = "aws_cognito_user_pool";
const CLIENT: &str = "aws_cognito_user_pool_client";
const IDENTITY: &str = "aws_cognito_identity_pool";
const PRINCIPAL: &str = "aws_cognito_identity_pool_provider_principal_tag";
const ROLES: &str = "aws_cognito_identity_pool_roles_attachment";
const PROVIDER: &str = "aws_cognito_identity_provider";
const LOG: &str = "aws_cognito_log_delivery_configuration";
const BRAND: &str = "aws_cognito_managed_login_branding";
const MANAGED: &str = "aws_cognito_managed_user_pool_client";
const SERVER: &str = "aws_cognito_resource_server";
const RISK: &str = "aws_cognito_risk_configuration";
const USER: &str = "aws_cognito_user";
const GROUP: &str = "aws_cognito_user_group";
const MEMBER: &str = "aws_cognito_user_in_group";
const DOMAIN: &str = "aws_cognito_user_pool_domain";
const UI: &str = "aws_cognito_user_pool_ui_customization";
const OIDC: &str = "aws_iam_openid_connect_provider";
const SAML: &str = "aws_iam_saml_provider";
const COLLECTIONS: &[&str] = &[
    "log_configurations",
    "role_mapping",
    "role_mapping.mapping_rule",
    "cognito_identity_providers",
];
const TRIGGERS: &[&str] = &[
    "create_auth_challenge",
    "custom_message",
    "define_auth_challenge",
    "post_authentication",
    "post_confirmation",
    "pre_authentication",
    "pre_sign_up",
    "pre_token_generation",
    "user_migration",
    "verify_auth_challenge_response",
    "custom_email_sender.lambda_arn",
    "custom_sms_sender.lambda_arn",
    "pre_token_generation_config.lambda_arn",
];
pub(crate) fn resource(k: &str) -> bool {
    matches!(
        k,
        POOL | CLIENT
            | IDENTITY
            | PRINCIPAL
            | ROLES
            | PROVIDER
            | LOG
            | BRAND
            | MANAGED
            | SERVER
            | RISK
            | USER
            | GROUP
            | MEMBER
            | DOMAIN
            | UI
    )
}
fn pool_child(k: &str) -> bool {
    matches!(
        k,
        CLIENT
            | PROVIDER
            | LOG
            | BRAND
            | MANAGED
            | SERVER
            | RISK
            | USER
            | GROUP
            | MEMBER
            | DOMAIN
            | UI
    )
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
pub(super) fn label(k: &str, p: &str) -> Option<&'static str> {
    match (k, p) {
        (k, "user_pool_id") if pool_child(k) && k != CLIENT => Some("Cognito user pool"),
        (PRINCIPAL | ROLES, "identity_pool_id") => Some("Cognito identity pool"),
        (IDENTITY, "openid_connect_provider_arns") => Some("Identity pool OIDC providers"),
        (IDENTITY, "saml_provider_arns") => Some("Identity pool SAML providers"),
        (PRINCIPAL, "identity_provider_name") => Some("Principal tag provider"),
        (ROLES, "roles") => Some("Identity pool roles"),
        (ROLES, "role_mapping.identity_provider") => Some("Role mapping providers"),
        (ROLES, "role_mapping.mapping_rule.role_arn") => Some("Role mapping roles"),
        (BRAND | RISK | UI, "client_id") => Some("Qualified Cognito client"),
        (GROUP, "role_arn") => Some("Cognito group role"),
        (MEMBER, "group_name") => Some("Qualified Cognito group"),
        (MEMBER, "username") => Some("Qualified Cognito user"),
        (DOMAIN, "certificate_arn") => Some("Cognito domain certificate"),
        (CLIENT, "supported_identity_providers") => Some("Client identity providers"),
        (
            CLIENT,
            "analytics_configuration.application_arn" | "analytics_configuration.application_id",
        ) => Some("Cognito analytics application"),
        (CLIENT, "analytics_configuration.role_arn") => Some("Cognito analytics role"),
        (POOL, p)
            if p.strip_prefix("lambda_config.")
                .is_some_and(|p| TRIGGERS.contains(&p)) =>
        {
            Some("Cognito Lambda trigger")
        }
        (POOL, "lambda_config.kms_key_id") => Some("Cognito sender encryption key"),
        (POOL, "sms_configuration.sns_caller_arn") => Some("Cognito SMS role"),
        (POOL, "email_configuration.source_arn")
        | (RISK, "account_takeover_risk_configuration.notify_configuration.source_arn") => {
            Some("Cognito email identity")
        }
        (LOG, "log_configurations.cloud_watch_logs_configuration.log_group_arn") => {
            Some("Cognito log groups")
        }
        (LOG, "log_configurations.s3_configuration.bucket_arn") => Some("Cognito log buckets"),
        (LOG, "log_configurations.firehose_configuration.stream_arn") => {
            Some("Cognito log streams")
        }
        _ => None,
    }
}
fn fields(k: &str, p: &str, s: &str) -> &'static [&'static str] {
    match (k, p, s) {
        (k, "user_pool_id", POOL) if pool_child(k) => &["id"],
        (PRINCIPAL | ROLES, "identity_pool_id", IDENTITY) => &["id"],
        (IDENTITY, "openid_connect_provider_arns", OIDC)
        | (IDENTITY, "saml_provider_arns", SAML) => &["arn"],
        (IDENTITY, "cognito_identity_providers.provider_name", POOL) => &["endpoint"],
        (IDENTITY, "cognito_identity_providers.client_id", CLIENT) => &["id"],
        (PRINCIPAL, "identity_provider_name", POOL) => &["endpoint"],
        (PRINCIPAL, "identity_provider_name", OIDC | SAML) => &["arn"],
        (ROLES, "roles" | "role_mapping.mapping_rule.role_arn", "aws_iam_role") => &["arn"],
        (BRAND | RISK | UI, "client_id", CLIENT | MANAGED) => &["id"],
        (GROUP, "role_arn", "aws_iam_role") => &["arn"],
        (MEMBER, "group_name", GROUP) => &["name"],
        (MEMBER, "username", USER) => &["username"],
        (DOMAIN, "certificate_arn", "aws_acm_certificate") => &["arn"],
        (CLIENT, "supported_identity_providers", PROVIDER) => &["provider_name"],
        (CLIENT, "analytics_configuration.application_arn", "aws_pinpoint_app") => &["arn"],
        (CLIENT, "analytics_configuration.application_id", "aws_pinpoint_app") => {
            &["application_id"]
        }
        (CLIENT, "analytics_configuration.role_arn", "aws_iam_role") => &["arn"],
        (POOL, p, "aws_lambda_function")
            if p.strip_prefix("lambda_config.")
                .is_some_and(|p| TRIGGERS.contains(&p)) =>
        {
            &["arn", "qualified_arn"]
        }
        (POOL, p, "aws_lambda_alias")
            if p.strip_prefix("lambda_config.")
                .is_some_and(|p| TRIGGERS.contains(&p)) =>
        {
            &["arn"]
        }
        (POOL, "lambda_config.kms_key_id", "aws_kms_key")
        | (POOL, "sms_configuration.sns_caller_arn", "aws_iam_role") => &["arn"],
        (
            POOL,
            "email_configuration.source_arn",
            "aws_ses_email_identity" | "aws_ses_domain_identity" | "aws_sesv2_email_identity",
        )
        | (
            RISK,
            "account_takeover_risk_configuration.notify_configuration.source_arn",
            "aws_ses_email_identity" | "aws_ses_domain_identity" | "aws_sesv2_email_identity",
        ) => &["arn"],
        (
            LOG,
            "log_configurations.cloud_watch_logs_configuration.log_group_arn",
            "aws_cloudwatch_log_group",
        )
        | (LOG, "log_configurations.s3_configuration.bucket_arn", "aws_s3_bucket")
        | (
            LOG,
            "log_configurations.firehose_configuration.stream_arn",
            "aws_kinesis_firehose_delivery_stream",
        ) => &["arn"],
        _ => &[],
    }
}
fn current(n: &TerraformEntity) -> bool {
    n.provider.is_aws()
        && n.mode == EntityMode::Managed
        && n.action != Action::Delete
        && n.deposed_key.is_none()
}
fn value<'a>(e: &[(&'a Value, &Value)], t: usize, p: &str) -> Option<&'a Value> {
    let (v, m) = e[t];
    super::nested_references::value(v, m, p)
}
fn string<'a>(e: &[(&'a Value, &Value)], t: usize, p: &str) -> Option<&'a str> {
    value(e, t, p)?.as_str().filter(|s| !s.is_empty())
}
fn absent(e: &[(&Value, &Value)], t: usize, p: &str) -> bool {
    value(e, t, p).is_some_and(|v| v.is_null() || v.as_array().is_some_and(Vec::is_empty))
}
fn shape(v: &Value, p: &[&str], prefix: &str) -> bool {
    if p.is_empty() {
        return v.is_string()
            || v.as_array()
                .is_some_and(|a| !a.is_empty() && a.iter().all(Value::is_string));
    }
    if let Some(a) = v.as_array() {
        return (a.len() == 1 || COLLECTIONS.contains(&prefix))
            && a.iter().all(|v| shape(v, p, prefix));
    }
    let path = if prefix.is_empty() {
        p[0].into()
    } else {
        format!("{prefix}.{}", p[0])
    };
    v.is_object() && (v[p[0]].is_null() || shape(&v[p[0]], &p[1..], &path))
}
fn simple(
    nodes: &[TerraformEntity],
    refs: &[AttributeReference],
    e: &[(&Value, &Value)],
    r: &AttributeReference,
) -> bool {
    shape(
        e[r.target].0,
        &r.attribute.split('.').collect::<Vec<_>>(),
        "",
    ) && super::nested_references::verify_collections(
        nodes,
        refs,
        e,
        r,
        fields,
        |_, _| true,
        COLLECTIONS,
    )
    .is_some()
}
fn parent(
    nodes: &[TerraformEntity],
    refs: &[AttributeReference],
    e: &[(&Value, &Value)],
    t: usize,
) -> Option<usize> {
    if !current(&nodes[t]) {
        return None;
    }
    if nodes[t].resource_type == POOL {
        return Some(t);
    }
    let path = if matches!(nodes[t].resource_type.as_str(), PRINCIPAL | ROLES) {
        "identity_pool_id"
    } else {
        "user_pool_id"
    };
    let r = super::lambda_core::unique(refs, t, path)?;
    let [s] = r.sources.as_slice() else {
        return None;
    };
    simple(nodes, refs, e, r).then_some(*s)
}
fn same_pool(
    nodes: &[TerraformEntity],
    refs: &[AttributeReference],
    e: &[(&Value, &Value)],
    a: usize,
    b: usize,
) -> bool {
    parent(nodes, refs, e, a)
        .zip(parent(nodes, refs, e, b))
        .is_some_and(|(a, b)| a == b)
}
fn member_provider(
    nodes: &[TerraformEntity],
    refs: &[AttributeReference],
    e: &[(&Value, &Value)],
    pool: usize,
    s: usize,
) -> bool {
    let p = match nodes[s].resource_type.as_str() {
        POOL => "cognito_identity_providers.provider_name",
        CLIENT => "cognito_identity_providers.client_id",
        OIDC => "openid_connect_provider_arns",
        SAML => "saml_provider_arns",
        _ => return false,
    };
    super::lambda_core::unique(refs, pool, p).is_some_and(|r| {
        r.sources.contains(&s)
            && if matches!(nodes[s].resource_type.as_str(), POOL | CLIENT) {
                r.service_reference_verified && r.complete
            } else {
                simple(nodes, refs, e, r)
            }
    })
}
fn output<'a>(
    nodes: &[TerraformEntity],
    e: &[(&'a Value, &Value)],
    r: &AttributeReference,
    s: usize,
    allowed: &[&str],
) -> Option<&'a str> {
    if !current(&nodes[s]) || s == r.target {
        return None;
    }
    let a = r.source_attributes_by_instance.get(&s)?;
    let mut a = a.iter();
    let out = a.next()?;
    if a.next().is_some() || !allowed.contains(&out.as_str()) {
        return None;
    }
    super::service_evidence::known(e, s, out)
}
fn eligible(nodes: &[TerraformEntity], r: &AttributeReference) -> bool {
    current(&nodes[r.target])
        && r.complete
        && r.instances_explicit
        && r.issues.is_empty()
        && !r.sources.is_empty()
}
fn role_map(
    nodes: &[TerraformEntity],
    e: &[(&Value, &Value)],
    r: &AttributeReference,
) -> Option<()> {
    if !eligible(nodes, r) {
        return None;
    }
    let (v, m) = e[r.target];
    let map = v["roles"].as_object()?;
    if m["roles"].as_object().is_some_and(|mask| {
        mask.iter().any(|(key, value)| {
            !map.contains_key(key) || !matches!(value, Value::Null | Value::Bool(false))
        })
    }) {
        return None;
    }
    if map.is_empty()
        || !map
            .keys()
            .all(|k| matches!(k.as_str(), "authenticated" | "unauthenticated"))
    {
        return None;
    }
    let actual: BTreeSet<_> = map
        .keys()
        .map(|k| {
            super::nested_references::value(v, m, &format!("roles.{k}")).and_then(Value::as_str)
        })
        .collect::<Option<_>>()?;
    let expected: BTreeSet<_> = r
        .sources
        .iter()
        .map(|&s| {
            (nodes[s].resource_type == "aws_iam_role")
                .then(|| output(nodes, e, r, s, &["arn"]))
                .flatten()
        })
        .collect::<Option<_>>()?;
    (!actual.contains("") && actual == expected && expected.len() == r.sources.len()).then_some(())
}
fn providers(
    nodes: &[TerraformEntity],
    refs: &[AttributeReference],
    e: &[(&Value, &Value)],
    r: &AttributeReference,
) -> Option<()> {
    if !eligible(nodes, r) {
        return None;
    }
    let values = value(e, r.target, "supported_identity_providers")?.as_array()?;
    let actual: BTreeSet<_> = values.iter().map(Value::as_str).collect::<Option<_>>()?;
    if actual.len() != values.len() {
        return None;
    }
    let actual: BTreeSet<_> = actual.into_iter().filter(|s| *s != "COGNITO").collect();
    let expected: BTreeSet<_> = r
        .sources
        .iter()
        .map(|&s| {
            (nodes[s].resource_type == PROVIDER
                && same_pool(nodes, refs, e, r.target, s)
                && matches!(
                    string(e, s, "provider_type"),
                    Some(
                        "SAML"
                            | "OIDC"
                            | "Google"
                            | "Facebook"
                            | "LoginWithAmazon"
                            | "SignInWithApple"
                    )
                ))
            .then(|| output(nodes, e, r, s, &["provider_name"]))
            .flatten()
        })
        .collect::<Option<_>>()?;
    (!actual.is_empty() && actual == expected && expected.len() == r.sources.len()).then_some(())
}
fn mapping(
    nodes: &[TerraformEntity],
    refs: &[AttributeReference],
    e: &[(&Value, &Value)],
    t: usize,
    i: usize,
) -> Option<Vec<usize>> {
    let pool = parent(nodes, refs, e, t)?;
    let path = format!("role_mapping.{i}.identity_provider");
    let r = super::lambda_core::unique(refs, t, &path)?;
    if !eligible(nodes, r)
        || !matches!(
            string(e, t, &format!("role_mapping.{i}.type")),
            Some("Rules" | "Token")
        )
    {
        return None;
    }
    let rows = e[t].0["role_mapping"].as_array()?;
    if r.indexed_block_lengths != [rows.len()] {
        return None;
    }
    let actual = string(e, t, &path)?;
    match r.sources.as_slice() {
        [s] if matches!(nodes[*s].resource_type.as_str(), OIDC | SAML) => {
            (output(nodes, e, r, *s, &["arn"]) == Some(actual)
                && member_provider(nodes, refs, e, pool, *s))
            .then(|| vec![*s])
        }
        [_, _] => {
            let up = *r
                .sources
                .iter()
                .find(|&&s| nodes[s].resource_type == POOL)?;
            let client = *r
                .sources
                .iter()
                .find(|&&s| nodes[s].resource_type == CLIENT)?;
            let endpoint = output(nodes, e, r, up, &["endpoint"])?;
            let id = output(nodes, e, r, client, &["id"])?;
            let configured = e[pool].0["cognito_identity_providers"].as_array()?;
            let mut pairs = 0;
            for i in 0..configured.len() {
                let provider = string(
                    e,
                    pool,
                    &format!("cognito_identity_providers.{i}.provider_name"),
                )?;
                let client_id = string(
                    e,
                    pool,
                    &format!("cognito_identity_providers.{i}.client_id"),
                )?;
                pairs += usize::from(provider == endpoint && client_id == id);
            }
            (actual == format!("{endpoint}:{id}")
                && pairs == 1
                && same_pool(nodes, refs, e, up, client)
                && member_provider(nodes, refs, e, pool, up)
                && member_provider(nodes, refs, e, pool, client))
            .then(|| vec![up, client])
        }
        _ => None,
    }
}
fn mappings(
    nodes: &[TerraformEntity],
    refs: &[AttributeReference],
    e: &[(&Value, &Value)],
    r: &AttributeReference,
) -> Option<()> {
    if !eligible(nodes, r) {
        return None;
    }
    let rows = e[r.target].0["role_mapping"].as_array()?;
    let mut sources = BTreeSet::new();
    for i in 0..rows.len() {
        sources.extend(mapping(nodes, refs, e, r.target, i)?);
    }
    (sources == r.sources.iter().copied().collect()).then_some(())
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
    if matches!(
        (k, p),
        (BRAND | RISK | UI, "client_id") | (MEMBER, "group_name" | "username")
    ) {
        return same_pool(nodes, refs, e, t, s) && string(e, t, p) != Some("ALL");
    }
    if k == PRINCIPAL && p == "identity_provider_name" {
        return parent(nodes, refs, e, t)
            .is_some_and(|pool| member_provider(nodes, refs, e, pool, s));
    }
    if k == ROLES && p == "role_mapping.mapping_rule.role_arn" {
        return path
            .split('.')
            .nth(1)
            .and_then(|i| i.parse().ok())
            .is_some_and(|i| {
                mapping(nodes, refs, e, t, i).is_some()
                    && string(e, t, &format!("role_mapping.{i}.type")) == Some("Rules")
            });
    }
    if k == LOG && p.starts_with("log_configurations.") {
        let Some(i) = path.split('.').nth(1) else {
            return false;
        };
        return match (
            string(e, t, &format!("log_configurations.{i}.event_source")),
            string(e, t, &format!("log_configurations.{i}.log_level")),
        ) {
            (Some("userNotification"), Some("ERROR")) => {
                p.contains(".cloud_watch_logs_configuration.")
            }
            (Some("userAuthEvents"), Some("INFO")) => true,
            _ => false,
        };
    }
    if k == POOL && p.starts_with("lambda_config.") {
        if p == "lambda_config.pre_token_generation"
            && !absent(e, t, "lambda_config.0.pre_token_generation_config")
        {
            return string(e, t, path)
                == string(
                    e,
                    t,
                    "lambda_config.0.pre_token_generation_config.0.lambda_arn",
                )
                && matches!(
                    string(
                        e,
                        t,
                        "lambda_config.0.pre_token_generation_config.0.lambda_version"
                    ),
                    Some("V1_0" | "V2_0" | "V3_0")
                );
        }
        if p.contains("custom_email_sender.") || p.contains("custom_sms_sender.") {
            let block = p.split('.').nth(1).unwrap();
            return string(e, t, &format!("lambda_config.0.{block}.0.lambda_version"))
                == Some("V1_0");
        }
        if p == "lambda_config.pre_token_generation_config.lambda_arn" {
            return matches!(
                string(
                    e,
                    t,
                    "lambda_config.0.pre_token_generation_config.0.lambda_version"
                ),
                Some("V1_0" | "V2_0" | "V3_0")
            ) && (absent(e, t, "lambda_config.0.pre_token_generation")
                || string(e, t, "lambda_config.0.pre_token_generation") == string(e, t, path));
        }
    }
    if k == POOL && p == "email_configuration.source_arn" {
        return matches!(
            string(e, t, "email_configuration.0.email_sending_account"),
            Some("DEVELOPER" | "COGNITO_DEFAULT")
        );
    }
    if k == CLIENT && p.starts_with("analytics_configuration.") {
        let arn = string(e, t, "analytics_configuration.0.application_arn");
        let id = string(e, t, "analytics_configuration.0.application_id");
        return if p.ends_with("application_arn") {
            arn.is_some()
                && absent(e, t, "analytics_configuration.0.application_id")
                && absent(e, t, "analytics_configuration.0.role_arn")
                && absent(e, t, "analytics_configuration.0.external_id")
        } else {
            id.is_some()
                && absent(e, t, "analytics_configuration.0.application_arn")
                && string(e, t, "analytics_configuration.0.role_arn").is_some()
                && string(e, t, "analytics_configuration.0.external_id").is_some()
        };
    }
    true
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
            let ok = match (k, r.attribute.as_str()) {
                (ROLES, "roles") => role_map(nodes, e, r).is_some(),
                (CLIENT, "supported_identity_providers") => providers(nodes, refs, e, r).is_some(),
                (ROLES, "role_mapping.identity_provider") => mappings(nodes, refs, e, r).is_some(),
                _ => {
                    shape(
                        e[r.target].0,
                        &r.attribute.split('.').collect::<Vec<_>>(),
                        "",
                    ) && super::nested_references::verify_collections(
                        nodes,
                        refs,
                        e,
                        r,
                        fields,
                        |s, p| valid(nodes, refs, e, r, s, p),
                        COLLECTIONS,
                    )
                    .is_some()
                }
            };
            Some(ok)
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
fn compatible(raw: &Input, a: usize, b: usize, region: Option<&RegionScope>) -> bool {
    let (l, r) = (&raw.nodes[a], &raw.nodes[b]);
    super::eventbridge::bindings(
        l.provider_configuration.as_ref(),
        r.provider_configuration.as_ref(),
        region.or_else(|| l.entity.scope.as_ref().and_then(|s| s.region.as_ref())),
        r.entity.scope.as_ref().and_then(|s| s.region.as_ref()),
        matches!(r.resource_type.as_str(), "aws_iam_role" | OIDC | SAML),
    )
}
fn parent_bound(raw: &Input, t: usize) -> bool {
    if raw.nodes[t].resource_type == POOL {
        return true;
    }
    let path = if matches!(raw.nodes[t].resource_type.as_str(), PRINCIPAL | ROLES) {
        "identity_pool_id"
    } else {
        "user_pool_id"
    };
    super::lambda_core::unique(&raw.attributes, t, path).is_some_and(|r| {
        r.service_reference_verified
            && r.sources.len() == 1
            && compatible(raw, t, r.sources[0], None)
    })
}
pub(super) fn bindings(raw: &Input, r: &AttributeReference) -> bool {
    let k = raw.nodes[r.target].resource_type.as_str();
    let p = r.attribute.as_str();
    if label(k, p).is_none() {
        return true;
    }
    let region = (k == DOMAIN && p == "certificate_arn").then(|| RegionScope("us-east-1".into()));
    if !r
        .sources
        .iter()
        .all(|&s| compatible(raw, r.target, s, region.as_ref()))
    {
        return false;
    }
    if matches!(
        (k, p),
        (BRAND | RISK | UI, "client_id")
            | (MEMBER, "group_name" | "username")
            | (CLIENT, "supported_identity_providers")
    ) {
        return parent_bound(raw, r.target) && r.sources.iter().all(|&s| parent_bound(raw, s));
    }
    if k == ROLES
        && p == "role_mapping.mapping_rule.role_arn"
        && !super::lambda_core::unique(&raw.attributes, r.target, "role_mapping.identity_provider")
            .is_some_and(|p| super::service_references::verified(raw, p))
    {
        return false;
    }
    if matches!(
        (k, p),
        (PRINCIPAL, "identity_provider_name")
            | (
                ROLES,
                "role_mapping.identity_provider" | "role_mapping.mapping_rule.role_arn"
            )
    ) {
        return parent_bound(raw, r.target)
            && r.sources
                .iter()
                .filter(|&&s| raw.nodes[s].resource_type == CLIENT)
                .all(|&s| parent_bound(raw, s));
    }
    true
}
pub(super) fn present(raw: &Input, g: &mut Graph) {
    for (t, n) in raw.nodes.iter().enumerate() {
        let candidates: &[&str] = match n.resource_type.as_str() {
            PRINCIPAL | ROLES => &["identity_pool_id"],
            BRAND => &["client_id"],
            RISK | UI => &["client_id", "user_pool_id"],
            PROVIDER | LOG | SERVER | USER | GROUP | DOMAIN => &["user_pool_id"],
            _ => continue,
        };
        for &path in candidates {
            let Some(r) = super::lambda_core::unique(&raw.attributes, t, path)
                .filter(|r| super::service_references::verified(raw, r))
            else {
                continue;
            };
            let [s] = r.sources.as_slice() else {
                continue;
            };
            if !g.nodes[t].presentation_scope_matches(&g.nodes[*s]) {
                continue;
            }
            g.nodes[t].member_owner = Some(PresentationOwner {
                target: raw.nodes[*s].entity.provenance[0].clone(),
                attribute: if path == "client_id" {
                    "client_id"
                } else if path == "identity_pool_id" {
                    "identity_pool_id"
                } else {
                    "user_pool_id"
                },
                label: label(&n.resource_type, path).unwrap(),
                section: "Cognito configuration",
            });
            break;
        }
    }
}
