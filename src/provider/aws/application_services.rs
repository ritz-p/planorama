use super::configured_links::{self, Evidence, Rule};
use crate::model::{AttributeReference, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;

const AMPLIFY_APP: &str = "aws_amplify_app";
const AMPLIFY_BACKEND: &str = "aws_amplify_backend_environment";
const AMPLIFY_BRANCH: &str = "aws_amplify_branch";

const AMPLIFY_DOMAIN: &str = "aws_amplify_domain_association";
const AMPLIFY_WEBHOOK: &str = "aws_amplify_webhook";

const CONFIG_APP: &str = "aws_appconfig_application";
const CONFIG_ENV: &str = "aws_appconfig_environment";
const CONFIG_PROFILE: &str = "aws_appconfig_configuration_profile";
const CONFIG_VERSION: &str = "aws_appconfig_hosted_configuration_version";

const CONFIG_DEPLOYMENT: &str = "aws_appconfig_deployment";

const CONFIG_EXTENSION: &str = "aws_appconfig_extension";
const CONFIG_ASSOCIATION: &str = "aws_appconfig_extension_association";
const ACTION_BLOCKS: &[&str] = &["action_point", "action_point.action"];

const FABRIC_BUNDLE: &str = "aws_appfabric_app_bundle";
const FABRIC_AUTH: &str = "aws_appfabric_app_authorization";
const FABRIC_CONNECTION: &str = "aws_appfabric_app_authorization_connection";

const FABRIC_INGESTION: &str = "aws_appfabric_ingestion";
const FABRIC_DESTINATION: &str = "aws_appfabric_ingestion_destination";
const FABRIC_S3: &str = "destination_configuration.audit_log.destination.s3_bucket.bucket_name";
const FABRIC_FIREHOSE: &str =
    "destination_configuration.audit_log.destination.firehose_stream.stream_name";

pub(crate) fn has_indexed(kind: &str) -> bool {
    matches!(
        kind,
        AMPLIFY_DOMAIN | CONFIG_ENV | CONFIG_EXTENSION | FABRIC_DESTINATION
    )
}

pub(crate) fn indexed(kind: &str, path: &str) -> bool {
    has_indexed(kind) && configured_links::indexed(kind, path, rule)
}

fn rule(kind: &str, path: &str) -> Option<Rule> {
    Some(match (kind, path) {
        (AMPLIFY_APP, "iam_service_role_arn") => role("Amplify build service role"),
        (AMPLIFY_APP, "compute_role_arn") => role("Amplify SSR compute role"),
        (AMPLIFY_BACKEND | AMPLIFY_BRANCH | AMPLIFY_DOMAIN | AMPLIFY_WEBHOOK, "app_id") => {
            scalar("Amplify app", &[(AMPLIFY_APP, &["id"])])
        }
        (AMPLIFY_BRANCH, "backend_environment_arn") => scalar(
            "Amplify backend environment",
            &[(AMPLIFY_BACKEND, &["arn"])],
        ),
        (AMPLIFY_WEBHOOK, "branch_name") => scalar(
            "Amplify target branch",
            &[(AMPLIFY_BRANCH, &["branch_name"])],
        ),
        (AMPLIFY_DOMAIN, "sub_domain.branch_name") => Rule::scalar_set(
            "Amplify target branches",
            &[(AMPLIFY_BRANCH, &["branch_name"])],
        ),
        (AMPLIFY_DOMAIN, "certificate_settings.custom_certificate_arn") => scalar(
            "Amplify custom certificate",
            &[("aws_acm_certificate", &["arn"])],
        )
        .in_control_plane_account(),
        (CONFIG_ENV | CONFIG_PROFILE | CONFIG_VERSION | CONFIG_DEPLOYMENT, "application_id") => {
            scalar("AppConfig application", &[(CONFIG_APP, &["id"])])
        }
        (CONFIG_VERSION | CONFIG_DEPLOYMENT, "configuration_profile_id") => scalar(
            "AppConfig configuration profile",
            &[(CONFIG_PROFILE, &["configuration_profile_id"])],
        ),
        (CONFIG_ENV, "monitor.alarm_arn") => Rule::scalar_set(
            "AppConfig monitor alarms",
            &[
                ("aws_cloudwatch_metric_alarm", &["arn"]),
                ("aws_cloudwatch_composite_alarm", &["arn"]),
            ],
        ),
        (CONFIG_ENV, "monitor.alarm_role_arn") => {
            Rule::scalar_set("AppConfig monitor roles", &[("aws_iam_role", &["arn"])])
        }
        (CONFIG_PROFILE, "retrieval_role_arn") => role("AppConfig retrieval role"),
        (CONFIG_PROFILE, "kms_key_identifier") => key("AppConfig hosted configuration key"),
        (CONFIG_PROFILE, "location_uri") => scalar(
            "AppConfig external configuration",
            &[
                ("aws_s3_object", &["key"]),
                ("aws_ssm_document", &["name", "arn"]),
                ("aws_ssm_parameter", &["name", "arn"]),
            ],
        )
        .decoded(configuration_locator),
        (CONFIG_DEPLOYMENT, "environment_id") => scalar(
            "AppConfig deployment environment",
            &[(CONFIG_ENV, &["environment_id"])],
        ),
        (CONFIG_DEPLOYMENT, "deployment_strategy_id") => scalar(
            "AppConfig deployment strategy",
            &[("aws_appconfig_deployment_strategy", &["id"])],
        ),
        (CONFIG_DEPLOYMENT, "configuration_version") => scalar(
            "AppConfig deployed hosted version",
            &[(CONFIG_VERSION, &["version_number", "version_label"])],
        ),
        (CONFIG_DEPLOYMENT, "kms_key_identifier") => key("AppConfig deployment encryption key"),
        (CONFIG_EXTENSION, "action_point.action.uri") => scalar(
            "AppConfig extension action",
            &[
                ("aws_lambda_function", &["arn"]),
                ("aws_lambda_alias", &["arn"]),
                ("aws_sns_topic", &["arn"]),
                ("aws_sqs_queue", &["arn"]),
                ("aws_cloudwatch_event_bus", &["arn"]),
            ],
        )
        .unordered(ACTION_BLOCKS),
        (CONFIG_EXTENSION, "action_point.action.role_arn") => {
            role("AppConfig extension action role").unordered(ACTION_BLOCKS)
        }
        (CONFIG_ASSOCIATION, "extension_arn") => {
            scalar("AppConfig extension", &[(CONFIG_EXTENSION, &["arn"])])
        }
        (CONFIG_ASSOCIATION, "resource_arn") => scalar(
            "AppConfig extension target",
            &[
                (CONFIG_APP, &["arn"]),
                (CONFIG_ENV, &["arn"]),
                (CONFIG_PROFILE, &["arn"]),
            ],
        ),
        (
            FABRIC_AUTH | FABRIC_CONNECTION | FABRIC_INGESTION | FABRIC_DESTINATION,
            "app_bundle_arn",
        ) => scalar("AppFabric bundle", &[(FABRIC_BUNDLE, &["arn"])]),
        (FABRIC_CONNECTION, "app_authorization_arn") => {
            scalar("AppFabric authorization", &[(FABRIC_AUTH, &["arn"])])
        }
        (FABRIC_BUNDLE, "customer_managed_key_arn") => scalar(
            "AppFabric bundle encryption key",
            &[("aws_kms_key", &["arn"])],
        ),
        (FABRIC_INGESTION, "authorization") => scalar(
            "AppFabric ingestion authorization",
            &[(FABRIC_AUTH, &["arn"])],
        ),
        (FABRIC_DESTINATION, "ingestion_arn") => {
            scalar("AppFabric ingestion", &[(FABRIC_INGESTION, &["arn"])])
        }
        (FABRIC_DESTINATION, FABRIC_S3) => scalar(
            "AppFabric delivery bucket",
            &[("aws_s3_bucket", &["id", "bucket"])],
        ),
        (FABRIC_DESTINATION, FABRIC_FIREHOSE) => scalar(
            "AppFabric delivery stream",
            &[("aws_kinesis_firehose_delivery_stream", &["name"])],
        ),
        _ => return None,
    })
}

fn key(label: &'static str) -> Rule {
    scalar(
        label,
        &[
            ("aws_kms_key", &["arn", "key_id", "id"]),
            ("aws_kms_alias", &["arn", "name"]),
        ],
    )
}

fn s3_object(uri: &str) -> Option<(&str, &str)> {
    let (bucket, key) = uri.strip_prefix("s3://")?.split_once('/')?;
    (!bucket.is_empty() && !key.is_empty() && !uri.contains(['?', '#'])).then_some((bucket, key))
}

fn configuration_locator<'a>(
    _: &str,
    value: &'a Value,
) -> Option<std::collections::BTreeSet<&'a str>> {
    let uri = value.as_str()?;
    let id = s3_object(uri)
        .map(|(_, key)| key)
        .or_else(|| uri.strip_prefix("ssm-document://"))
        .or_else(|| uri.strip_prefix("ssm-parameter://"))
        .or_else(|| uri.starts_with("arn:").then_some(uri))?;
    (!id.is_empty()).then(|| [id].into())
}

