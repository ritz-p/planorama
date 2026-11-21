use super::configured_links::{self, Evidence, Rule};
use crate::model::{AttributeReference, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;

const SLACK: &str = "aws_chatbot_slack_channel_configuration";
const TEAMS: &str = "aws_chatbot_teams_channel_configuration";

const VOICE_CONNECTOR: &str = "aws_chime_voice_connector";
const VOICE_GROUP: &str = "aws_chime_voice_connector_group";
const VOICE_LOGGING: &str = "aws_chime_voice_connector_logging";
const VOICE_ORIGINATION: &str = "aws_chime_voice_connector_origination";
const VOICE_TERMINATION: &str = "aws_chime_voice_connector_termination";
const VOICE_CREDENTIALS: &str = "aws_chime_voice_connector_termination_credentials";

const STREAMING: &str = "aws_chime_voice_connector_streaming";
const MEDIA: &str = "aws_chimesdkmediapipelines_media_insights_pipeline_configuration";
const INSIGHTS: &str = "media_insights_configuration.configuration_arn";
const ANALYTICS_KEY: &str = "elements.amazon_transcribe_call_analytics_processor_configuration.post_call_analytics_settings.output_encryption_kms_key_id";

const VOICE_GLOBAL: &str = "aws_chimesdkvoice_global_settings";
const PROFILE_DOMAIN: &str = "aws_chimesdkvoice_voice_profile_domain";
const PROFILE_KEY: &str = "server_side_encryption_configuration.kms_key_arn";
const CDR: &str = "voice_connector.cdr_bucket";
const BUCKET: Outputs = &[("aws_s3_bucket", &["bucket", "id"])];

const SIP_APP: &str = "aws_chimesdkvoice_sip_media_application";
const SIP_RULE: &str = "aws_chimesdkvoice_sip_rule";

const CONNECT: &str = "aws_connect_instance";
const BOT: &str = "aws_connect_bot_association";
const FLOW: &str = "aws_connect_contact_flow";
const MODULE: &str = "aws_connect_contact_flow_module";
const HOURS: &str = "aws_connect_hours_of_operation";
const VOCABULARY: &str = "aws_connect_vocabulary";
const INSTANCE_ID: Outputs = &[(CONNECT, &["id"])];

fn instance_child(kind: &str) -> bool {
    matches!(
        kind,
        BOT | FLOW
            | MODULE
            | HOURS
            | VOCABULARY
            | STORAGE
            | CONNECT_LAMBDA
            | QUEUE
            | QUICK
            | ROUTING
            | PHONE_FLOW
            | USER
            | SECURITY_PROFILE
            | HIERARCHY
            | STRUCTURE
    )
}

const STORAGE: &str = "aws_connect_instance_storage_config";
const CONNECT_LAMBDA: &str = "aws_connect_lambda_function_association";
const STORAGE_FIREHOSE: &str = "storage_config.kinesis_firehose_config.firehose_arn";
const STORAGE_KINESIS: &str = "storage_config.kinesis_stream_config.stream_arn";
const STORAGE_BUCKET: &str = "storage_config.s3_config.bucket_name";
const STORAGE_S3_KEY: &str = "storage_config.s3_config.encryption_config.key_id";
const STORAGE_VIDEO_KEY: &str =
    "storage_config.kinesis_video_stream_config.encryption_config.key_id";

const QUEUE: &str = "aws_connect_queue";
const QUICK: &str = "aws_connect_quick_connect";
const ROUTING: &str = "aws_connect_routing_profile";
const USER: &str = "aws_connect_user";
const PHONE: &str = "aws_connect_phone_number";
const QUEUE_ID: Outputs = &[(QUEUE, &["queue_id"])];
const FLOW_ID: Outputs = &[(FLOW, &["contact_flow_id"])];
const PHONE_ID: Outputs = &[(PHONE, &["id"])];

const PHONE_FLOW: &str = "aws_connect_phone_number_contact_flow_association";

const SECURITY_PROFILE: &str = "aws_connect_security_profile";
const HIERARCHY: &str = "aws_connect_user_hierarchy_group";
const STRUCTURE: &str = "aws_connect_user_hierarchy_structure";
const HIERARCHY_ID: Outputs = &[(HIERARCHY, &["hierarchy_group_id"])];

const CUSTOMER_DOMAIN: &str = "aws_customerprofiles_domain";
const CUSTOMER_PROFILE: &str = "aws_customerprofiles_profile";
const MATCH_BUCKET: &str = "matching.exporting_config.s3_exporting.s3_bucket_name";
const RULE_BUCKET: &str = "rule_based_matching.exporting_config.s3_exporting.s3_bucket_name";

fn voice_configuration(kind: &str) -> bool {
    matches!(
        kind,
        VOICE_LOGGING | VOICE_ORIGINATION | VOICE_TERMINATION | VOICE_CREDENTIALS | STREAMING
    )
}

pub(crate) fn has_indexed(kind: &str) -> bool {
    matches!(
        kind,
        VOICE_GROUP
            | MEDIA
            | STREAMING
            | VOICE_GLOBAL
            | PROFILE_DOMAIN
            | SIP_APP
            | SIP_RULE
            | STORAGE
            | QUEUE
            | QUICK
            | ROUTING
            | CUSTOMER_DOMAIN
    )
}

pub(crate) fn indexed(kind: &str, path: &str) -> bool {
    has_indexed(kind) && configured_links::indexed(kind, path, rule)
}

fn rule(kind: &str, path: &str) -> Option<Rule> {
    Some(match (kind, path) {
        (SLACK | TEAMS, "iam_role_arn") => Rule::new(
            "Chatbot channel role",
            &[("aws_iam_role", &["arn"])],
            false,
            &[],
        )
        .in_control_plane_account(),
        (SLACK | TEAMS, "guardrail_policy_arns") => Rule::new(
            "Chatbot guardrail policies",
            &[("aws_iam_policy", &["arn"])],
            true,
            &[],
        )
        .in_control_plane_account(),
        (SLACK | TEAMS, "sns_topic_arns") => Rule::new(
            "Chatbot notification topics",
            &[("aws_sns_topic", &["arn"])],
            true,
            &[],
        )
        .in_control_plane_account(),
        (kind, "voice_connector_id") if voice_configuration(kind) => Rule::new(
            "Chime voice connector",
            &[(VOICE_CONNECTOR, &["id"])],
            false,
            &[],
        )
        .in_control_plane_account(),
        (VOICE_GROUP, "connector.voice_connector_id") => Rule::scalar_set(
            "Chime voice connector members",
            &[(VOICE_CONNECTOR, &["id"])],
        )
        .in_control_plane_account(),
        (STREAMING, INSIGHTS) => scalar("Chime Media Insights configuration", &[(MEDIA, &["arn"])])
            .in_control_plane_account(),
        (MEDIA, "resource_access_role_arn") => {
            scalar("Media Insights access role", &[("aws_iam_role", &["arn"])])
        }
        (MEDIA, "elements.kinesis_data_stream_sink_configuration.insights_target") => element(
            "Media Insights Kinesis sink",
            &[("aws_kinesis_stream", &["arn"])],
        ),
        (MEDIA, "elements.lambda_function_sink_configuration.insights_target") => {
            element("Media Insights Lambda sink", LAMBDA)
        }
        (MEDIA, "elements.sns_topic_sink_configuration.insights_target") => {
            element("Media Insights SNS sink", &[("aws_sns_topic", &["arn"])])
        }
        (MEDIA, "elements.sqs_queue_sink_configuration.insights_target") => {
            element("Media Insights SQS sink", &[("aws_sqs_queue", &["arn"])])
        }
        (MEDIA, "elements.s3_recording_sink_configuration.destination") => element(
            "Media Insights recording bucket",
            &[("aws_s3_bucket", &["arn", "bucket", "id"])],
        )
        .decoded(recording_bucket),
        (MEDIA, ANALYTICS_KEY) => element("Media Insights analytics output key", KMS),
        (VOICE_GLOBAL, CDR) => {
            scalar("Chime call detail records bucket", BUCKET).in_control_plane_account()
        }
        (PROFILE_DOMAIN, PROFILE_KEY) => scalar(
            "Chime voice profile encryption key",
            &[("aws_kms_key", &["arn"])],
        ),
        (SIP_APP, "endpoints.lambda_arn") => {
            scalar("Chime SIP Lambda endpoint", LAMBDA).in_control_plane_account()
        }
        (SIP_RULE, "trigger_value") => scalar(
            "Chime SIP outbound host",
            &[(VOICE_CONNECTOR, &["outbound_host_name"])],
        )
        .in_control_plane_account(),
        (SIP_RULE, "target_applications.sip_media_application_id") => {
            Rule::scalar_set("Chime SIP target applications", &[(SIP_APP, &["id"])])
                .in_control_plane_account()
        }
        (kind, "instance_id") if instance_child(kind) => scalar("Connect instance", INSTANCE_ID),
        (CONNECT, "directory_id") => scalar(
            "Connect identity directory",
            &[("aws_directory_service_directory", &["id"])],
        ),
        (CONNECT_LAMBDA, "function_arn") => scalar("Connect associated Lambda", LAMBDA),
        (STORAGE, STORAGE_FIREHOSE) => scalar(
            "Connect storage delivery stream",
            &[("aws_kinesis_firehose_delivery_stream", &["arn"])],
        ),
        (STORAGE, STORAGE_KINESIS) => scalar(
            "Connect storage data stream",
            &[("aws_kinesis_stream", &["arn"])],
        ),
        (STORAGE, STORAGE_BUCKET) => scalar("Connect storage bucket", BUCKET),
        (STORAGE, STORAGE_S3_KEY | STORAGE_VIDEO_KEY) => {
            scalar("Connect storage encryption key", KMS)
        }
        (QUEUE, "hours_of_operation_id") => scalar(
            "Connect queue schedule",
            &[(HOURS, &["hours_of_operation_id"])],
        ),
        (QUEUE, "quick_connect_ids") => Rule::new(
            "Connect queue quick connects",
            &[(QUICK, &["quick_connect_id"])],
            true,
            &[],
        ),
        (QUEUE, "outbound_caller_config.outbound_caller_id_number_id") => {
            scalar("Connect outbound number", PHONE_ID)
        }
        (QUEUE, "outbound_caller_config.outbound_flow_id") => {
            scalar("Connect outbound flow", FLOW_ID)
        }
        (QUICK, "quick_connect_config.queue_config.queue_id") => {
            scalar("Connect quick connect queue", QUEUE_ID)
        }
        (QUICK, "quick_connect_config.user_config.user_id") => {
            scalar("Connect quick connect user", &[(USER, &["user_id"])])
        }
        (
            QUICK,
            "quick_connect_config.queue_config.contact_flow_id"
            | "quick_connect_config.user_config.contact_flow_id",
        ) => scalar("Connect quick connect flow", FLOW_ID),
        (ROUTING, "default_outbound_queue_id") => {
            scalar("Connect default outbound queue", QUEUE_ID)
        }
        (ROUTING, "queue_configs.queue_id") => Rule::scalar_set("Connect routing queues", QUEUE_ID),
        (PHONE, "target_arn") => {
            scalar("Connect phone number claim target", &[(CONNECT, &["arn"])])
        }
        (PHONE_FLOW, "phone_number_id") => scalar("Connect associated phone number", PHONE_ID),
        (PHONE_FLOW, "contact_flow_id") => scalar("Connect associated contact flow", FLOW_ID),
        (USER, "routing_profile_id") => scalar(
            "Connect user routing profile",
            &[(ROUTING, &["routing_profile_id"])],
        ),
        (USER, "hierarchy_group_id") => scalar("Connect user hierarchy group", HIERARCHY_ID),
        (USER, "security_profile_ids") => Rule::new(
            "Connect user security profiles",
            &[(SECURITY_PROFILE, &["security_profile_id"])],
            true,
            &[],
        ),
        (HIERARCHY, "parent_group_id") => scalar("Connect parent hierarchy group", HIERARCHY_ID),
        (CUSTOMER_DOMAIN, "default_encryption_key") => {
            scalar("Customer Profiles encryption key", KMS)
        }
        (CUSTOMER_DOMAIN, "dead_letter_queue_url") => scalar(
            "Customer Profiles dead-letter queue",
            &[("aws_sqs_queue", &["url", "id"])],
        ),
        (CUSTOMER_DOMAIN, MATCH_BUCKET | RULE_BUCKET) => {
            scalar("Customer Profiles export bucket", BUCKET)
        }
        (CUSTOMER_PROFILE, "domain_name") => scalar(
            "Customer Profiles domain",
            &[(CUSTOMER_DOMAIN, &["domain_name", "id"])],
        ),
        _ => return None,
    })
}

pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    rule(kind, path).map(|r| r.label)
}

