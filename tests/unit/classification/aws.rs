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
        "aws_vpc_endpoint",
        "aws_security_group_rule",
        "aws_",
        "",
        "AWS_VPC",
    ] {
        assert_eq!(classify(resource_type), ResourceRole::Unknown);
    }
}
