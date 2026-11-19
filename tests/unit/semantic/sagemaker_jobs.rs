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

#[test]
fn tuning_placement_requires_complete_network_evidence_for_each_definition() {
    let mut plan = json!({
        "format_version":"1.2",
        "resource_changes":[],
        "configuration": {
            "provider_config":{"aws":{"full_name":"hashicorp/aws","expressions":{"region":{"constant_value":"us-east-1"}}}},
            "root_module":{"resources":[]}
        }
    });
    let network = json!({"subnets":["subnet-0123"],"security_group_ids":["sg-0123"]});
    let expressions = json!({"subnets":{"references":["aws_subnet.training.id"]},"security_group_ids":{"references":["aws_security_group.training.id"]}});
    for (kind, name, after, config) in [
        ("aws_vpc", "training", json!({"id":"vpc-0123"}), json!({})),
        (
            "aws_subnet",
            "training",
            json!({"id":"subnet-0123","vpc_id":"vpc-0123"}),
            json!({"vpc_id":{"references":["aws_vpc.training.id"]}}),
        ),
        (
            "aws_security_group",
            "training",
            json!({"id":"sg-0123","vpc_id":"vpc-0123"}),
            json!({"vpc_id":{"references":["aws_vpc.training.id"]}}),
        ),
        (
            "aws_sagemaker_hyper_parameter_tuning_job",
            "job",
            json!({"training_job_definitions":[{"vpc_config":[network]},{"vpc_config":[network]}]}),
            json!({"training_job_definitions":[{"vpc_config":[expressions]},{"vpc_config":[expressions]}]}),
        ),
    ] {
        let address = format!("{kind}.{name}");
        plan["resource_changes"].as_array_mut().unwrap().push(json!({
            "address":address,"mode":"managed","type":kind,"name":name,"provider_name":"registry.terraform.io/hashicorp/aws","change":{"actions":["create"],"after":after}
        }));
        plan["configuration"]["root_module"]["resources"].as_array_mut().unwrap().push(json!({
            "address":address,"mode":"managed","type":kind,"name":name,"provider_config_key":"aws","expressions":config
        }));
    }
    let contained = |input: &Value| {
        let graph = semantic::transform(&plan::parse(&input.to_string()).unwrap());
        graph.edges.iter().any(|edge| {
            edge.kind == crate::model::EdgeKind::Containment
                && graph.nodes[edge.from].address == "aws_vpc.training"
                && graph.nodes[edge.to].address == "aws_sagemaker_hyper_parameter_tuning_job.job"
        })
    };
    assert!(contained(&plan));
    plan["resource_changes"][3]["change"]["after"]["training_job_definitions"][1]["vpc_config"] =
        json!([]);
    assert!(!contained(&plan));
}