fn role(label: &'static str) -> Rule {
    scalar(label, &[("aws_iam_role", &["arn"])])
}

fn scalar(label: &'static str, outputs: &'static [(&str, &[&str])]) -> Rule {
    Rule::new(label, outputs, false, &[])
}

pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    rule(kind, path).map(|rule| rule.label)
}

pub(super) fn bindings(raw: &Input, reference: &AttributeReference) -> bool {
    configured_links::bindings(raw, reference, rule)
}

pub(super) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    values: &[(&Value, &Value)],
) {
    configured_links::annotate(nodes, refs, values, rule, qualifies);
    annotate_versions(nodes, refs, values);
    annotate_fabric_connections(nodes, refs, values);
}

fn arn_resource<'a>(
    node: &TerraformEntity,
    arn: &'a str,
    service: &str,
    regional: bool,
) -> Option<&'a str> {
    let mut parts = arn.splitn(6, ':');
    if parts.next()? != "arn" || !parts.next()?.starts_with("aws") || parts.next()? != service {
        return None;
    }
    let region = parts.next()?;
    let account = parts.next()?;
    let resource = parts.next()?;
    let config = node.provider_configuration.as_ref()?;
    let valid_region = if regional {
        super::scope::scope(node)
            .and_then(|scope| scope.region)
            .is_some_and(|actual| actual.0 == region)
    } else {
        region.is_empty()
    };
    (valid_region
        && !config.account_conflict
        && crate::model::AwsAccountScope::new(account).is_some()
        && config
            .account
            .as_ref()
            .is_none_or(|actual| actual.as_str() == account)
        && !resource.is_empty())
    .then_some(resource)
}