pub(super) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    values: &[(&Value, &Value)],
) {
    configured_links::annotate(nodes, refs, values, rule, qualifies);
}

pub(super) fn bindings(raw: &Input, r: &AttributeReference) -> bool {
    configured_links::bindings(raw, r, rule)
}

fn arn<'a>(
    node: &TerraformEntity,
    value: &'a str,
    service: &str,
    prefix: &str,
) -> Option<(&'a str, &'a str)> {
    let mut fields = value.splitn(6, ':');
    if fields.next()? != "arn" || !fields.next()?.starts_with("aws") || fields.next()? != service {
        return None;
    }
    let region = fields.next()?;
    let account = fields.next()?;
    let identity = fields.next()?.strip_prefix(prefix)?;
    let binding = node.provider_configuration.as_ref()?;
    (!identity.is_empty()
        && !binding.account_conflict
        && crate::model::AwsAccountScope::new(account).is_some()
        && binding
            .account
            .as_ref()
            .is_none_or(|a| a.as_str() == account))
    .then_some((region, identity))
}

fn qualified_arn(node: &TerraformEntity, value: &str, service: &str, prefix: &str) -> bool {
    arn(node, value, service, prefix).is_some_and(|(region, _)| {
        let scope = super::scope::scope(node);
        if service == "iam" {
            region.is_empty()
        } else {
            scope.and_then(|s| s.region).is_some_and(|s| s.0 == region)
        }
    })
}

