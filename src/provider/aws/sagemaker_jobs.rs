use super::configured_links::{self, Evidence, Rule};
use crate::model::{AttributeReference, Graph, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;

pub(crate) fn resource(kind: &str) -> bool {
    matches!(
        kind,
        "aws_sagemaker_algorithm"
            | "aws_sagemaker_data_quality_job_definition"
            | "aws_sagemaker_hyper_parameter_tuning_job"
            | "aws_sagemaker_monitoring_schedule"
            | "aws_sagemaker_pipeline"
            | "aws_sagemaker_training_job"
    )
}
fn rule(kind: &str, path: &str) -> Option<Rule> {
    Some(match (kind, path) {
        ("aws_sagemaker_algorithm", "validation_specification.validation_role") => Rule::new(
            "Validation role",
            &[("aws_iam_role", &["arn"])],
            false,
            &["validation_specification"],
        ),
        ("aws_sagemaker_data_quality_job_definition", "role_arn") => {
            Rule::new("Monitoring role", &[("aws_iam_role", &["arn"])], false, &[])
        }
        (
            "aws_sagemaker_data_quality_job_definition",
            "data_quality_job_input.endpoint_input.endpoint_name",
        ) => Rule::new(
            "Monitored endpoint",
            &[("aws_sagemaker_endpoint", &["name"])],
            false,
            &[],
        ),
        (
            "aws_sagemaker_hyper_parameter_tuning_job",
            "warm_start_config.parent_hyper_parameter_tuning_jobs.name",
        ) => Rule::new(
            "Warm-start tuning jobs",
            &[("aws_sagemaker_hyper_parameter_tuning_job", &["name"])],
            false,
            &[
                "warm_start_config",
                "warm_start_config.parent_hyper_parameter_tuning_jobs",
            ],
        ),
        ("aws_sagemaker_hyper_parameter_tuning_job", "training_job_definition.role_arn") => {
            Rule::new(
                "Template execution role",
                &[("aws_iam_role", &["arn"])],
                false,
                &["training_job_definition"],
            )
        }
        (
            "aws_sagemaker_hyper_parameter_tuning_job",
            "training_job_definition.algorithm_specification.algorithm_name",
        ) => Rule::new(
            "Training algorithm",
            &[("aws_sagemaker_algorithm", &["algorithm_name"])],
            false,
            &[
                "training_job_definition",
                "training_job_definition.algorithm_specification",
            ],
        ),
        ("aws_sagemaker_hyper_parameter_tuning_job", "training_job_definitions.role_arn") => {
            Rule::new(
                "Template execution role",
                &[("aws_iam_role", &["arn"])],
                false,
                &["training_job_definitions"],
            )
        }
        (
            "aws_sagemaker_hyper_parameter_tuning_job",
            "training_job_definitions.algorithm_specification.algorithm_name",
        ) => Rule::new(
            "Training algorithm",
            &[("aws_sagemaker_algorithm", &["algorithm_name"])],
            false,
            &[
                "training_job_definitions",
                "training_job_definitions.algorithm_specification",
            ],
        ),
        (
            "aws_sagemaker_monitoring_schedule",
            "monitoring_schedule_config.monitoring_job_definition_name",
        ) => Rule::new(
            "Data quality job definition",
            &[("aws_sagemaker_data_quality_job_definition", &["name"])],
            false,
            &[],
        ),
        (
            "aws_sagemaker_monitoring_schedule",
            "monitoring_schedule_config.monitoring_job_definition.role_arn",
        ) => Rule::new(
            "Inline monitoring role",
            &[("aws_iam_role", &["arn"])],
            false,
            &[],
        ),
        (
            "aws_sagemaker_monitoring_schedule",
            "monitoring_schedule_config.monitoring_job_definition.monitoring_inputs.endpoint_input.endpoint_name",
        ) => Rule::new(
            "Inline monitored endpoint",
            &[("aws_sagemaker_endpoint", &["name"])],
            false,
            &[],
        ),
        ("aws_sagemaker_pipeline", "role_arn") => Rule::new(
            "Pipeline execution role",
            &[("aws_iam_role", &["arn"])],
            false,
            &[],
        ),
        ("aws_sagemaker_pipeline", "pipeline_definition_s3_location.bucket") => Rule::new(
            "Pipeline definition bucket",
            &[("aws_s3_bucket", &["bucket", "id"])],
            false,
            &[],
        ),
        ("aws_sagemaker_training_job", "role_arn") => Rule::new(
            "Training execution role",
            &[("aws_iam_role", &["arn"])],
            false,
            &[],
        ),
        ("aws_sagemaker_training_job", "algorithm_specification.algorithm_name") => Rule::new(
            "Training algorithm",
            &[("aws_sagemaker_algorithm", &["algorithm_name"])],
            false,
            &["algorithm_specification"],
        ),
        ("aws_sagemaker_training_job", "mlflow_config.mlflow_resource_arn") => Rule::new(
            "MLflow resource",
            &[
                ("aws_sagemaker_mlflow_app", &["arn"]),
                ("aws_sagemaker_mlflow_tracking_server", &["arn"]),
            ],
            false,
            &["mlflow_config"],
        ),
        ("aws_sagemaker_training_job", "model_package_config.model_package_group_arn") => {
            Rule::new(
                "Model package group",
                &[("aws_sagemaker_model_package_group", &["arn"])],
                false,
                &["model_package_config"],
            )
        }
        (
            "aws_sagemaker_algorithm",
            "validation_specification.validation_profiles.training_job_definition.input_data_config.data_source.file_system_data_source.file_system_id",
        ) => Rule::new(
            "Training file system",
            &[
                ("aws_efs_file_system", &["id"]),
                ("aws_fsx_lustre_file_system", &["id"]),
            ],
            false,
            &[
                "validation_specification",
                "validation_specification.validation_profiles",
                "validation_specification.validation_profiles.training_job_definition",
                "validation_specification.validation_profiles.training_job_definition.input_data_config",
                "validation_specification.validation_profiles.training_job_definition.input_data_config.data_source",
                "validation_specification.validation_profiles.training_job_definition.input_data_config.data_source.file_system_data_source",
            ],
        ),
        (
            "aws_sagemaker_algorithm",
            "validation_specification.validation_profiles.training_job_definition.output_data_config.kms_key_id",
        ) => Rule::new(
            "Configured encryption key",
            &[("aws_kms_key", &["arn", "key_id"])],
            false,
            &[
                "validation_specification",
                "validation_specification.validation_profiles",
                "validation_specification.validation_profiles.training_job_definition",
                "validation_specification.validation_profiles.training_job_definition.output_data_config",
            ],
        ),
        (
            "aws_sagemaker_algorithm",
            "validation_specification.validation_profiles.training_job_definition.resource_config.volume_kms_key_id",
        ) => Rule::new(
            "Configured encryption key",
            &[("aws_kms_key", &["arn", "key_id"])],
            false,
            &[
                "validation_specification",
                "validation_specification.validation_profiles",
                "validation_specification.validation_profiles.training_job_definition",
                "validation_specification.validation_profiles.training_job_definition.resource_config",
            ],
        ),
        (
            "aws_sagemaker_algorithm",
            "validation_specification.validation_profiles.transform_job_definition.transform_output.kms_key_id",
        ) => Rule::new(
            "Configured encryption key",
            &[("aws_kms_key", &["arn", "key_id"])],
            false,
            &[
                "validation_specification",
                "validation_specification.validation_profiles",
                "validation_specification.validation_profiles.transform_job_definition",
                "validation_specification.validation_profiles.transform_job_definition.transform_output",
            ],
        ),
        (
            "aws_sagemaker_algorithm",
            "validation_specification.validation_profiles.transform_job_definition.transform_resources.volume_kms_key_id",
        ) => Rule::new(
            "Configured encryption key",
            &[("aws_kms_key", &["arn", "key_id"])],
            false,
            &[
                "validation_specification",
                "validation_specification.validation_profiles",
                "validation_specification.validation_profiles.transform_job_definition",
                "validation_specification.validation_profiles.transform_job_definition.transform_resources",
            ],
        ),
        (
            "aws_sagemaker_data_quality_job_definition",
            "data_quality_job_output_config.kms_key_id"
            | "job_resources.cluster_config.volume_kms_key_id",
        ) => Rule::new(
            "Configured encryption key",
            &[("aws_kms_key", &["arn", "key_id"])],
            false,
            &[],
        ),
        (
            "aws_sagemaker_data_quality_job_definition",
            "network_config.vpc_config.security_group_ids",
        ) => Rule::new(
            "Job security groups",
            &[("aws_security_group", &["id"])],
            true,
            &[],
        ),
        ("aws_sagemaker_data_quality_job_definition", "network_config.vpc_config.subnets") => {
            Rule::new("Job subnets", &[("aws_subnet", &["id"])], true, &[])
        }
        (
            "aws_sagemaker_hyper_parameter_tuning_job",
            "training_job_definition.hyper_parameter_tuning_resource_config.volume_kms_key_id",
        ) => Rule::new(
            "Configured encryption key",
            &[("aws_kms_key", &["arn", "key_id"])],
            false,
            &[
                "training_job_definition",
                "training_job_definition.hyper_parameter_tuning_resource_config",
            ],
        ),
        (
            "aws_sagemaker_hyper_parameter_tuning_job",
            "training_job_definition.input_data_config.data_source.file_system_data_source.file_system_id",
        ) => Rule::new(
            "Training file system",
            &[
                ("aws_efs_file_system", &["id"]),
                ("aws_fsx_lustre_file_system", &["id"]),
            ],
            false,
            &[
                "training_job_definition",
                "training_job_definition.input_data_config",
                "training_job_definition.input_data_config.data_source",
                "training_job_definition.input_data_config.data_source.file_system_data_source",
            ],
        ),
        (
            "aws_sagemaker_hyper_parameter_tuning_job",
            "training_job_definition.output_data_config.kms_key_id",
        ) => Rule::new(
            "Configured encryption key",
            &[("aws_kms_key", &["arn", "key_id"])],
            false,
            &[
                "training_job_definition",
                "training_job_definition.output_data_config",
            ],
        ),
        (
            "aws_sagemaker_hyper_parameter_tuning_job",
            "training_job_definition.resource_config.volume_kms_key_id",
        ) => Rule::new(
            "Configured encryption key",
            &[("aws_kms_key", &["arn", "key_id"])],
            false,
            &[
                "training_job_definition",
                "training_job_definition.resource_config",
            ],
        ),
        (
            "aws_sagemaker_hyper_parameter_tuning_job",
            "training_job_definition.vpc_config.security_group_ids",
        ) => Rule::new(
            "Job security groups",
            &[("aws_security_group", &["id"])],
            true,
            &[
                "training_job_definition",
                "training_job_definition.vpc_config",
            ],
        ),
        (
            "aws_sagemaker_hyper_parameter_tuning_job",
            "training_job_definition.vpc_config.subnets",
        ) => Rule::new(
            "Job subnets",
            &[("aws_subnet", &["id"])],
            true,
            &[
                "training_job_definition",
                "training_job_definition.vpc_config",
            ],
        ),
        (
            "aws_sagemaker_hyper_parameter_tuning_job",
            "training_job_definitions.hyper_parameter_tuning_resource_config.volume_kms_key_id",
        ) => Rule::new(
            "Configured encryption key",
            &[("aws_kms_key", &["arn", "key_id"])],
            false,
            &[
                "training_job_definitions",
                "training_job_definitions.hyper_parameter_tuning_resource_config",
            ],
        ),
        (
            "aws_sagemaker_hyper_parameter_tuning_job",
            "training_job_definitions.input_data_config.data_source.file_system_data_source.file_system_id",
        ) => Rule::new(
            "Training file system",
            &[
                ("aws_efs_file_system", &["id"]),
                ("aws_fsx_lustre_file_system", &["id"]),
            ],
            false,
            &[
                "training_job_definitions",
                "training_job_definitions.input_data_config",
                "training_job_definitions.input_data_config.data_source",
                "training_job_definitions.input_data_config.data_source.file_system_data_source",
            ],
        ),
        (
            "aws_sagemaker_hyper_parameter_tuning_job",
            "training_job_definitions.output_data_config.kms_key_id",
        ) => Rule::new(
            "Configured encryption key",
            &[("aws_kms_key", &["arn", "key_id"])],
            false,
            &[
                "training_job_definitions",
                "training_job_definitions.output_data_config",
            ],
        ),
        (
            "aws_sagemaker_hyper_parameter_tuning_job",
            "training_job_definitions.resource_config.volume_kms_key_id",
        ) => Rule::new(
            "Configured encryption key",
            &[("aws_kms_key", &["arn", "key_id"])],
            false,
            &[
                "training_job_definitions",
                "training_job_definitions.resource_config",
            ],
        ),
        (
            "aws_sagemaker_hyper_parameter_tuning_job",
            "training_job_definitions.vpc_config.security_group_ids",
        ) => Rule::new(
            "Job security groups",
            &[("aws_security_group", &["id"])],
            true,
            &[
                "training_job_definitions",
                "training_job_definitions.vpc_config",
            ],
        ),
        (
            "aws_sagemaker_hyper_parameter_tuning_job",
            "training_job_definitions.vpc_config.subnets",
        ) => Rule::new(
            "Job subnets",
            &[("aws_subnet", &["id"])],
            true,
            &[
                "training_job_definitions",
                "training_job_definitions.vpc_config",
            ],
        ),
        (
            "aws_sagemaker_monitoring_schedule",
            "monitoring_schedule_config.monitoring_job_definition.monitoring_output_config.kms_key_id"
            | "monitoring_schedule_config.monitoring_job_definition.monitoring_resources.cluster_config.volume_kms_key_id",
        ) => Rule::new(
            "Configured encryption key",
            &[("aws_kms_key", &["arn", "key_id"])],
            false,
            &[],
        ),
        (
            "aws_sagemaker_monitoring_schedule",
            "monitoring_schedule_config.monitoring_job_definition.network_config.vpc_config.security_group_ids",
        ) => Rule::new(
            "Job security groups",
            &[("aws_security_group", &["id"])],
            true,
            &[],
        ),
        (
            "aws_sagemaker_monitoring_schedule",
            "monitoring_schedule_config.monitoring_job_definition.network_config.vpc_config.subnets",
        ) => Rule::new("Job subnets", &[("aws_subnet", &["id"])], true, &[]),
        (
            "aws_sagemaker_training_job",
            "input_data_config.data_source.file_system_data_source.file_system_id",
        ) => Rule::new(
            "Training file system",
            &[
                ("aws_efs_file_system", &["id"]),
                ("aws_fsx_lustre_file_system", &["id"]),
            ],
            false,
            &[
                "input_data_config",
                "input_data_config.data_source",
                "input_data_config.data_source.file_system_data_source",
            ],
        ),
        ("aws_sagemaker_training_job", "output_data_config.kms_key_id") => Rule::new(
            "Configured encryption key",
            &[("aws_kms_key", &["arn", "key_id"])],
            false,
            &["output_data_config"],
        ),
        ("aws_sagemaker_training_job", "resource_config.volume_kms_key_id") => Rule::new(
            "Configured encryption key",
            &[("aws_kms_key", &["arn", "key_id"])],
            false,
            &["resource_config"],
        ),
        ("aws_sagemaker_training_job", "vpc_config.security_group_ids") => Rule::new(
            "Job security groups",
            &[("aws_security_group", &["id"])],
            true,
            &["vpc_config"],
        ),
        ("aws_sagemaker_training_job", "vpc_config.subnets") => Rule::new(
            "Job subnets",
            &[("aws_subnet", &["id"])],
            true,
            &["vpc_config"],
        ),
        _ => return None,
    })
}
pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    rule(kind, path).map(|r| r.label)
}
pub(crate) fn indexed(kind: &str, path: &str) -> bool {
    resource(kind) && configured_links::indexed(kind, path, rule)
}
pub(super) fn bindings(raw: &Input, r: &AttributeReference) -> bool {
    configured_links::bindings(raw, r, rule)
}
pub(super) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    values: &[(&Value, &Value)],
) {
    configured_links::annotate(nodes, refs, values, rule, qualifies)
}
fn qualifies(e: &Evidence<'_>, r: &AttributeReference, s: usize, path: &str) -> bool {
    match (
        e.nodes[r.target].resource_type.as_str(),
        r.attribute.as_str(),
    ) {
        (
            "aws_sagemaker_monitoring_schedule",
            "monitoring_schedule_config.monitoring_job_definition_name",
        ) => {
            e.text(r.target, "monitoring_schedule_config.0.monitoring_type") == Some("DataQuality")
                && empty(
                    e,
                    r.target,
                    "monitoring_schedule_config.0.monitoring_job_definition",
                )
        }
        ("aws_sagemaker_monitoring_schedule", _) => empty(
            e,
            r.target,
            "monitoring_schedule_config.0.monitoring_job_definition_name",
        ),
        (_, p) if p.ends_with("file_system_data_source.file_system_id") => {
            let block = path.strip_suffix(".file_system_id").unwrap();
            let parent = block
                .strip_suffix(".file_system_data_source.0")
                .unwrap_or("");
            empty(e, r.target, &format!("{parent}.s3_data_source"))
                && matches!(
                    (
                        e.text(r.target, &format!("{block}.file_system_type")),
                        e.nodes[s].resource_type.as_str()
                    ),
                    (Some("EFS"), "aws_efs_file_system")
                        | (Some("FSxLustre"), "aws_fsx_lustre_file_system")
                )
        }
        (
            "aws_sagemaker_data_quality_job_definition",
            "data_quality_job_input.endpoint_input.endpoint_name",
        ) => empty(
            e,
            r.target,
            "data_quality_job_input.0.batch_transform_input",
        ),
        ("aws_sagemaker_hyper_parameter_tuning_job", p)
            if p.starts_with("training_job_definition.") =>
        {
            empty(e, r.target, "training_job_definitions")
        }
        ("aws_sagemaker_hyper_parameter_tuning_job", p)
            if p.starts_with("training_job_definitions.") =>
        {
            empty(e, r.target, "training_job_definition")
        }

        _ => true,
    }
}
pub(super) fn present(raw: &Input, graph: &mut Graph) {
    for (kind, subnets, groups) in [
        (
            "aws_sagemaker_training_job",
            "vpc_config.subnets",
            "vpc_config.security_group_ids",
        ),
        (
            "aws_sagemaker_data_quality_job_definition",
            "network_config.vpc_config.subnets",
            "network_config.vpc_config.security_group_ids",
        ),
        (
            "aws_sagemaker_monitoring_schedule",
            "monitoring_schedule_config.monitoring_job_definition.network_config.vpc_config.subnets",
            "monitoring_schedule_config.monitoring_job_definition.network_config.vpc_config.security_group_ids",
        ),
        (
            "aws_sagemaker_hyper_parameter_tuning_job",
            "training_job_definition.vpc_config.subnets",
            "training_job_definition.vpc_config.security_group_ids",
        ),
    ] {
        super::lambda_core::present_network(raw, graph, kind, (subnets, groups), bindings);
    }
}
fn empty(e: &Evidence<'_>, i: usize, p: &str) -> bool {
    e.value(i, p)
        .is_some_and(|v| v.is_null() || v.as_array().is_some_and(Vec::is_empty))
}
#[cfg(test)]
#[path = "../../../tests/unit/semantic/sagemaker_jobs.rs"]
mod tests;