fn endpoint(
    e: &Evidence<'_>,
    target: usize,
    path: &str,
    kind: &str,
    outputs: &[&str],
) -> Option<usize> {
    configured_source(e, target, path, kind, outputs, |source, output| {
        e.text(target, path)
            .is_some_and(|value| e.text(source, output) == Some(value))
    })
}

fn configured_source(
    e: &Evidence<'_>,
    target: usize,
    path: &str,
    kind: &str,
    outputs: &[&str],
    matches: impl FnOnce(usize, &str) -> bool,
) -> Option<usize> {
    let r = super::lambda_core::unique(e.refs, target, path)?;
    let [source] = *r.sources.as_slice() else {
        return None;
    };
    let left = &e.nodes[target];
    let right = &e.nodes[source];
    let attrs = r.source_attributes_by_instance.get(&source)?;
    let output = attrs.first()?;
    let current = |node: &TerraformEntity| {
        node.provider.is_aws()
            && node.mode == crate::model::EntityMode::Managed
            && node.action != crate::model::Action::Delete
            && node.deposed_key.is_none()
    };
    (r.complete
        && r.instances_explicit
        && r.issues.is_empty()
        && source != target
        && current(left)
        && current(right)
        && right.resource_type == kind
        && attrs.len() == 1
        && outputs.contains(&output.as_str())
        && matches(source, output)
        && super::eventbridge::bindings(
            left.provider_configuration.as_ref(),
            right.provider_configuration.as_ref(),
            super::scope::scope(left)
                .as_ref()
                .and_then(|s| s.region.as_ref()),
            super::scope::scope(right)
                .as_ref()
                .and_then(|s| s.region.as_ref()),
            false,
        ))
    .then_some(source)
}