fn qualifies(e: &Evidence<'_>, r: &AttributeReference, source: usize, path: &str) -> bool {
    if e.nodes[source].resource_type == VOICE_CONNECTOR {
        return e.nodes[source].region_override.is_some()
            && (r.attribute != "trigger_value"
                || (e.text(r.target, "trigger_type") == Some("RequestUriHostname")
                    && e.text(r.target, "trigger_value").is_some_and(|host| {
                        !host.contains(['/', ':', '@']) && host.contains('.')
                    })));
    }
    let target = r.target;
    if r.attribute == INSIGHTS {
        return ["disabled", "media_insights_configuration.0.disabled"]
            .iter()
            .all(|path| {
                matches!(
                    e.value(target, path),
                    Some(Value::Bool(false) | Value::Null)
                )
            })
            && endpoint(e, target, "voice_connector_id", VOICE_CONNECTOR, "id").is_some_and(
                |connector| {
                    same_region(&e.nodes[connector], &e.nodes[source])
                        && e.text(source, "arn").is_some_and(|value| {
                            qualified_arn(
                                &e.nodes[source],
                                value,
                                "chime",
                                "media-insights-pipeline-configuration/",
                            )
                        })
                },
            );
    }
    if e.nodes[target].resource_type == MEDIA {
        if r.attribute == "resource_access_role_arn" {
            return e
                .text(source, "arn")
                .is_some_and(|value| qualified_arn(&e.nodes[source], value, "iam", "role/"));
        }
        return media_element(e, r, source, path);
    }
    if r.attribute == CDR {
        return e
            .text(target, path)
            .is_some_and(|name| !name.contains([':', '/']));
    }
    if r.attribute == PROFILE_KEY {
        return input_arn(e, target, source, path, "kms", "key/");
    }
    if r.attribute == "endpoints.lambda_arn" {
        return same_region(&e.nodes[target], &e.nodes[source])
            && input_arn(e, target, source, path, "lambda", "function:");
    }
    if r.attribute == "target_applications.sip_media_application_id" {
        let Some((block, _)) = path.rsplit_once('.') else {
            return false;
        };
        return e
            .text(target, &format!("{block}.aws_region"))
            .is_some_and(|wanted| {
                e.nodes[source]
                    .region_override
                    .as_ref()
                    .is_some_and(|region| region.0 == wanted)
            });
    }
    if e.nodes[target].resource_type == PHONE_FLOW {
        let number = endpoint(e, target, "phone_number_id", PHONE, "id");
        let flow = endpoint(e, target, "contact_flow_id", FLOW, "contact_flow_id");
        return number.zip(flow).is_some_and(|(number, flow)| {
            same_instance(e, target, number) && same_instance(e, target, flow)
        });
    }
    if r.attribute == "instance_id" && instance_child(&e.nodes[target].resource_type) {
        return true;
    }
    if e.nodes[target].resource_type == CONNECT && r.attribute == "directory_id" {
        return e.text(target, "identity_management_type") == Some("EXISTING_DIRECTORY");
    }
    if r.attribute == "function_arn" && e.nodes[target].resource_type == CONNECT_LAMBDA {
        return input_arn(e, target, source, path, "lambda", "function:");
    }
    if e.nodes[target].resource_type == STORAGE {
        return storage_destination(e, r, source, path);
    }
    if matches!(
        e.nodes[target].resource_type.as_str(),
        QUEUE | QUICK | ROUTING
    ) {
        return same_instance(e, target, source)
            && (e.nodes[target].resource_type != QUICK || quick_connect_shape(e, target, path));
    }
    if e.nodes[target].resource_type == PHONE && r.attribute == "target_arn" {
        return connect_instance_arn(e, source);
    }
    if e.nodes[target].resource_type == USER {
        return same_instance(e, target, source);
    }
    if e.nodes[target].resource_type == HIERARCHY && r.attribute == "parent_group_id" {
        return hierarchy_parent(e, target, source);
    }
    if e.nodes[target].resource_type == CUSTOMER_DOMAIN {
        return match r.attribute.as_str() {
            "default_encryption_key" => key_input(e, target, source, path),
            "dead_letter_queue_url" => queue_url(e, target, source),
            MATCH_BUCKET => e.value(target, "matching.0.enabled") == Some(&Value::Bool(true)),
            RULE_BUCKET => {
                e.value(target, "rule_based_matching.0.enabled") == Some(&Value::Bool(true))
            }
            _ => false,
        };
    }
    if e.nodes[target].resource_type == CUSTOMER_PROFILE {
        return r.attribute == "domain_name";
    }
    let (service, prefix) = match r.attribute.as_str() {
        "iam_role_arn" => ("iam", "role/"),
        "guardrail_policy_arns" => ("iam", "policy/"),
        "sns_topic_arns" => ("sns", ""),
        _ => return false,
    };
    e.text(source, "arn")
        .is_some_and(|value| qualified_arn(&e.nodes[source], value, service, prefix))
}

