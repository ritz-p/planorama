use crate::{
    icons::ResourceIcon,
    model::{Placement, ResourceRole},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Scope {
    Regional,
    Global,
    Unknown,
}

pub(super) struct Basic {
    pub resource_type: &'static str,
    pub service: &'static str,
    pub role: ResourceRole,
    pub placement: Placement,
    pub scope: Scope,
    pub icon: Option<ResourceIcon>,
}

macro_rules! entry {
    ($resource:literal, $service:literal, $role:ident, $placement:ident, $scope:ident, $icon:expr) => {
        Basic {
            resource_type: $resource,
            service: $service,
            role: ResourceRole::$role,
            placement: Placement::$placement,
            scope: Scope::$scope,
            icon: $icon,
        }
    };
}

#[path = "basic_entries.rs"]
mod entries;
pub(super) use entries::ENTRIES;

pub(super) fn get(resource_type: &str) -> Option<&'static Basic> {
    ENTRIES
        .binary_search_by_key(&resource_type, |entry| entry.resource_type)
        .ok()
        .map(|index| &ENTRIES[index])
}

#[cfg(test)]
pub(super) fn unknown_scope_reason(resource_type: &str) -> Option<&'static str> {
    match resource_type {
        "aws_bedrock_use_case_for_model_access" => Some(
            "Account model-access setting; registration disables region override and does not prove a resource region",
        ),
        "aws_cloudcontrolapi_resource" => Some(
            "Scope depends on the Cloud Control type_name; arbitrary target schemas are not interpreted",
        ),
        "aws_cloudformation_stack_instances" | "aws_cloudformation_stack_set_instance" => Some(
            "Targets other accounts/regions; deployment target evidence is not resolved by basic classification",
        ),
        "aws_config_aggregate_authorization" => Some(
            "Authorization refers to an aggregation account and region; target evidence is not resolved",
        ),
        "aws_dx_hosted_connection" => Some(
            "Partner connection uses an interconnect/location; provider region is not target evidence",
        ),
        "aws_s3control_multi_region_access_point"
        | "aws_s3control_multi_region_access_point_policy"
        | "aws_s3control_multi_region_access_point_routes" => Some(
            "Multi-region access point; control-plane region does not identify the represented bucket regions",
        ),
        "aws_ssmincidents_replication_set" => {
            Some("Replication set spans configured regions; no single resource region")
        }
        "aws_wafv2_api_key"
        | "aws_wafv2_ip_set"
        | "aws_wafv2_regex_pattern_set"
        | "aws_wafv2_web_acl_logging_configuration"
        | "aws_wafv2_web_acl_rule"
        | "aws_wafv2_web_acl_rule_group_association" => Some(
            "WAF scope or referenced ACL evidence is required; basic classification does not resolve it",
        ),
        _ => None,
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/aws_inventory.rs"]
mod tests;
