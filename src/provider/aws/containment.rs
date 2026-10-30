use crate::model::{EdgeKind, EntityMode, Graph, ResourceRole};
use crate::semantic::Input;
use std::collections::BTreeSet;
mod indirect;
pub(super) use indirect::{group_configuration_matches, resolved_subnets};
mod spanning;
mod vpc_scope;
pub(super) use vpc_scope::attribute as vpc_scope_attribute;

pub(super) fn direct_rule(resource_type: &str) -> Option<(&'static str, &'static str)> {
    let resource_type = super::aliases::canonical(resource_type);
    match resource_type {
        "aws_subnet"
        | "aws_vpc_endpoint"
        | "aws_route_table"
        | "aws_security_group"
        | "aws_network_acl"
        | "aws_lb_target_group"
        | "aws_internet_gateway"
        | "aws_egress_only_internet_gateway" => Some(("vpc_id", "aws_vpc")),
        "aws_network_interface"
        | "aws_ec2_instance_connect_endpoint"
        | "aws_instance"
        | "aws_nat_gateway"
        | "aws_efs_mount_target" => Some(("subnet_id", "aws_subnet")),
        _ => None,
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/semantic/containment.rs"]
mod tests;

pub(super) fn infer(raw: &Input) -> Graph {
    let mut relationships = BTreeSet::new();
    for reference in &raw.attributes {
        let child = &raw.nodes[reference.target];
        if child.mode != EntityMode::Managed
            || !child.provider.is_aws()
            || !reference.complete
            || reference
                .issues
                .contains(&crate::model::DiagnosticReason::DynamicInstanceSelection)
        {
            continue;
        }
        let rule = if child.resource_type == "aws_nat_gateway" && reference.attribute == "vpc_id" {
            Some(("vpc_id", "aws_vpc"))
        } else {
            direct_rule(&child.resource_type)
        };
        let Some((attribute, parent_type)) = rule else {
            continue;
        };
        if reference.attribute != attribute {
            continue;
        }
        if matches!(
            child.resource_type.as_str(),
            "aws_internet_gateway" | "aws_egress_only_internet_gateway" | "aws_nat_gateway"
        ) && (!reference.service_reference_verified
            || (reference.attribute != "subnet_id"
                && !super::service_references::verified(raw, reference)))
        {
            continue;
        }
        if matches!(
            child.resource_type.as_str(),
            "aws_network_interface" | "aws_ec2_instance_connect_endpoint"
        ) && !super::service_references::verified(raw, reference)
        {
            continue;
        }
        if child.resource_type == "aws_efs_mount_target" && !reference.service_reference_verified {
            continue;
        }
        if let [parent] = reference.sources.as_slice() {
            let node = &raw.nodes[*parent];
            if *parent != reference.target
                && node.resource_type == parent_type
                && node.provider.is_aws()
                && node.role == ResourceRole::Container
            {
                relationships.insert((*parent, reference.target));
            }
        }
    }
    let mut graph = raw.graph.clone();
    for edge in &mut graph.edges {
        if edge.kind == EdgeKind::Dependency && relationships.contains(&edge.endpoints()) {
            edge.kind = EdgeKind::Containment;
        }
    }
    for (from, to) in relationships {
        crate::semantic::relationships::record_reference(
            &mut graph,
            from,
            to,
            EdgeKind::Containment,
        );
    }
    indirect::infer(raw, vpc_scope::infer(raw, spanning::infer(raw, graph)))
}
