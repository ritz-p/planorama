use super::*;

#[test]
fn identities_require_exact_aws_types_and_keep_logical_placement() {
    use crate::model::{Placement, ProviderIdentity};
    for resource_type in [
        "aws_iam_role",
        "aws_iam_user",
        "aws_iam_group",
        "aws_iam_openid_connect_provider",
        "aws_iam_saml_provider",
    ] {
        let input = serde_json::json!({"format_version":"1.2", "resource_changes": [
            {"address":format!("{resource_type}.managed"), "type":resource_type},
            {"address":format!("data.{resource_type}.data"), "type":resource_type, "mode":"data"},
            {"address":format!("{resource_type}.managed"), "type":resource_type, "deposed":"old", "change":{"actions":["delete"]}},
            {"address":format!("{resource_type}.foreign"), "type":resource_type, "provider_name":"example.com/foreign/aws"},
            {"address":format!("{resource_type}_future.unknown"), "type":format!("{resource_type}_future")}
        ]});
        let graph = crate::semantic::transform(&crate::plan::parse(&input.to_string()).unwrap());
        assert_eq!(graph.nodes.len(), 5);
        assert_eq!(graph.coverage.selected.resources, 5);
        for node in &graph.nodes {
            if node.address.ends_with(".foreign") || node.address.ends_with(".unknown") {
                assert_eq!(node.role, ResourceRole::Unknown);
                assert_eq!(node.placement, Placement::Unresolved);
            } else {
                assert_eq!(node.role, ResourceRole::Identity);
                assert_eq!(node.placement, Placement::Logical);
            }
        }
        assert!(graph.edges.is_empty());
        assert_eq!(
            classify(&ProviderIdentity::inferred(resource_type), resource_type),
            ResourceRole::Identity
        );
    }
    for resource_type in [
        "aws_iam_policy",
        "aws_iam_role_policy",
        "aws_iam_user_policy",
        "aws_iam_group_policy",
        "aws_iam_instance_profile",
    ] {
        assert_eq!(
            classify(&ProviderIdentity::inferred(resource_type), resource_type),
            ResourceRole::Policy
        );
    }
    for resource_type in [
        "aws_iam_group_membership",
        "aws_iam_user_group_membership",
        "aws_iam_role_policy_attachment",
        "aws_iam_user_policy_attachment",
        "aws_iam_group_policy_attachment",
    ] {
        assert_eq!(
            classify(&ProviderIdentity::inferred(resource_type), resource_type),
            ResourceRole::Association
        );
    }
}

#[test]
fn dispatches_aws_and_preserves_unknown_provider_types() {
    assert_eq!(
        classify(
            &crate::model::ProviderIdentity::inferred("aws_vpc"),
            "aws_vpc"
        ),
        ResourceRole::Container
    );
    for resource_type in [
        "azurerm_virtual_network",
        "terraform_data",
        "custom_aws_vpc",
        "vpc",
        "aws_unknown",
        "",
    ] {
        assert_eq!(
            classify(
                &crate::model::ProviderIdentity::inferred(resource_type),
                resource_type
            ),
            ResourceRole::Unknown
        );
    }
}
