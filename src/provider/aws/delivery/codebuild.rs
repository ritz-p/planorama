use super::{Evidence, Rule};
use crate::model::AttributeReference;

pub(super) fn resource(kind: &str) -> bool {
    matches!(
        kind,
        "aws_codebuild_project"
            | "aws_codebuild_fleet"
            | "aws_codebuild_report_group"
            | "aws_codebuild_resource_policy"
            | "aws_codebuild_webhook"
    )
}

pub(super) fn rule(kind: &str, path: &str) -> Option<Rule> {
    Some(match (kind, path) {
        ("aws_codebuild_fleet", "fleet_service_role") => Rule::new(
            "Build fleet role",
            &[("aws_iam_role", &["arn"])],
            false,
            &[],
        ),
        ("aws_codebuild_project", "service_role") => Rule::new(
            "Build project role",
            &[("aws_iam_role", &["arn"])],
            false,
            &[],
        ),
        ("aws_codebuild_project", "encryption_key") => Rule::new(
            "Build encryption key",
            &[("aws_kms_key", &["arn", "id", "key_id"])],
            false,
            &[],
        ),
        ("aws_codebuild_project", "environment.fleet.fleet_arn") => Rule::new(
            "Build fleet",
            &[("aws_codebuild_fleet", &["arn"])],
            false,
            &[],
        ),
        ("aws_codebuild_project" | "aws_codebuild_fleet", "vpc_config.vpc_id") => {
            Rule::new("Build VPC", &[("aws_vpc", &["id"])], false, &[])
        }
        ("aws_codebuild_project" | "aws_codebuild_fleet", "vpc_config.subnets") => {
            Rule::new("Build subnets", &[("aws_subnet", &["id"])], true, &[])
        }
        ("aws_codebuild_project" | "aws_codebuild_fleet", "vpc_config.security_group_ids") => {
            Rule::new(
                "Build security groups",
                &[("aws_security_group", &["id"])],
                true,
                &[],
            )
        }
        ("aws_codebuild_project", "artifacts.location") => Rule::new(
            "Build artifact bucket",
            &[("aws_s3_bucket", &["bucket", "id"])],
            false,
            &[],
        ),
        ("aws_codebuild_project", "secondary_artifacts.location") => Rule::new(
            "Secondary artifact bucket",
            &[("aws_s3_bucket", &["bucket", "id"])],
            false,
            &["secondary_artifacts"],
        ),
        ("aws_codebuild_project", "logs_config.cloudwatch_logs.group_name") => Rule::new(
            "Build log group",
            &[("aws_cloudwatch_log_group", &["name"])],
            false,
            &[],
        ),
        ("aws_codebuild_project", "logs_config.s3_logs.location") => Rule::new(
            "Build logs bucket",
            &[("aws_s3_bucket", &["bucket", "id"])],
            false,
            &[],
        )
        .decoded(log_bucket),
        ("aws_codebuild_report_group", "export_config.s3_destination.bucket") => Rule::new(
            "Report export bucket",
            &[("aws_s3_bucket", &["bucket", "id"])],
            false,
            &[],
        ),
        ("aws_codebuild_report_group", "export_config.s3_destination.encryption_key") => Rule::new(
            "Report encryption key",
            &[("aws_kms_key", &["arn"])],
            false,
            &[],
        ),
        ("aws_codebuild_resource_policy", "resource_arn") => Rule::new(
            "CodeBuild policy resource",
            &[
                ("aws_codebuild_project", &["arn"]),
                ("aws_codebuild_report_group", &["arn"]),
            ],
            false,
            &[],
        ),
        ("aws_codebuild_webhook", "project_name") => Rule::new(
            "Webhook build project",
            &[("aws_codebuild_project", &["name"])],
            false,
            &[],
        ),
        _ => return None,
    })
}

pub(super) fn qualifies(
    e: &Evidence<'_>,
    r: &AttributeReference,
    source: usize,
    path: &str,
) -> bool {
    let consumer = r.target;
    match r.attribute.as_str() {
        "vpc_config.subnets" | "vpc_config.security_group_ids" => {
            let vpc = e.configured_endpoint(consumer, "vpc_config.0.vpc_id", "aws_vpc", &["id"]);
            let parent =
                crate::provider::aws::lambda_core::parent(e.nodes, e.refs, e.values, source);
            vpc.is_some() && vpc == parent
        }
        "artifacts.location" | "secondary_artifacts.location" => {
            let (block, _) = path.rsplit_once('.').unwrap();
            e.text(consumer, &format!("{block}.type")) == Some("S3")
        }
        "logs_config.cloudwatch_logs.group_name" | "logs_config.s3_logs.location" => {
            let (block, _) = path.rsplit_once('.').unwrap();
            e.text(consumer, &format!("{block}.status")) == Some("ENABLED")
        }
        "export_config.s3_destination.bucket" | "export_config.s3_destination.encryption_key" => {
            let s3 = e.text(consumer, "export_config.0.type") == Some("S3");
            s3 && (r.attribute.ends_with(".bucket")
                || e.value(
                    consumer,
                    "export_config.0.s3_destination.0.encryption_disabled",
                )
                .is_some_and(|v| v.is_null() || v.as_bool() == Some(false)))
        }
        _ => true,
    }
}

fn log_bucket<'a>(
    _: &str,
    value: &'a serde_json::Value,
) -> Option<std::collections::BTreeSet<&'a str>> {
    let location = value.as_str()?;
    let bucket = location.split('/').next()?;
    (!bucket.is_empty()).then(|| [bucket].into())
}
