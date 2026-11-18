use crate::{plan, semantic};
use serde_json::{Value, json};
fn proven(v: &Value, address: &str, path: &str) -> bool {
    let g = semantic::transform(&plan::parse(&v.to_string()).unwrap());
    g.nodes
        .iter()
        .find(|n| n.address == address)
        .is_some_and(|n| {
            n.references.iter().any(|r| {
                r.attribute == path
                    && r.complete
                    && !r.dependency
                    && !r.targets.is_empty()
                    && super::label(&n.resource_type, path) == Some(r.label)
            })
        })
}

#[test]
fn sagemaker_training_jobs_requires_scoped_provenance() {
    let original: Value = serde_json::from_str(include_str!(
        "../../fixtures/sagemaker-training-jobs-plan.json"
    ))
    .unwrap();
    for (addr, path) in [
        ("aws_sagemaker_training_job.job", "role_arn"),
        (
            "aws_sagemaker_training_job.job",
            "algorithm_specification.algorithm_name",
        ),
        (
            "aws_sagemaker_training_job.job",
            "input_data_config.data_source.file_system_data_source.file_system_id",
        ),
        (
            "aws_sagemaker_monitoring_schedule.s",
            "monitoring_schedule_config.monitoring_job_definition_name",
        ),
    ] {
        assert!(proven(&original, addr, path), "{addr}/{path}");
    }
    {
        let mut v = original.clone();
        v["resource_changes"][3]["change"]["after"]["input_data_config"][0]["data_source"][0]["file_system_data_source"]
            [0]["file_system_type"] = json!("FSxLustre");
        assert!(
            !proven(
                &v,
                "aws_sagemaker_training_job.job",
                "input_data_config.data_source.file_system_data_source.file_system_id"
            ),
            "unexpected proof for aws_sagemaker_training_job.job input_data_config.data_source.file_system_data_source.file_system_id"
        );
    }
    {
        let mut v = original.clone();
        v["resource_changes"][5]["change"]["after"]["monitoring_schedule_config"][0]["monitoring_type"] =
            json!("ModelQuality");
        assert!(
            !proven(
                &v,
                "aws_sagemaker_monitoring_schedule.s",
                "monitoring_schedule_config.monitoring_job_definition_name"
            ),
            "unexpected proof for aws_sagemaker_monitoring_schedule.s monitoring_schedule_config.monitoring_job_definition_name"
        );
    }
    {
        let mut v = original.clone();
        v["resource_changes"][0]["change"]["after_unknown"] = json!({"arn":true});
        assert!(
            !proven(&v, "aws_sagemaker_training_job.job", "role_arn"),
            "unexpected proof for aws_sagemaker_training_job.job role_arn"
        );
    }
}