fn same_endpoint(
    e: &Evidence<'_>,
    left: usize,
    right: usize,
    path: &str,
    kind: &str,
    outputs: &[&str],
) -> bool {
    endpoint(e, left, path, kind, outputs)
        .is_some_and(|owner| endpoint(e, right, path, kind, outputs) == Some(owner))
}

fn amplify_backend_in_app(e: &Evidence<'_>, backend: usize, app: usize) -> bool {
    let Some((id, environment)) = e.text(app, "id").zip(e.text(backend, "environment_name")) else {
        return false;
    };
    e.text(backend, "app_id") == Some(id)
        && e.text(backend, "arn")
            .and_then(|arn| arn_resource(&e.nodes[backend], arn, "amplify", true))
            .is_some_and(|resource| {
                resource == format!("apps/{id}/backendenvironments/{environment}")
            })
}

fn qualifies(e: &Evidence<'_>, r: &AttributeReference, source: usize, path: &str) -> bool {
    let target = r.target;
    if e.nodes[source].resource_type == "aws_iam_role" && !iam_role(e, source) {
        return false;
    }
    match (e.nodes[target].resource_type.as_str(), path) {
        (FABRIC_INGESTION, "app_bundle_arn") => {
            service_arn(e, source, "appfabric", "appbundle/").is_some_and(segment)
                && fabric_child_id(e, target, source, "ingestion").is_some()
        }
        (FABRIC_DESTINATION, "ingestion_arn") => {
            let bundle = endpoint(e, target, "app_bundle_arn", FABRIC_BUNDLE, &["arn"]);
            bundle.is_some_and(|bundle| {
                service_arn(e, bundle, "appfabric", "appbundle/").is_some_and(segment)
                    && endpoint(e, source, "app_bundle_arn", FABRIC_BUNDLE, &["arn"])
                        == Some(bundle)
                    && fabric_child_id(e, source, bundle, "ingestion").is_some()
            })
        }
        (FABRIC_DESTINATION, _) if matches!(r.attribute.as_str(), FABRIC_S3 | FABRIC_FIREHOSE) => {
            let other = if r.attribute == FABRIC_S3 {
                "firehose_stream"
            } else {
                "s3_bucket"
            };
            e.absent(
                target,
                &format!("destination_configuration.0.audit_log.0.destination.0.{other}"),
            )
        }
        (FABRIC_AUTH | FABRIC_CONNECTION | FABRIC_DESTINATION, "app_bundle_arn") => {
            service_arn(e, source, "appfabric", "appbundle/").is_some_and(segment)
        }
        (FABRIC_BUNDLE, "customer_managed_key_arn") => {
            service_arn(e, source, "kms", "key/").is_some_and(segment)
        }
        (CONFIG_EXTENSION, _) => extension_action(e, target, source, path),
        (CONFIG_ASSOCIATION, "extension_arn") => {
            service_arn(e, source, "appconfig", "extension/").is_some_and(segment)
        }
        (CONFIG_ASSOCIATION, "resource_arn") => extension_target(e, source),
        (CONFIG_PROFILE, "location_uri") => configuration_source(e, target, source),
        (CONFIG_PROFILE, "retrieval_role_arn") => {
            e.text(target, "location_uri").is_some_and(|uri| {
                uri != "hosted" && configuration_locator("", &Value::String(uri.into())).is_some()
            })
        }
        (CONFIG_PROFILE, "kms_key_identifier") => e.text(target, "location_uri") == Some("hosted"),
        (CONFIG_VERSION | CONFIG_DEPLOYMENT, "configuration_profile_id") => {
            same_endpoint(e, target, source, "application_id", CONFIG_APP, &["id"])
        }
        (CONFIG_DEPLOYMENT, "environment_id") => deployment_context(e, target, source),
        (CONFIG_ENV, _) if r.attribute == "monitor.alarm_arn" => e
            .text(source, "arn")
            .and_then(|arn| arn_resource(&e.nodes[source], arn, "cloudwatch", true))
            .and_then(|resource| resource.strip_prefix("alarm:"))
            .is_some_and(|name| !name.is_empty()),
        (AMPLIFY_DOMAIN, "certificate_settings.0.custom_certificate_arn") => {
            e.text(target, "certificate_settings.0.type") == Some("CUSTOM")
                && super::scope::scope(&e.nodes[source])
                    .and_then(|scope| scope.region)
                    .is_some_and(|region| region.0 == "us-east-1")
                && e.text(source, "arn")
                    .and_then(|arn| arn_resource(&e.nodes[source], arn, "acm", true))
                    .and_then(|resource| resource.strip_prefix("certificate/"))
                    .is_some_and(|id| !id.is_empty())
        }
        (AMPLIFY_DOMAIN | AMPLIFY_WEBHOOK, _) if r.attribute != "app_id" => {
            e.same_endpoint(target, "app_id", source, "app_id", AMPLIFY_APP, "id")
        }
        (AMPLIFY_BACKEND, "app_id") => amplify_backend_in_app(e, target, source),
        (AMPLIFY_BRANCH, "backend_environment_arn") => {
            endpoint(e, target, "app_id", AMPLIFY_APP, &["id"]).is_some_and(|app| {
                same_endpoint(e, target, source, "app_id", AMPLIFY_APP, &["id"])
                    && amplify_backend_in_app(e, source, app)
            })
        }
        _ => true,
    }
}

