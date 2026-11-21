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

fn voice_configuration(kind: &str) -> bool {
    matches!(
        kind,
        VOICE_LOGGING | VOICE_ORIGINATION | VOICE_TERMINATION | VOICE_CREDENTIALS | STREAMING
    )
}

pub(crate) fn has_indexed(kind: &str) -> bool {
    matches!(kind, VOICE_GROUP | MEDIA | STREAMING)
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
        return e.nodes[source].region_override.is_some();
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

#[cfg(test)]
#[path = "../../../tests/unit/semantic/communications.rs"]
mod tests;