fn current(node: &TerraformEntity) -> bool {
    node.provider.is_aws()
        && node.mode == crate::model::EntityMode::Managed
        && node.action != crate::model::Action::Delete
        && node.deposed_key.is_none()
}

pub(crate) fn service_regions(nodes: &mut [TerraformEntity], values: &[(&Value, &Value)]) {
    for (index, node) in nodes.iter_mut().enumerate() {
        let prefix = match node.resource_type.as_str() {
            VOICE_CONNECTOR => "vc/",
            SIP_APP => "sma/",
            _ => continue,
        };
        if !current(node) {
            continue;
        }
        let read = |field| super::service_evidence::known(values, index, field);
        let selected = read("aws_region").and_then(super::scope_selections::region);
        let observed = read("arn")
            .and_then(|value| arn(node, value, "chime", prefix))
            .filter(|&(_, id)| read("id") == Some(id))
            .and_then(|(region, _)| super::scope_selections::region(region));
        let selected = match (selected, observed, read("arn")) {
            (Some(a), Some(b), _) if a != b => None,
            (_, None, Some(_)) => None,
            (a, b, _) => a.or(b),
        };
        node.has_region_override = true;
        node.region_override = selected.map(|r| crate::model::RegionScope(r.to_owned()));
    }
}

pub(crate) fn inherit_voice_region(nodes: &mut [TerraformEntity], refs: &[AttributeReference]) {
    for r in refs {
        if voice_configuration(&nodes[r.target].resource_type)
            && r.attribute == "voice_connector_id"
            && r.service_reference_verified
        {
            if let [source] = r.sources.as_slice() {
                nodes[r.target].has_region_override = true;
                nodes[r.target].region_override = nodes[*source].region_override.clone();
            }
        }
    }
}