fn appconfig_profile(e: &Evidence<'_>, consumer: usize) -> Option<usize> {
    endpoint(
        e,
        consumer,
        "configuration_profile_id",
        CONFIG_PROFILE,
        &["configuration_profile_id"],
    )
    .filter(|&profile| same_endpoint(e, consumer, profile, "application_id", CONFIG_APP, &["id"]))
}

fn deployment_context(e: &Evidence<'_>, deployment: usize, environment: usize) -> bool {
    same_endpoint(
        e,
        deployment,
        environment,
        "application_id",
        CONFIG_APP,
        &["id"],
    ) && appconfig_profile(e, deployment).is_some()
}

fn annotate_versions(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    values: &[(&Value, &Value)],
) {
    let e = Evidence {
        nodes,
        refs,
        values,
    };
    let verified: Vec<_> = refs
        .iter()
        .map(|r| {
            if nodes[r.target].resource_type != CONFIG_DEPLOYMENT
                || r.attribute != "configuration_version"
            {
                return None;
            }
            let version = configured_source(
                &e,
                r.target,
                "configuration_version",
                CONFIG_VERSION,
                &["version_number", "version_label"],
                |source, output| {
                    e.text(r.target, "configuration_version")
                        .is_some_and(|actual| match output {
                            "version_number" => e
                                .value(source, output)
                                .and_then(Value::as_u64)
                                .is_some_and(|number| number > 0 && actual == number.to_string()),
                            _ => e.text(source, output) == Some(actual),
                        })
                },
            );
            Some(version.is_some_and(|source| {
                appconfig_profile(&e, r.target).is_some_and(|profile| {
                    appconfig_profile(&e, source) == Some(profile)
                        && e.text(profile, "location_uri") == Some("hosted")
                })
            }))
        })
        .collect();
    for (r, verified) in refs.iter_mut().zip(verified) {
        if let Some(verified) = verified {
            r.service_reference_verified = verified;
        }
    }
}

fn service_arn<'a>(
    e: &'a Evidence<'_>,
    source: usize,
    service: &str,
    prefix: &str,
) -> Option<&'a str> {
    let resource = arn_resource(&e.nodes[source], e.text(source, "arn")?, service, true)?;
    resource.strip_prefix(prefix).filter(|id| !id.is_empty())
}

fn segment(value: &str) -> bool {
    !value.is_empty() && !value.contains(['/', ':'])
}

fn extension_target(e: &Evidence<'_>, source: usize) -> bool {
    let Some(id) = service_arn(e, source, "appconfig", "application/") else {
        return false;
    };
    let parts: Vec<_> = id.split('/').collect();
    match (e.nodes[source].resource_type.as_str(), parts.as_slice()) {
        (CONFIG_APP, [app]) => segment(app),
        (CONFIG_ENV, [app, "environment", env]) => segment(app) && segment(env),
        (CONFIG_PROFILE, [app, "configurationprofile", profile]) => {
            segment(app) && segment(profile)
        }
        _ => false,
    }
}

