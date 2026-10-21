mod associations;
mod autoscaling;
mod classification;
mod cloudfront;
#[cfg(test)]
#[path = "../../tests/unit/semantic/cognito.rs"]
mod cognito_tests;
mod components;
mod containment;
mod data;
pub(super) mod diagnostics;
mod efs;
mod firehose;
mod glue;
mod iam;
mod icons;
mod members;
mod messaging;
mod monitoring;
mod network;
pub(crate) mod organization_accounts;
pub(crate) mod organization_policies;
pub(crate) mod organizations;
mod placement;
mod rds_proxy;

#[cfg(test)]
#[path = "../../tests/unit/semantic/rds_global.rs"]
mod rds_global_tests;
pub(crate) mod service_references;
pub(crate) use placement::owner_affinities;
mod routes;
pub(crate) mod s3;
mod scope;
mod security_groups;
mod service_members;
pub(crate) mod waf;

#[cfg(test)]
#[path = "../../tests/unit/semantic/athena.rs"]
mod athena_tests;

#[cfg(test)]
#[path = "../../tests/unit/semantic/private_ca.rs"]
mod private_ca_tests;

#[cfg(test)]
#[path = "../../tests/unit/semantic/log_groups.rs"]
mod log_group_tests;

#[cfg(test)]
#[path = "../../tests/unit/semantic/ecs_clusters.rs"]
mod ecs_cluster_tests;

#[cfg(test)]
#[path = "../../tests/unit/semantic/eks.rs"]
mod eks_tests;

#[cfg(test)]
#[path = "../../tests/unit/semantic/serverless.rs"]
mod serverless_tests;

#[cfg(test)]
#[path = "../../tests/unit/resource_support.rs"]
mod resource_support;
pub(super) use scope::scope;

pub(super) fn reference_label(resource_type: &str, attribute: &str) -> &'static str {
    match (resource_type, attribute) {
        ("aws_ecs_service", "cluster") => "Cluster",
        ("aws_ecs_service", "task_definition") => "Task definition",
        ("aws_instance", "iam_instance_profile") => "Instance profile",
        ("aws_instance", "ami") => "AMI",
        (
            "aws_autoscaling_group",
            "launch_template.id"
            | "launch_template.name"
            | "mixed_instances_policy.launch_template.launch_template_specification.launch_template_id"
            | "mixed_instances_policy.launch_template.launch_template_specification.launch_template_name",
        ) => "Launch template",
        ("aws_iam_instance_profile", "role") => "Role",
        ("aws_ecs_task_definition", "execution_role_arn") => "Execution role",
        ("aws_ecs_task_definition", "task_role_arn") => "Task role",
        ("aws_cloudwatch_metric_alarm", "dimensions" | "dimensions.DBInstanceIdentifier") => {
            "Monitors"
        }
        ("aws_autoscaling_policy", "autoscaling_group_name") => "Auto Scaling group",
        _ => "References",
    }
}

pub(super) fn placement(resource_type: &str) -> crate::model::Placement {
    match resource_type {
        "aws_organizations_policy" | "aws_organizations_policy_attachment" => {
            crate::model::Placement::Logical
        }
        "aws_db_proxy_default_target_group" | "aws_db_proxy_target" => {
            crate::model::Placement::Logical
        }
        "aws_eks_addon" | "aws_eks_access_entry" => crate::model::Placement::Logical,
        "aws_ecr_repository_policy" | "aws_ecr_lifecycle_policy" => {
            crate::model::Placement::Logical
        }
        "aws_s3_bucket_policy"
        | "aws_s3_bucket_notification"
        | "aws_s3_bucket_replication_configuration" => crate::model::Placement::Logical,
        "aws_wafv2_web_acl_association" => crate::model::Placement::Logical,
        "aws_cloudfront_origin_access_control"
        | "aws_cloudfront_origin_access_identity"
        | "aws_cloudfront_cache_policy"
        | "aws_cloudfront_origin_request_policy"
        | "aws_cloudfront_response_headers_policy" => crate::model::Placement::Logical,
        "aws_acmpca_certificate_authority_certificate"
        | "aws_acm_certificate_validation"
        | "aws_ssm_parameter"
        | "aws_s3_bucket_server_side_encryption_configuration"
        | "aws_ecs_task_definition"
        | "aws_iam_role"
        | "aws_iam_user"
        | "aws_iam_openid_connect_provider"
        | "aws_iam_saml_provider"
        | "aws_iam_group"
        | "aws_iam_user_policy"
        | "aws_iam_group_policy"
        | "aws_iam_group_membership"
        | "aws_iam_user_group_membership"
        | "aws_iam_user_policy_attachment"
        | "aws_iam_group_policy_attachment"
        | "aws_iam_policy"
        | "aws_iam_role_policy"
        | "aws_iam_instance_profile"
        | "aws_cloudwatch_metric_alarm"
        | "aws_autoscaling_policy"
        | "aws_ami"
        | "aws_launch_template" => crate::model::Placement::Logical,
        "aws_sns_topic_subscription"
        | "aws_lambda_event_source_mapping"
        | "aws_cloudwatch_event_target" => crate::model::Placement::Logical,
        _ if classify(resource_type) != crate::model::ResourceRole::Unknown => {
            crate::model::Placement::Spatial
        }
        _ => crate::model::Placement::Unresolved,
    }
}

pub(crate) fn classify(resource_type: &str) -> crate::model::ResourceRole {
    classification::classify(resource_type)
}
pub(crate) fn icon_for(resource_type: &str) -> Option<crate::icons::ResourceIcon> {
    icons::icon_for(resource_type)
}
pub(super) fn transform(raw: crate::semantic::Input) -> crate::model::Graph {
    let mut graph = security_groups::infer(&raw, containment::infer(&raw));
    routes::present(&raw, &mut graph);
    members::present(&raw, &mut graph);
    monitoring::present(&raw, &mut graph);
    autoscaling::present(&raw, &mut graph);
    messaging::present(&raw, &mut graph);
    service_references::present(&raw, &mut graph);
    efs::present(&raw, &mut graph);
    service_members::present(&raw, &mut graph);
    let contained = crate::semantic::Input { graph, ..raw };
    let mut graph = data::visible(associations::lower(&contained));
    graph
        .components
        .extend(components::infer(&contained, &graph));
    organizations::present(&contained, &mut graph);
    organization_policies::present(&contained, &mut graph);
    graph
}