pub(super) fn owner(kind: &str) -> Option<(&'static str, &'static str, &'static str)> {
    if kind == CUSTOMER_PROFILE {
        return Some(("domain_name", "Customer Profiles domain", "Profiles"));
    }
    if kind == HIERARCHY {
        return Some((
            "parent_group_id",
            "Connect parent hierarchy group",
            "Child hierarchy groups",
        ));
    }
    if instance_child(kind) {
        return Some(("instance_id", "Connect instance", "Instance configuration"));
    }
    voice_configuration(kind).then_some((
        "voice_connector_id",
        "Chime voice connector",
        "Connector settings",
    ))
}

type Outputs = &'static [(&'static str, &'static [&'static str])];
const LAMBDA: Outputs = &[
    ("aws_lambda_function", &["arn", "qualified_arn"]),
    ("aws_lambda_alias", &["arn"]),
];
const KMS: Outputs = &[
    ("aws_kms_key", &["arn", "key_id", "id"]),
    ("aws_kms_alias", &["arn", "name"]),
];

fn scalar(label: &'static str, fields: Outputs) -> Rule {
    Rule::new(label, fields, false, &[])
}

fn element(label: &'static str, fields: Outputs) -> Rule {
    Rule::new(label, fields, false, &["elements"])
}

fn recording_bucket<'a>(_: &str, value: &'a Value) -> Option<std::collections::BTreeSet<&'a str>> {
    let value = value.as_str()?;
    if let Some(resource) = value.strip_prefix("s3://") {
        let bucket = resource.split('/').next()?;
        return (!bucket.is_empty()).then(|| [bucket].into());
    }
    let mut arn = value.splitn(6, ':');
    if arn.next()? != "arn"
        || !arn.next()?.starts_with("aws")
        || arn.next()? != "s3"
        || !arn.next()?.is_empty()
        || !arn.next()?.is_empty()
    {
        return None;
    }
    Some([value].into())
}

