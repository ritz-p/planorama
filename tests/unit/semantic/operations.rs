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

#[test]
fn resource_groups_verifies_configured_endpoints() {
    let fixture = include_str!("../../fixtures/resource-groups-references-plan.json");
    check(
        fixture,
        parse_fixture,
        label,
        &[
            ("aws_resourcegroups_resource.workloads", "group_arn"),
            ("aws_resourcegroups_resource.workloads", "resource_arn"),
            ("aws_resourceexplorer2_view.workloads", "scope"),
            (
                "aws_servicecatalogappregistry_attribute_group_association.workloads",
                "application_id",
            ),
            (
                "aws_servicecatalogappregistry_attribute_group_association.workloads",
                "attribute_group_id",
            ),
        ],
        ("aws_resourcegroups_group.workloads", "arn"),
    );
    assert!(label("aws_resourcegroups_resource", "resource_type").is_none());
    assert!(label("aws_resourceexplorer2_view", "default_view").is_none());
    assert!(label("aws_resourceexplorer2_view", "filters.filter_string").is_none());
    check_provider_alias(
        fixture,
        "aws_servicecatalogappregistry_attribute_group_association.workloads",
        "attribute_group_id",
        "aws_servicecatalogappregistry_attribute_group.shared",
        "AppRegistry attribute group",
        true,
    );
}

#[test]
fn service_catalog_budget_uses_effective_account() {
    let fixture = include_str!("../../fixtures/service-catalog-associations-references-plan.json");
    for (account_id, unknown, expected) in [
        (Value::Null, false, true),
        (json!("123456789012"), false, true),
        (json!("210987654321"), false, false),
        (json!("123456789012"), true, false),
    ] {
        let mut plan: Value = serde_json::from_str(fixture).unwrap();
        let budget = plan["resource_changes"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|row| row["address"] == "aws_budgets_budget.main")
            .unwrap();
        budget["change"]["after"]["account_id"] = account_id;
        budget["change"]["after_unknown"]["account_id"] = json!(unknown);
        let graph = transform(&parse_fixture(&plan.to_string()).unwrap());
        assert_eq!(
            verified(
                &graph,
                "aws_servicecatalog_budget_resource_association.main",
                "budget_name",
                Some("Associated budget")
            ),
            expected
        );
    }
}

#[test]
fn service_catalog_associations_verifies_configured_endpoints() {
    let fixture = include_str!("../../fixtures/service-catalog-associations-references-plan.json");
    check(
        fixture,
        parse_fixture,
        label,
        &[
            ("aws_servicecatalog_constraint.main", "portfolio_id"),
            ("aws_servicecatalog_constraint.main", "product_id"),
            (
                "aws_servicecatalog_budget_resource_association.main",
                "budget_name",
            ),
            (
                "aws_servicecatalog_budget_resource_association.main",
                "resource_id",
            ),
            ("aws_servicecatalog_portfolio_share.main", "portfolio_id"),
            ("aws_servicecatalog_portfolio_share.main", "principal_id"),
            (
                "aws_servicecatalog_principal_portfolio_association.main",
                "portfolio_id",
            ),
            (
                "aws_servicecatalog_principal_portfolio_association.main",
                "principal_arn",
            ),
            (
                "aws_servicecatalog_product_portfolio_association.main",
                "portfolio_id",
            ),
            (
                "aws_servicecatalog_product_portfolio_association.main",
                "product_id",
            ),
            (
                "aws_servicecatalog_product_portfolio_association.main",
                "source_portfolio_id",
            ),
            (
                "aws_servicecatalog_tag_option_resource_association.main",
                "tag_option_id",
            ),
            (
                "aws_servicecatalog_tag_option_resource_association.main",
                "resource_id",
            ),
        ],
        ("aws_servicecatalog_portfolio.main", "id"),
    );
    assert!(label("aws_servicecatalog_constraint", "parameters").is_none());
    assert!(label("aws_servicecatalog_portfolio_share", "accepted").is_none());
    assert!(
        label(
            "aws_servicecatalog_tag_option_resource_association",
            "resource_arn"
        )
        .is_none()
    );
    check_provider_alias(
        fixture,
        "aws_servicecatalog_constraint.main",
        "portfolio_id",
        "aws_servicecatalog_portfolio.main",
        "Service Catalog portfolio",
        true,
    );
    let mut plan: Value = serde_json::from_str(fixture).unwrap();
    plan["resource_changes"][2]["change"]["after"]["type"] = json!("ORGANIZATION");
    plan["resource_changes"][3]["change"]["after"]["principal_type"] = json!("IAM_PATTERN");
    let graph = transform(&parse_fixture(&plan.to_string()).unwrap());
    assert!(!verified(
        &graph,
        "aws_servicecatalog_portfolio_share.main",
        "principal_id",
        Some("Portfolio share recipient")
    ));
    assert!(!verified(
        &graph,
        "aws_servicecatalog_principal_portfolio_association.main",
        "principal_arn",
        Some("Portfolio IAM principal")
    ));
}

#[test]
fn service_catalog_provisioning_verifies_configured_endpoints() {
    let fixture = include_str!("../../fixtures/service-catalog-provisioning-references-plan.json");
    check(
        fixture,
        parse_fixture,
        label,
        &[
            ("aws_servicecatalog_provisioned_product.id", "product_id"),
            (
                "aws_servicecatalog_provisioned_product.id",
                "provisioning_artifact_id",
            ),
            (
                "aws_servicecatalog_provisioned_product.id",
                "notification_arns",
            ),
            (
                "aws_servicecatalog_provisioned_product.name",
                "product_name",
            ),
            (
                "aws_servicecatalog_provisioned_product.name",
                "provisioning_artifact_name",
            ),
            (
                "aws_servicecatalog_provisioning_artifact.main",
                "product_id",
            ),
            ("aws_servicecatalog_service_action.main", "definition.name"),
            (
                "aws_servicecatalog_service_action.main",
                "definition.assume_role",
            ),
        ],
        ("aws_servicecatalog_product.main", "id"),
    );
    assert!(label("aws_servicecatalog_provisioned_product", "outputs").is_none());
    assert!(label("aws_servicecatalog_provisioned_product", "launch_role_arn").is_none());
    assert!(label("aws_servicecatalog_provisioning_artifact", "template_url").is_none());
    assert!(label("aws_servicecatalog_service_action", "definition.version").is_none());
    check_provider_alias(
        fixture,
        "aws_servicecatalog_provisioning_artifact.main",
        "product_id",
        "aws_servicecatalog_product.main",
        "Artifact product",
        true,
    );
    let mut plan: Value = serde_json::from_str(fixture).unwrap();
    plan["resource_changes"][0]["change"]["after"]["product_id"] = json!("prod-456");
    plan["configuration"]["root_module"]["resources"][0]["expressions"]["product_id"] =
        json!({"references":["aws_servicecatalog_product.other.id"]});
    plan["resource_changes"][3]["change"]["after"]["definition"][0]["assume_role"] =
        json!("LAUNCH_ROLE");
    let graph = transform(&parse_fixture(&plan.to_string()).unwrap());
    assert!(!verified(
        &graph,
        "aws_servicecatalog_provisioned_product.id",
        "provisioning_artifact_id",
        Some("Provisioned product artifact")
    ));
    assert!(!verified(
        &graph,
        "aws_servicecatalog_service_action.main",
        "definition.assume_role",
        Some("Service action role")
    ));
}
