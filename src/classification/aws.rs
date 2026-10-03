use crate::model::ResourceRole;

#[cfg(test)]
#[path = "../../tests/unit/classification/aws.rs"]
mod tests;

pub(super) fn classify(resource_type: &str) -> ResourceRole {
    match resource_type {
        "aws_vpc" | "aws_subnet" => ResourceRole::Container,
        "aws_instance" | "aws_db_instance" => ResourceRole::Node,
        "aws_vpc_peering_connection" => ResourceRole::Connector,
        "aws_route_table_association" => ResourceRole::Association,
        "aws_security_group" | "aws_network_acl" => ResourceRole::Policy,
        "aws_autoscaling_group" | "aws_ecs_service" => ResourceRole::Controller,
        _ => ResourceRole::Unknown,
    }
}
