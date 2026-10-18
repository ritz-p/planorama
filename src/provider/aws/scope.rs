use crate::model::{TerraformEntity, architecture::DeploymentScope};

pub(in crate::provider) fn scope(entity: &TerraformEntity) -> Option<DeploymentScope> {
    let resource = entity.resource_type.as_str();
    let global = resource.starts_with("aws_iam_")
        || resource.starts_with("aws_cloudfront_")
        || matches!(
            resource,
            "aws_route53_zone"
                | "aws_route53_record"
                | "aws_route53_health_check"
                | "aws_route53_delegation_set"
                | "aws_route53_traffic_policy"
                | "aws_route53_traffic_policy_instance"
        );
    let account = entity
        .provider_configuration
        .as_ref()
        .and_then(|c| c.account.clone());
    let region = if global {
        None
    } else if matches!(
        resource,
        "aws_vpc"
            | "aws_efs_file_system"
            | "aws_efs_mount_target"
            | "aws_efs_access_point"
            | "aws_acm_certificate"
            | "aws_acm_certificate_validation"
            | "aws_kms_key"
            | "aws_kms_alias"
            | "aws_secretsmanager_secret"
            | "aws_ssm_parameter"
            | "aws_secretsmanager_secret_version"
            | "aws_s3_bucket_server_side_encryption_configuration"
            | "aws_subnet"
            | "aws_instance"
            | "aws_ami"
            | "aws_eip"
            | "aws_db_instance"
            | "aws_db_subnet_group"
            | "aws_lambda_function"
            | "aws_sqs_queue"
            | "aws_sns_topic"
            | "aws_cloudwatch_event_bus"
            | "aws_cloudwatch_event_rule"
            | "aws_cloudwatch_event_target"
            | "aws_sns_topic_subscription"
            | "aws_lambda_event_source_mapping"
            | "aws_rds_cluster"
            | "aws_rds_cluster_instance"
            | "aws_dynamodb_table"
            | "aws_elasticache_cluster"
            | "aws_elasticache_subnet_group"
            | "aws_elasticache_replication_group"
            | "aws_opensearch_domain"
            | "aws_s3_bucket"
            | "aws_cloudwatch_log_group"
            | "aws_launch_template"
            | "aws_internet_gateway"
            | "aws_nat_gateway"
            | "aws_route_table"
            | "aws_route"
            | "aws_vpc_endpoint"
            | "aws_ec2_transit_gateway"
            | "aws_ec2_transit_gateway_route_table"
            | "aws_ec2_transit_gateway_route"
            | "aws_lb"
            | "aws_lb_listener"
            | "aws_api_gateway_rest_api"
            | "aws_apigatewayv2_api"
            | "aws_route_table_association"
            | "aws_lb_target_group_attachment"
            | "aws_ec2_transit_gateway_vpc_attachment"
            | "aws_ec2_transit_gateway_route_table_association"
            | "aws_ec2_transit_gateway_route_table_propagation"
            | "aws_vpc_endpoint_route_table_association"
            | "aws_vpc_endpoint_subnet_association"
            | "aws_security_group"
            | "aws_network_acl"
            | "aws_lb_listener_rule"
            | "aws_autoscaling_group"
            | "aws_autoscaling_policy"
            | "aws_ecs_service"
            | "aws_ecs_cluster"
            | "aws_ecs_task_definition"
            | "aws_eks_cluster"
            | "aws_eks_node_group"
            | "aws_lb_target_group"
            | "aws_cloudwatch_metric_alarm"
    ) {
        if entity.has_region_override {
            entity.region_override.clone()
        } else {
            entity
                .provider_configuration
                .as_ref()
                .and_then(|c| c.region.clone())
        }
    } else {
        None
    };
    if !global && region.is_none() && account.is_none() {
        return None;
    }
    Some(DeploymentScope {
        provider: "AWS".into(),
        region,
        account,
        global,
    })
}
