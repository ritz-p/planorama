use super::label;
use crate::configured_service_links::{check, check_provider_alias, verified};
use crate::plan::evidence::tests::parse_fixture;
use crate::semantic::transform;
use serde_json::{Value, json};

#[test]
fn cloudtrail_verifies_topic_name_and_event_store_key() {
    let fixture = include_str!("../../fixtures/cloudtrail-references-plan.json");
    let trail = "aws_cloudtrail.audit";
    let store = "aws_cloudtrail_event_data_store.audit";
    for (consumer, path, endpoint, output) in [
        (trail, "sns_topic_name", "aws_sns_topic.trail", "name"),
        (store, "kms_key_id", "aws_kms_key.audit", "arn"),
    ] {
        check(
            fixture,
            parse_fixture,
            label,
            &[(consumer, path)],
            (endpoint, output),
        );
        check_provider_alias(
            fixture,
            consumer,
            path,
            endpoint,
            label(consumer.split('.').next().unwrap(), path).unwrap(),
            true,
        );
    }

    let mut plan: Value = serde_json::from_str(fixture).unwrap();
    let graph = transform(&parse_fixture(&plan.to_string()).unwrap());
    assert!(verified(
        &graph,
        trail,
        "s3_bucket_name",
        Some("Trail bucket reference")
    ));
    assert!(label("aws_cloudtrail", "sns_topic_arn").is_none());
    assert!(label("aws_cloudtrail_event_data_store", "organization_enabled").is_none());
    assert!(label("aws_cloudtrail_event_data_store", "multi_region_enabled").is_none());

    let expressions = &mut plan["configuration"]["root_module"]["resources"][0]["expressions"];
    expressions["sns_topic_name"] = json!({"references": [
        "aws_sns_topic.trail.name",
        "aws_sns_topic.other.name"
    ]});
    let graph = transform(&parse_fixture(&plan.to_string()).unwrap());
    assert!(!verified(
        &graph,
        trail,
        "sns_topic_name",
        Some("Trail notification topic")
    ));
}

#[test]
fn controltower_verifies_configured_endpoints() {
    let fixture = include_str!("../../fixtures/controltower-references-plan.json");
    check(
        fixture,
        parse_fixture,
        label,
        &[
            ("aws_controltower_baseline.workloads", "target_identifier"),
            ("aws_controltower_control.workloads", "target_identifier"),
        ],
        ("aws_organizations_organizational_unit.workloads", "arn"),
    );
    assert!(label("aws_controltower_baseline", "baseline_identifier").is_none());
    assert!(label("aws_controltower_baseline", "operation_identifier").is_none());
    assert!(label("aws_controltower_control", "control_identifier").is_none());
    assert!(label("aws_controltower_control", "arn").is_none());
    check_provider_alias(
        fixture,
        "aws_controltower_baseline.workloads",
        "target_identifier",
        "aws_organizations_organizational_unit.workloads",
        "Control Tower target OU",
        false,
    );
}

#[test]
fn license_grant_acceptance_uses_known_home_region() {
    let fixture = include_str!("../../fixtures/license-manager-references-plan.json");
    for (home_region, unknown, expected) in [
        (json!("us-east-1"), false, true),
        (json!("us-west-2"), false, false),
        (Value::Null, false, false),
        (json!("us-east-1"), true, false),
    ] {
        let mut plan: Value = serde_json::from_str(fixture).unwrap();
        let grant = plan["resource_changes"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|row| row["address"] == "aws_licensemanager_grant.recipient")
            .unwrap();
        grant["change"]["after"]["home_region"] = home_region;
        grant["change"]["after_unknown"]["home_region"] = json!(unknown);
        let graph = transform(&parse_fixture(&plan.to_string()).unwrap());
        assert_eq!(
            verified(
                &graph,
                "aws_licensemanager_grant_accepter.recipient",
                "grant_arn",
                Some("Accepted license grant")
            ),
            expected
        );
    }
}

#[test]
fn license_manager_verifies_configured_endpoints() {
    let fixture = include_str!("../../fixtures/license-manager-references-plan.json");
    check(
        fixture,
        parse_fixture,
        label,
        &[
            (
                "aws_licensemanager_association.host",
                "license_configuration_arn",
            ),
            ("aws_licensemanager_association.host", "resource_arn"),
            ("aws_licensemanager_grant.recipient", "principal"),
            ("aws_licensemanager_grant_accepter.recipient", "grant_arn"),
        ],
        ("aws_licensemanager_license_configuration.cpu", "arn"),
    );
    assert!(label("aws_licensemanager_grant", "license_arn").is_none());
    assert!(label("aws_licensemanager_grant_accepter", "principal").is_none());
    assert!(label("aws_licensemanager_grant_accepter", "status").is_none());
    check_provider_alias(
        fixture,
        "aws_licensemanager_association.host",
        "resource_arn",
        "aws_instance.machine",
        "License assignment target",
        true,
    );
}

#[test]
fn ram_verifies_global_core_network_in_both_associations() {
    let fixture = include_str!("../../fixtures/ram-references-plan.json");
    for (address, path, label) in [
        (
            "aws_ram_resource_association.core",
            "resource_arn",
            "Shared resource",
        ),
        (
            "aws_ram_resource_share_associations_exclusive.acl",
            "resource_arns",
            "Shared resources",
        ),
    ] {
        check_provider_alias(
            fixture,
            address,
            path,
            "aws_networkmanager_core_network.shared",
            label,
            false,
        );
    }
}

#[test]
fn ram_verifies_configured_endpoints() {
    let fixture = include_str!("../../fixtures/ram-references-plan.json");
    check(
        fixture,
        parse_fixture,
        label,
        &[
            (
                "aws_ram_principal_association.account",
                "resource_share_arn",
            ),
            ("aws_ram_principal_association.account", "principal"),
            ("aws_ram_resource_association.subnet", "resource_arn"),
            ("aws_ram_resource_share.standard", "permission_arns"),
            ("aws_ram_resource_share_accepter.recipient", "share_arn"),
            (
                "aws_ram_resource_share_associations_exclusive.acl",
                "resource_share_arn",
            ),
            (
                "aws_ram_resource_share_associations_exclusive.acl",
                "resource_arns",
            ),
            (
                "aws_ram_resource_share_associations_exclusive.acl",
                "principals",
            ),
        ],
        ("aws_ram_resource_share.standard", "arn"),
    );
    assert!(label("aws_ram_resource_share_associations_exclusive", "sources").is_none());
    assert!(label("aws_ram_resource_share", "allow_external_principals").is_none());
    assert!(label("aws_ram_resource_share_accepter", "resources").is_none());
    assert!(label("aws_ram_resource_share_accepter", "sender_account_id").is_none());
    check_provider_alias(
        fixture,
        "aws_ram_resource_association.subnet",
        "resource_arn",
        "aws_subnet.shared",
        "Shared resource",
        true,
    );
}
