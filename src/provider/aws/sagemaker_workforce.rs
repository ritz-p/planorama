use super::configured_links::{self, Evidence, Rule};
use crate::model::{AttributeReference, Graph, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;

pub(crate) fn resource(kind: &str) -> bool {
    matches!(
        kind,
        "aws_sagemaker_flow_definition"
            | "aws_sagemaker_labeling_job"
            | "aws_sagemaker_workforce"
            | "aws_sagemaker_workteam"
    )
}
fn rule(kind: &str, path: &str) -> Option<Rule> {
    Some(match (kind, path) {
        ("aws_sagemaker_flow_definition", "role_arn") => Rule::new(
            "Human review role",
            &[("aws_iam_role", &["arn"])],
            false,
            &[],
        ),
        ("aws_sagemaker_flow_definition", "human_loop_config.human_task_ui_arn") => Rule::new(
            "Human task UI",
            &[("aws_sagemaker_human_task_ui", &["arn"])],
            false,
            &[],
        ),
        ("aws_sagemaker_flow_definition", "human_loop_config.workteam_arn") => Rule::new(
            "Review workteam",
            &[("aws_sagemaker_workteam", &["arn"])],
            false,
            &[],
        ),
        ("aws_sagemaker_labeling_job", "role_arn") => {
            Rule::new("Labeling role", &[("aws_iam_role", &["arn"])], false, &[])
        }
        ("aws_sagemaker_labeling_job", "human_task_config.workteam_arn") => Rule::new(
            "Labeling workteam",
            &[("aws_sagemaker_workteam", &["arn"])],
            false,
            &["human_task_config"],
        ),
        ("aws_sagemaker_labeling_job", "human_task_config.pre_human_task_lambda_arn") => Rule::new(
            "Pre-task Lambda",
            &[("aws_lambda_function", &["arn"])],
            false,
            &["human_task_config"],
        ),
        (
            "aws_sagemaker_labeling_job",
            "human_task_config.annotation_consolidation_config.annotation_consolidation_lambda_arn",
        ) => Rule::new(
            "Consolidation Lambda",
            &[("aws_lambda_function", &["arn"])],
            false,
            &[
                "human_task_config",
                "human_task_config.annotation_consolidation_config",
            ],
        ),
        ("aws_sagemaker_labeling_job", "human_task_config.ui_config.human_task_ui_arn") => {
            Rule::new(
                "Labeling task UI",
                &[("aws_sagemaker_human_task_ui", &["arn"])],
                false,
                &["human_task_config", "human_task_config.ui_config"],
            )
        }
        (
            "aws_sagemaker_labeling_job",
            "input_config.data_source.sns_data_source.sns_topic_arn",
        ) => Rule::new(
            "Labeling topic",
            &[("aws_sns_topic", &["arn"])],
            false,
            &[
                "input_config",
                "input_config.data_source",
                "input_config.data_source.sns_data_source",
            ],
        ),
        (
            "aws_sagemaker_labeling_job",
            "labeling_job_algorithms_config.labeling_job_resource_config.volume_kms_key_id",
        ) => Rule::new(
            "Labeling key",
            &[("aws_kms_key", &["arn", "key_id"])],
            false,
            &[
                "labeling_job_algorithms_config",
                "labeling_job_algorithms_config.labeling_job_resource_config",
            ],
        ),
        (
            "aws_sagemaker_labeling_job",
            "labeling_job_algorithms_config.labeling_job_resource_config.vpc_config.security_group_ids",
        ) => Rule::new(
            "Labeling job security groups",
            &[("aws_security_group", &["id"])],
            true,
            &[
                "labeling_job_algorithms_config",
                "labeling_job_algorithms_config.labeling_job_resource_config",
                "labeling_job_algorithms_config.labeling_job_resource_config.vpc_config",
            ],
        ),
        (
            "aws_sagemaker_labeling_job",
            "labeling_job_algorithms_config.labeling_job_resource_config.vpc_config.subnets",
        ) => Rule::new(
            "Labeling job subnets",
            &[("aws_subnet", &["id"])],
            true,
            &[
                "labeling_job_algorithms_config",
                "labeling_job_algorithms_config.labeling_job_resource_config",
                "labeling_job_algorithms_config.labeling_job_resource_config.vpc_config",
            ],
        ),
        ("aws_sagemaker_labeling_job", "output_config.kms_key_id") => Rule::new(
            "Labeling key",
            &[("aws_kms_key", &["arn", "key_id"])],
            false,
            &["output_config"],
        ),
        ("aws_sagemaker_labeling_job", "output_config.sns_topic_arn") => Rule::new(
            "Labeling topic",
            &[("aws_sns_topic", &["arn"])],
            false,
            &["output_config"],
        ),
        ("aws_sagemaker_workforce", "cognito_config.user_pool") => Rule::new(
            "Workforce user pool",
            &[("aws_cognito_user_pool", &["id"])],
            false,
            &[],
        ),
        ("aws_sagemaker_workforce", "cognito_config.client_id") => Rule::new(
            "Workforce pool client",
            &[("aws_cognito_user_pool_client", &["id"])],
            false,
            &[],
        ),
        ("aws_sagemaker_workforce", "workforce_vpc_config.vpc_id") => {
            Rule::new("Workforce VPC", &[("aws_vpc", &["id"])], false, &[])
        }
        ("aws_sagemaker_workforce", "workforce_vpc_config.subnets") => {
            Rule::new("Workforce subnets", &[("aws_subnet", &["id"])], true, &[])
        }
        ("aws_sagemaker_workforce", "workforce_vpc_config.security_group_ids") => Rule::new(
            "Workforce security groups",
            &[("aws_security_group", &["id"])],
            true,
            &[],
        ),
        ("aws_sagemaker_workteam", "workforce_name") => Rule::new(
            "Private workforce",
            &[("aws_sagemaker_workforce", &["workforce_name"])],
            false,
            &[],
        ),
        ("aws_sagemaker_workteam", "member_definition.cognito_member_definition.user_pool") => {
            Rule::new(
                "Member user pool",
                &[("aws_cognito_user_pool", &["id"])],
                false,
                &["member_definition"],
            )
        }
        ("aws_sagemaker_workteam", "member_definition.cognito_member_definition.client_id") => {
            Rule::new(
                "Member pool client",
                &[("aws_cognito_user_pool_client", &["id"])],
                false,
                &["member_definition"],
            )
        }
        ("aws_sagemaker_workteam", "member_definition.cognito_member_definition.user_group") => {
            Rule::new(
                "Member pool group",
                &[("aws_cognito_user_group", &["name"])],
                false,
                &["member_definition"],
            )
        }
        ("aws_sagemaker_workteam", "notification_configuration.notification_topic_arn") => {
            Rule::new(
                "Workteam notifications",
                &[("aws_sns_topic", &["arn"])],
                false,
                &[],
            )
        }
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
        (_, p)
            if p.contains("cognito")
                && (p.ends_with(".client_id") || p.ends_with(".user_group")) =>
        {
            (!p.starts_with("cognito_config.")
                || e.value(r.target, "oidc_config")
                    .is_some_and(|v| v.is_null() || v.as_array().is_some_and(Vec::is_empty)))
                && path.rsplit_once('.').is_some_and(|(parent, _)| {
                    e.same_endpoint(
                        r.target,
                        &format!("{parent}.user_pool"),
                        s,
                        "user_pool_id",
                        "aws_cognito_user_pool",
                        "id",
                    )
                })
        }
        ("aws_sagemaker_workforce", p) if p.starts_with("cognito_config.") => e
            .value(r.target, "oidc_config")
            .is_some_and(|v| v.is_null() || v.as_array().is_some_and(Vec::is_empty)),
        _ => true,
    }
}
pub(super) fn present(raw: &Input, graph: &mut Graph) {
    super::lambda_core::present_network(
        raw,
        graph,
        "aws_sagemaker_labeling_job",
        (
            "labeling_job_algorithms_config.labeling_job_resource_config.vpc_config.subnets",
            "labeling_job_algorithms_config.labeling_job_resource_config.vpc_config.security_group_ids",
        ),
        bindings,
    );
    super::lambda_core::present_network(
        raw,
        graph,
        "aws_sagemaker_workforce",
        (
            "workforce_vpc_config.subnets",
            "workforce_vpc_config.security_group_ids",
        ),
        |input, r| {
            bindings(input, r)
                && super::lambda_core::unique(
                    &input.attributes,
                    r.target,
                    "workforce_vpc_config.vpc_id",
                )
                .is_some_and(|v| {
                    v.service_reference_verified && v.sources == r.endpoint_network_context
                })
        },
    );
}

#[cfg(test)]
#[path = "../../../tests/unit/semantic/sagemaker_workforce.rs"]
mod tests;
