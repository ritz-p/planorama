use super::{Evidence, Rule};
use crate::model::AttributeReference;

pub(super) fn resource(kind: &str) -> bool {
    matches!(kind, "aws_codedeploy_deployment_group")
}

pub(super) fn rule(kind: &str, path: &str) -> Option<Rule> {
    Some(match (kind, path) {
        ("aws_codedeploy_deployment_group", "app_name") => Rule::new(
            "Deployment application",
            &[("aws_codedeploy_app", &["name"])],
            false,
            &[],
        ),
        ("aws_codedeploy_deployment_group", "deployment_config_name") => Rule::new(
            "Deployment configuration",
            &[(
                "aws_codedeploy_deployment_config",
                &["deployment_config_name"],
            )],
            false,
            &[],
        ),
        ("aws_codedeploy_deployment_group", "service_role_arn") => {
            Rule::new("Deployment role", &[("aws_iam_role", &["arn"])], false, &[])
        }
        ("aws_codedeploy_deployment_group", "autoscaling_groups") => Rule::new(
            "Deployment Auto Scaling groups",
            &[("aws_autoscaling_group", &["name"])],
            true,
            &[],
        ),
        ("aws_codedeploy_deployment_group", "ecs_service.cluster_name") => Rule::new(
            "Deployment ECS cluster",
            &[("aws_ecs_cluster", &["name"])],
            false,
            &["ecs_service"],
        ),
        ("aws_codedeploy_deployment_group", "ecs_service.service_name") => Rule::new(
            "Deployment ECS service",
            &[("aws_ecs_service", &["name"])],
            false,
            &["ecs_service"],
        ),
        ("aws_codedeploy_deployment_group", "load_balancer_info.elb_info.name") => Rule::new(
            "Deployment classic load balancer",
            &[("aws_elb", &["name"])],
            false,
            &["load_balancer_info.elb_info"],
        ),
        ("aws_codedeploy_deployment_group", "load_balancer_info.target_group_info.name") => {
            Rule::new(
                "Deployment target group",
                &[("aws_lb_target_group", &["name"])],
                false,
                &["load_balancer_info.target_group_info"],
            )
        }
        (
            "aws_codedeploy_deployment_group",
            "load_balancer_info.target_group_pair_info.target_group.name",
        ) => Rule::new(
            "Blue/green target group",
            &[("aws_lb_target_group", &["name"])],
            false,
            &[
                "load_balancer_info.target_group_pair_info",
                "load_balancer_info.target_group_pair_info.target_group",
            ],
        ),
        (
            "aws_codedeploy_deployment_group",
            "load_balancer_info.target_group_pair_info.prod_traffic_route.listener_arns",
        ) => Rule::new(
            "Deployment prod listener",
            &[("aws_lb_listener", &["arn"])],
            true,
            &["load_balancer_info.target_group_pair_info"],
        ),
        (
            "aws_codedeploy_deployment_group",
            "load_balancer_info.target_group_pair_info.test_traffic_route.listener_arns",
        ) => Rule::new(
            "Deployment test listener",
            &[("aws_lb_listener", &["arn"])],
            true,
            &["load_balancer_info.target_group_pair_info"],
        ),
        ("aws_codedeploy_deployment_group", "trigger_configuration.trigger_target_arn") => {
            Rule::new(
                "Deployment notification topic",
                &[("aws_sns_topic", &["arn"])],
                false,
                &["trigger_configuration"],
            )
        }
        _ => return None,
    })
}

pub(super) fn qualifies(
    e: &Evidence<'_>,
    r: &AttributeReference,
    source: usize,
    path: &str,
) -> bool {
    match r.attribute.as_str() {
        "deployment_config_name" => e
            .text(source, "deployment_config_name")
            .is_some_and(|name| !name.starts_with("CodeDeployDefault.")),
        "ecs_service.service_name" => {
            let (block, _) = path.rsplit_once('.').unwrap();
            let named = e.configured_endpoint(
                r.target,
                &format!("{block}.cluster_name"),
                "aws_ecs_cluster",
                &["name"],
            );
            let actual =
                e.configured_endpoint(source, "cluster", "aws_ecs_cluster", &["id", "arn", "name"]);
            named.is_some() && named == actual
        }
        _ => true,
    }
}
