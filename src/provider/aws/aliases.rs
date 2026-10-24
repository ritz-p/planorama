use crate::model::{EntityMode, TerraformEntity};

pub(super) fn canonical(resource: &str) -> &str {
    match resource {
        "aws_alb" => "aws_lb",
        "aws_alb_listener" => "aws_lb_listener",
        "aws_alb_listener_certificate" => "aws_lb_listener_certificate",
        "aws_alb_listener_rule" => "aws_lb_listener_rule",
        "aws_alb_target_group" => "aws_lb_target_group",
        "aws_alb_target_group_attachment" => "aws_lb_target_group_attachment",
        _ => resource,
    }
}

pub(super) fn data_type(resource: &str) -> &str {
    match resource {
        "aws_alb" | "aws_alb_listener" | "aws_alb_target_group" => canonical(resource),
        _ => resource,
    }
}

pub(super) fn entity_type(entity: &TerraformEntity) -> &str {
    if !entity.provider.is_aws() {
        &entity.resource_type
    } else if entity.mode == EntityMode::Data {
        data_type(&entity.resource_type)
    } else {
        canonical(&entity.resource_type)
    }
}

pub(super) fn node_type(node: &crate::model::Node) -> &str {
    if !node.provider.is_aws() {
        &node.resource_type
    } else if node.mode == EntityMode::Data {
        data_type(&node.resource_type)
    } else {
        canonical(&node.resource_type)
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/aws_aliases.rs"]
mod tests;