fn extension_action(e: &Evidence<'_>, target: usize, source: usize, path: &str) -> bool {
    let Some((action, leaf)) = path.rsplit_once('.') else {
        return false;
    };
    let Some((point, _)) = action.rsplit_once(".action.") else {
        return false;
    };
    let point = e.text(target, &format!("{point}.point"));
    let deployment_notification = matches!(
        point,
        Some(
            "ON_DEPLOYMENT_START"
                | "ON_DEPLOYMENT_COMPLETE"
                | "ON_DEPLOYMENT_ROLLED_BACK"
                | "ON_DEPLOYMENT_BAKING"
                | "ON_DEPLOYMENT_STEP"
        )
    );
    if !deployment_notification
        && !matches!(
            point,
            Some(
                "PRE_CREATE_HOSTED_CONFIGURATION_VERSION"
                    | "PRE_START_DEPLOYMENT"
                    | "AT_DEPLOYMENT_TICK"
            )
        )
    {
        return false;
    }
    let Some(uri) = e.text(target, &format!("{action}.uri")) else {
        return false;
    };
    let node = if leaf == "role_arn" {
        &e.nodes[target]
    } else {
        &e.nodes[source]
    };
    let (service, prefix) = match uri.split(':').nth(2) {
        Some("lambda") => ("lambda", "function:"),
        Some("events") if deployment_notification => ("events", "event-bus/default"),
        Some("sns") if deployment_notification => ("sns", ""),
        Some("sqs") if deployment_notification => ("sqs", ""),
        _ => return false,
    };
    if leaf == "uri"
        && !matches!(
            (e.nodes[source].resource_type.as_str(), service),
            ("aws_lambda_function" | "aws_lambda_alias", "lambda")
                | ("aws_sns_topic", "sns")
                | ("aws_sqs_queue", "sqs")
                | ("aws_cloudwatch_event_bus", "events")
        )
    {
        return false;
    }
    arn_resource(node, uri, service, true).is_some_and(|resource| match service {
        "events" => resource == prefix,
        "lambda" => resource
            .strip_prefix(prefix)
            .is_some_and(|name| !name.is_empty()),
        _ => segment(resource),
    })
}

fn fabric_child_id<'a>(
    e: &'a Evidence<'_>,
    child: usize,
    bundle: usize,
    kind: &str,
) -> Option<&'a str> {
    let bundle_arn = e.text(bundle, "arn")?;
    let child_arn = e.text(child, "arn")?;
    let id = child_arn
        .strip_prefix(bundle_arn)?
        .strip_prefix('/')?
        .strip_prefix(kind)?
        .strip_prefix('/')?;
    (segment(id) && arn_resource(&e.nodes[child], child_arn, "appfabric", true).is_some())
        .then_some(id)
}

fn annotate_fabric_connections(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    values: &[(&Value, &Value)],
) {
    let e = Evidence {
        nodes,
        refs,
        values,
    };
    let results: Vec<_> = refs
        .iter()
        .map(|r| {
            if nodes[r.target].resource_type != FABRIC_CONNECTION
                || r.attribute != "app_authorization_arn"
            {
                return None;
            }
            let verified = endpoint(&e, r.target, "app_bundle_arn", FABRIC_BUNDLE, &["arn"])
                .is_some_and(|bundle| {
                    service_arn(&e, bundle, "appfabric", "appbundle/").is_some_and(segment)
                        && configured_source(
                            &e,
                            r.target,
                            "app_authorization_arn",
                            FABRIC_AUTH,
                            &["arn"],
                            |source, _| {
                                let Some((actual, id)) = e
                                    .text(r.target, "app_authorization_arn")
                                    .zip(fabric_child_id(&e, source, bundle, "appauthorization"))
                                else {
                                    return false;
                                };
                                endpoint(&e, source, "app_bundle_arn", FABRIC_BUNDLE, &["arn"])
                                    == Some(bundle)
                                    && (e.text(source, "arn") == Some(actual)
                                        || (uuid(actual) && actual == id))
                            },
                        )
                        .is_some()
                });
            Some(verified)
        })
        .collect();
    for (r, verified) in refs.iter_mut().zip(results) {
        if let Some(verified) = verified {
            r.service_reference_verified = verified;
        }
    }
}

