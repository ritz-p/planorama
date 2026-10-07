use super::*;

#[test]
fn representative_aws_types_cover_each_architectural_role() {
    for (resource_type, expected) in [
        ("aws_vpc", ResourceRole::Container),
        ("aws_subnet", ResourceRole::Container),
        ("aws_instance", ResourceRole::Node),
        ("aws_db_instance", ResourceRole::Node),
        ("aws_vpc_peering_connection", ResourceRole::Connector),
        ("aws_route_table_association", ResourceRole::Association),
        ("aws_security_group", ResourceRole::Policy),
        ("aws_network_acl", ResourceRole::Policy),
        ("aws_autoscaling_group", ResourceRole::Controller),
        ("aws_ecs_service", ResourceRole::Controller),
    ] {
        assert_eq!(classify(resource_type), expected, "{resource_type}");
        assert_eq!(classify(resource_type), classify(resource_type));
    }
}

#[test]
fn unknown_and_similar_names_are_not_guessed() {
    for resource_type in [
        "aws_future_service",
        "aws_vpc_endpoint_future",
        "aws_security_group_rule",
        "aws_",
        "",
        "AWS_VPC",
    ] {
        assert_eq!(classify(resource_type), ResourceRole::Unknown);
    }
}

#[test]
fn expanded_roles_cover_managed_and_equivalent_data_entities() {
    use crate::{model::EntityMode, plan};
    use serde_json::json;
    for (expected, types) in [
        (
            ResourceRole::Node,
            "aws_ami aws_eip aws_db_subnet_group aws_lambda_function aws_rds_cluster aws_dynamodb_table aws_elasticache_cluster aws_opensearch_domain aws_s3_bucket aws_cloudwatch_log_group",
        ),
        (
            ResourceRole::Connector,
            "aws_internet_gateway aws_nat_gateway aws_route_table aws_route aws_vpc_endpoint aws_ec2_transit_gateway aws_ec2_transit_gateway_route_table aws_ec2_transit_gateway_route aws_lb aws_lb_listener aws_cloudfront_distribution aws_api_gateway_rest_api aws_apigatewayv2_api",
        ),
        (
            ResourceRole::Association,
            "aws_lb_target_group_attachment aws_ec2_transit_gateway_vpc_attachment aws_ec2_transit_gateway_peering_attachment aws_ec2_transit_gateway_route_table_association aws_ec2_transit_gateway_route_table_propagation aws_vpc_endpoint_route_table_association aws_vpc_endpoint_subnet_association aws_iam_role_policy_attachment",
        ),
        (
            ResourceRole::Policy,
            "aws_iam_role aws_iam_policy aws_iam_role_policy aws_iam_instance_profile aws_lb_listener_rule",
        ),
        (
            ResourceRole::Controller,
            "aws_ecs_cluster aws_ecs_task_definition aws_eks_cluster aws_eks_node_group aws_lb_target_group aws_cloudwatch_metric_alarm",
        ),
    ] {
        for resource_type in types.split_whitespace() {
            assert_eq!(classify(resource_type), expected, "{resource_type}");
            let input = json!({"format_version":"1.2","resource_changes":[
                {"address":format!("{resource_type}.main"),"type":resource_type,"mode":"managed","change":{"actions":["create"]}},
                {"address":format!("data.{resource_type}.main"),"type":resource_type,"mode":"data","change":{"actions":["read"]}}
            ]});
            let graph = plan::parse(&input.to_string()).unwrap();
            assert_eq!(graph.nodes.len(), 2);
            assert!(graph.nodes.iter().all(|node| node.role == expected));
            assert!(
                graph
                    .nodes
                    .iter()
                    .any(|node| node.mode == EntityMode::Managed)
            );
            assert!(graph.nodes.iter().any(|node| node.mode == EntityMode::Data));
        }
    }
}

#[test]
fn large_example_has_classified_architecture_resources_and_only_spatial_containers() {
    let raw =
        crate::plan::parse(include_str!("../../../examples/terraform-large/plan.json")).unwrap();
    let graph = crate::semantic::transform(&raw);
    assert_eq!(graph.nodes.len(), 38);
    assert_eq!(graph.edges.len(), 73);
    for node in &graph.nodes {
        assert_ne!(node.role, ResourceRole::Unknown, "{}", node.resource_type);
        assert_eq!(
            node.role == ResourceRole::Container,
            matches!(node.resource_type.as_str(), "aws_vpc" | "aws_subnet")
        );
    }
}