fn same_region(left: &TerraformEntity, right: &TerraformEntity) -> bool {
    super::scope::scope(left)
        .and_then(|s| s.region)
        .is_some_and(|region| super::scope::scope(right).and_then(|s| s.region) == Some(region))
}

fn endpoint(e: &Evidence<'_>, node: usize, path: &str, kind: &str, output: &str) -> Option<usize> {
    let r = super::lambda_core::unique(e.refs, node, path)?;
    let [source] = *r.sources.as_slice() else {
        return None;
    };
    let attributes = r.source_attributes_by_instance.get(&source)?;
    let left = &e.nodes[node];
    let right = &e.nodes[source];
    (current(left)
        && current(right)
        && right.resource_type == kind
        && source != node
        && r.complete
        && r.instances_explicit
        && r.issues.is_empty()
        && attributes.len() == 1
        && attributes.contains(output)
        && configured_links::same_account(
            left.provider_configuration.as_ref(),
            right.provider_configuration.as_ref(),
        )
        && e.text(node, path)
            .is_some_and(|value| e.text(source, output) == Some(value)))
    .then_some(source)
}

fn media_element(e: &Evidence<'_>, r: &AttributeReference, source: usize, path: &str) -> bool {
    let mut parts = path.split('.');
    let Some(index) = parts.nth(1) else {
        return false;
    };
    let expected = match parts.next() {
        Some("kinesis_data_stream_sink_configuration") => "KinesisDataStreamSink",
        Some("lambda_function_sink_configuration") => "LambdaFunctionSink",
        Some("sns_topic_sink_configuration") => "SnsTopicSink",
        Some("sqs_queue_sink_configuration") => "SqsQueueSink",
        Some("s3_recording_sink_configuration") => "S3RecordingSink",
        Some("amazon_transcribe_call_analytics_processor_configuration") => {
            "AmazonTranscribeCallAnalyticsProcessor"
        }
        _ => return false,
    };
    e.text(r.target, &format!("elements.{index}.type")) == Some(expected)
        && match expected {
            "KinesisDataStreamSink" => input_arn(e, r.target, source, path, "kinesis", "stream/"),
            "LambdaFunctionSink" => input_arn(e, r.target, source, path, "lambda", "function:"),
            "SnsTopicSink" => input_arn(e, r.target, source, path, "sns", ""),
            "SqsQueueSink" => input_arn(e, r.target, source, path, "sqs", ""),
            "AmazonTranscribeCallAnalyticsProcessor" => key_input(e, r.target, source, path),
            _ => true,
        }
}

