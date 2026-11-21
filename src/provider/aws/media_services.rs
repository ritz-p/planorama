use super::configured_links::{self, Rule};
use crate::model::{AttributeReference, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;

fn rule(kind: &str, path: &str) -> Option<Rule> {
    Some(match (kind, path) {
        ("aws_elastictranscoder_pipeline", "input_bucket") => Rule::new(
            "Transcoder input bucket",
            &[("aws_s3_bucket", &["bucket", "id"])],
            false,
            &[],
        ),
        ("aws_elastictranscoder_pipeline", "output_bucket") => Rule::new(
            "Transcoder output bucket",
            &[("aws_s3_bucket", &["bucket", "id"])],
            false,
            &[],
        ),
        ("aws_elastictranscoder_pipeline", "role") => Rule::new(
            "Transcoder service role",
            &[("aws_iam_role", &["arn"])],
            false,
            &[],
        ),
        ("aws_elastictranscoder_pipeline", "aws_kms_key_arn") => Rule::new(
            "Transcoder encryption key",
            &[("aws_kms_key", &["arn"])],
            false,
            &[],
        ),
        ("aws_gamelift_alias", "routing_strategy.fleet_id") => Rule::new(
            "GameLift fleet",
            &[("aws_gamelift_fleet", &["id"])],
            false,
            &["routing_strategy"],
        ),
        ("aws_gamelift_build", "storage_location.bucket") => Rule::new(
            "Build source bucket",
            &[("aws_s3_bucket", &["bucket", "id"])],
            false,
            &["storage_location"],
        ),
        ("aws_gamelift_build", "storage_location.role_arn") => Rule::new(
            "Build access role",
            &[("aws_iam_role", &["arn"])],
            false,
            &["storage_location"],
        ),
        ("aws_gamelift_fleet", "build_id") => Rule::new(
            "GameLift build",
            &[("aws_gamelift_build", &["id"])],
            false,
            &[],
        ),
        ("aws_gamelift_fleet", "script_id") => Rule::new(
            "GameLift script",
            &[("aws_gamelift_script", &["id"])],
            false,
            &[],
        ),
        ("aws_gamelift_fleet", "instance_role_arn") => Rule::new(
            "Fleet instance role",
            &[("aws_iam_role", &["arn"])],
            false,
            &[],
        ),
        ("aws_gamelift_game_server_group", "role_arn") => Rule::new(
            "FleetIQ service role",
            &[("aws_iam_role", &["arn"])],
            false,
            &[],
        ),
        ("aws_gamelift_game_server_group", "vpc_subnets") => {
            Rule::new("FleetIQ subnets", &[("aws_subnet", &["id"])], true, &[])
        }
        ("aws_gamelift_game_session_queue", "destinations") => Rule::new(
            "Session placement destinations",
            &[
                ("aws_gamelift_fleet", &["arn"]),
                ("aws_gamelift_alias", &["arn"]),
            ],
            true,
            &[],
        ),
        ("aws_gamelift_script", "storage_location.bucket") => Rule::new(
            "Script source bucket",
            &[("aws_s3_bucket", &["bucket", "id"])],
            false,
            &["storage_location"],
        ),
        ("aws_gamelift_script", "storage_location.role_arn") => Rule::new(
            "Script access role",
            &[("aws_iam_role", &["arn"])],
            false,
            &["storage_location"],
        ),
        ("aws_ivs_channel", "recording_configuration_arn") => Rule::new(
            "IVS recording configuration",
            &[("aws_ivs_recording_configuration", &["arn"])],
            false,
            &[],
        ),
        ("aws_ivs_recording_configuration", "destination_configuration.s3.bucket_name") => {
            Rule::new(
                "Recording S3 bucket",
                &[("aws_s3_bucket", &["bucket", "id"])],
                false,
                &["destination_configuration", "destination_configuration.s3"],
            )
        }
        (
            "aws_ivschat_logging_configuration",
            "destination_configuration.cloudwatch_logs.log_group_name",
        ) => Rule::new(
            "Chat log group",
            &[("aws_cloudwatch_log_group", &["name"])],
            false,
            &[
                "destination_configuration",
                "destination_configuration.cloudwatch_logs",
            ],
        ),
        (
            "aws_ivschat_logging_configuration",
            "destination_configuration.firehose.delivery_stream_name",
        ) => Rule::new(
            "Chat delivery stream",
            &[("aws_kinesis_firehose_delivery_stream", &["name"])],
            false,
            &[
                "destination_configuration",
                "destination_configuration.firehose",
            ],
        ),
        ("aws_ivschat_logging_configuration", "destination_configuration.s3.bucket_name") => {
            Rule::new(
                "Chat log bucket",
                &[("aws_s3_bucket", &["bucket", "id"])],
                false,
                &["destination_configuration", "destination_configuration.s3"],
            )
        }
        ("aws_ivschat_room", "logging_configuration_identifiers") => Rule::new(
            "Chat logging configurations",
            &[("aws_ivschat_logging_configuration", &["arn"])],
            true,
            &[],
        ),
        ("aws_location_geofence_collection", "kms_key_id") => Rule::new(
            "Location encryption key",
            &[("aws_kms_key", &["arn", "key_id"])],
            false,
            &[],
        ),
        ("aws_location_tracker", "kms_key_id") => Rule::new(
            "Location encryption key",
            &[("aws_kms_key", &["arn", "key_id"])],
            false,
            &[],
        ),
        ("aws_location_tracker_association", "consumer_arn") => Rule::new(
            "Geofence collection",
            &[("aws_location_geofence_collection", &["collection_arn"])],
            false,
            &[],
        ),
        ("aws_location_tracker_association", "tracker_name") => Rule::new(
            "Location tracker",
            &[("aws_location_tracker", &["tracker_name"])],
            false,
            &[],
        ),
        _ => return None,
    })
}

pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    rule(kind, path).map(|r| r.label)
}
pub(crate) fn indexed(kind: &str, path: &str) -> bool {
    configured_links::indexed(kind, path, rule)
}
pub(super) fn bindings(raw: &Input, r: &AttributeReference) -> bool {
    configured_links::bindings(raw, r, rule)
}
pub(super) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    values: &[(&Value, &Value)],
) {
    configured_links::annotate(nodes, refs, values, rule, network_matches)
}

fn network_matches(
    evidence: &configured_links::Evidence<'_>,
    reference: &AttributeReference,
    endpoint: usize,
    _: &str,
) -> bool {
    if reference.attribute != "vpc_subnets" || reference.sources.len() < 2 {
        return true;
    }
    evidence.text(endpoint, "vpc_id").is_some_and(|vpc| {
        reference
            .sources
            .iter()
            .all(|&source| evidence.text(source, "vpc_id") == Some(vpc))
    })
}