pub(crate) fn fabric_ingestion_authorizations(
    nodes: &[TerraformEntity],
    refs: &[AttributeReference],
    values: &[(&Value, &Value)],
) -> Vec<AttributeReference> {
    let e = Evidence {
        nodes,
        refs,
        values,
    };
    let authorizations: Vec<_> = nodes
        .iter()
        .enumerate()
        .filter_map(|(index, node)| {
            if node.resource_type != FABRIC_AUTH {
                return None;
            }
            let bundle = endpoint(&e, index, "app_bundle_arn", FABRIC_BUNDLE, &["arn"])?;
            fabric_child_id(&e, index, bundle, "appauthorization")?;
            Some((
                index,
                bundle,
                e.text(index, "app")?,
                e.text(index, "tenant.0.tenant_identifier")?,
            ))
        })
        .collect();
    refs.iter()
        .filter_map(|r| {
            let target = r.target;
            if nodes[target].resource_type != FABRIC_INGESTION
                || r.attribute != "app_bundle_arn"
                || !r.service_reference_verified
            {
                return None;
            }
            let [bundle] = *r.sources.as_slice() else {
                return None;
            };
            let app = e.text(target, "app")?;
            let tenant = e.text(target, "tenant_id")?;
            let mut matches = authorizations
                .iter()
                .filter(|(_, b, a, t)| (*b, *a, *t) == (bundle, app, tenant));
            let &(source, _, _, _) = matches.next()?;
            if matches.next().is_some() {
                return None;
            }
            let mut matched = r.clone();
            matched.attribute = "authorization".into();
            matched.has_references = false;
            matched.sources = vec![source];
            matched.source_attributes_by_instance.clear();
            matched.source_attributes.clear();
            Some(matched)
        })
        .collect()
}

fn uuid(value: &str) -> bool {
    value.split('-').map(|part| part.len()).eq([8, 4, 4, 4, 12])
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() || byte == b'-')
}

fn iam_role(e: &Evidence<'_>, source: usize) -> bool {
    e.text(source, "arn")
        .and_then(|arn| arn_resource(&e.nodes[source], arn, "iam", false))
        .and_then(|resource| resource.strip_prefix("role/"))
        .is_some_and(|name| !name.is_empty())
}

fn configuration_source(e: &Evidence<'_>, target: usize, source: usize) -> bool {
    let Some(uri) = e.text(target, "location_uri") else {
        return false;
    };
    let ssm = |scheme: &str, prefix: &str| {
        uri.starts_with(scheme)
            || arn_resource(&e.nodes[source], uri, "ssm", true)
                .and_then(|resource| resource.strip_prefix(prefix))
                .is_some_and(|name| !name.is_empty())
    };
    match e.nodes[source].resource_type.as_str() {
        "aws_s3_object" => {
            s3_object(uri).is_some_and(|(bucket, _)| e.text(source, "bucket") == Some(bucket))
        }
        "aws_ssm_document" => ssm("ssm-document://", "document/"),
        "aws_ssm_parameter" => ssm("ssm-parameter://", "parameter/"),
        _ => false,
    }
}

pub(super) fn owner(kind: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match kind {
        FABRIC_INGESTION => Some(("app_bundle_arn", "AppFabric bundle", "Ingestions")),
        FABRIC_DESTINATION => Some((
            "ingestion_arn",
            "AppFabric ingestion",
            "Ingestion destinations",
        )),
        FABRIC_AUTH => Some(("app_bundle_arn", "AppFabric bundle", "Authorizations")),
        FABRIC_CONNECTION => Some((
            "app_authorization_arn",
            "AppFabric authorization",
            "Authorization connections",
        )),
        CONFIG_DEPLOYMENT => Some((
            "environment_id",
            "AppConfig deployment environment",
            "Deployments",
        )),
        CONFIG_ENV => Some(("application_id", "AppConfig application", "Environments")),
        CONFIG_PROFILE => Some((
            "application_id",
            "AppConfig application",
            "Configuration profiles",
        )),
        CONFIG_VERSION => Some((
            "configuration_profile_id",
            "AppConfig configuration profile",
            "Hosted versions",
        )),
        AMPLIFY_DOMAIN => Some(("app_id", "Amplify app", "Domains")),
        AMPLIFY_WEBHOOK => Some(("app_id", "Amplify app", "Webhooks")),
        AMPLIFY_BRANCH => Some(("app_id", "Amplify app", "Branches")),
        AMPLIFY_BACKEND => Some(("app_id", "Amplify app", "Backend environments")),
        _ => None,
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/semantic/application_services.rs"]
mod tests;