fn input_arn(
    e: &Evidence<'_>,
    target: usize,
    source: usize,
    path: &str,
    service: &str,
    prefix: &str,
) -> bool {
    e.text(target, path)
        .is_some_and(|value| qualified_arn(&e.nodes[source], value, service, prefix))
}

fn key_input(e: &Evidence<'_>, target: usize, source: usize, path: &str) -> bool {
    let alias = e.nodes[source].resource_type == "aws_kms_alias";
    e.text(target, path).is_some_and(|value| {
        if value.starts_with("arn:") {
            qualified_arn(
                &e.nodes[source],
                value,
                "kms",
                if alias { "alias/" } else { "key/" },
            )
        } else if alias {
            value
                .strip_prefix("alias/")
                .is_some_and(|name| !name.is_empty())
        } else {
            !value.contains([':', '/'])
        }
    })
}

fn storage_destination(
    e: &Evidence<'_>,
    r: &AttributeReference,
    source: usize,
    path: &str,
) -> bool {
    let target = r.target;
    let (selector, selected) = match r.attribute.as_str() {
        STORAGE_FIREHOSE => ("KINESIS_FIREHOSE", "kinesis_firehose_config"),
        STORAGE_KINESIS => ("KINESIS_STREAM", "kinesis_stream_config"),
        STORAGE_BUCKET | STORAGE_S3_KEY => ("S3", "s3_config"),
        STORAGE_VIDEO_KEY => ("KINESIS_VIDEO_STREAM", "kinesis_video_stream_config"),
        _ => return false,
    };
    e.text(target, "storage_config.0.storage_type") == Some(selector)
        && [
            "kinesis_firehose_config",
            "kinesis_stream_config",
            "s3_config",
            "kinesis_video_stream_config",
        ]
        .iter()
        .filter(|&&name| name != selected)
        .all(|name| e.absent(target, &format!("storage_config.0.{name}")))
        && match r.attribute.as_str() {
            STORAGE_FIREHOSE => input_arn(e, target, source, path, "firehose", "deliverystream/"),
            STORAGE_KINESIS => input_arn(e, target, source, path, "kinesis", "stream/"),
            STORAGE_BUCKET => true,
            STORAGE_VIDEO_KEY | STORAGE_S3_KEY => {
                e.text(
                    target,
                    &format!("storage_config.0.{selected}.0.encryption_config.0.encryption_type"),
                ) == Some("KMS")
                    && key_input(e, target, source, path)
            }
            _ => false,
        }
}

fn instance(e: &Evidence<'_>, node: usize) -> Option<usize> {
    let owner = if e.nodes[node].resource_type == PHONE {
        let owner = endpoint(e, node, "target_arn", CONNECT, "arn")?;
        connect_instance_arn(e, owner).then_some(owner)?
    } else {
        endpoint(e, node, "instance_id", CONNECT, "id")?
    };
    same_region(&e.nodes[node], &e.nodes[owner]).then_some(owner)
}

fn same_instance(e: &Evidence<'_>, a: usize, b: usize) -> bool {
    instance(e, a).is_some_and(|id| instance(e, b) == Some(id))
}

fn connect_instance_arn(e: &Evidence<'_>, source: usize) -> bool {
    e.text(source, "arn").is_some_and(|value| {
        qualified_arn(&e.nodes[source], value, "connect", "instance/")
            && arn(&e.nodes[source], value, "connect", "instance/")
                .is_some_and(|(_, id)| e.text(source, "id") == Some(id))
    })
}

fn quick_connect_shape(e: &Evidence<'_>, node: usize, path: &str) -> bool {
    let (selector, other) = if path.starts_with("quick_connect_config.0.queue_config.") {
        ("QUEUE", "user_config")
    } else {
        ("USER", "queue_config")
    };
    e.text(node, "quick_connect_config.0.quick_connect_type") == Some(selector)
        && e.absent(node, &format!("quick_connect_config.0.{other}"))
        && e.absent(node, "quick_connect_config.0.phone_config")
}

fn hierarchy_parent(e: &Evidence<'_>, child: usize, parent: usize) -> bool {
    let mut seen = std::collections::BTreeSet::from([child]);
    let mut parent = parent;
    while seen.insert(parent) && same_instance(e, child, parent) {
        if e.absent(parent, "parent_group_id") {
            return true;
        }
        let Some(next) = endpoint(
            e,
            parent,
            "parent_group_id",
            HIERARCHY,
            "hierarchy_group_id",
        ) else {
            return false;
        };
        parent = next;
    }
    false
}

fn queue_url(e: &Evidence<'_>, target: usize, source: usize) -> bool {
    let Some(url) = e
        .text(target, "dead_letter_queue_url")
        .and_then(|url| url.strip_prefix("https://"))
    else {
        return false;
    };
    let parts: Vec<_> = url.split('/').collect();
    let [host, account, queue] = parts.as_slice() else {
        return false;
    };
    let scope = super::scope::scope(&e.nodes[source]);
    scope.and_then(|s| s.region).is_some_and(|region| {
        let region = region.0;
        *host == format!("sqs.{region}.amazonaws.com")
            || *host == format!("sqs.{region}.amazonaws.com.cn")
            || *host == format!("sqs.{region}.api.aws")
            || *host == format!("{region}.queue.amazonaws.com")
            || (region == "us-east-1" && *host == "queue.amazonaws.com")
    }) && crate::model::AwsAccountScope::new(account).is_some()
        && e.nodes[source]
            .provider_configuration
            .as_ref()
            .and_then(|c| c.account.as_ref())
            .is_none_or(|a| a.as_str() == *account)
        && !queue.is_empty()
        && !queue.contains(['?', '#', ':'])
}

#[cfg(test)]
#[path = "../../../tests/unit/semantic/communications.rs"]
mod tests;
