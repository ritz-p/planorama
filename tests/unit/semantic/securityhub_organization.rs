#[test]
fn configured_links_require_current_typed_evidence() {
    crate::configured_service_links::check(
        include_str!("../../fixtures/securityhub-organization-plan.json"),
        crate::plan::parse,
        super::label,
        &[
            (
                "aws_securityhub_configuration_policy_association.sample0",
                "policy_id",
            ),
            (
                "aws_securityhub_configuration_policy_association.sample0",
                "target_id",
            ),
            ("aws_securityhub_invite_accepter.sample3", "master_id"),
            ("aws_securityhub_member.sample4", "account_id"),
            (
                "aws_securityhub_organization_admin_account.sample5",
                "admin_account_id",
            ),
        ],
        ("aws_securityhub_configuration_policy.sample1", "id"),
    );
}

#[test]
fn self_managed_policy_is_never_a_managed_endpoint() {
    use serde_json::{Value, json};
    let mut fixture: Value = serde_json::from_str(include_str!(
        "../../fixtures/securityhub-organization-plan.json"
    ))
    .unwrap();
    let check = |v: &Value| {
        let parsed = crate::plan::parse(&v.to_string()).unwrap();
        crate::configured_service_links::verified(
            &crate::semantic::transform(&parsed),
            "aws_securityhub_configuration_policy_association.sample0",
            "policy_id",
            super::label(
                "aws_securityhub_configuration_policy_association",
                "policy_id",
            ),
        )
    };

    assert!(check(&fixture));
    fixture["resource_changes"][0]["change"]["after"]["policy_id"] =
        json!("SELF_MANAGED_SECURITY_HUB");
    fixture["resource_changes"][1]["change"]["after"]["id"] = json!("SELF_MANAGED_SECURITY_HUB");
    assert!(!check(&fixture));
}

#[test]
fn organization_targets_can_be_managed_in_a_separate_account() {
    crate::configured_service_links::organization_targets(
        include_str!("../../fixtures/securityhub-organization-plan.json"),
        super::label,
        &[
            (
                "aws_securityhub_configuration_policy_association.sample0",
                "target_id",
            ),
            ("aws_securityhub_invite_accepter.sample3", "master_id"),
            ("aws_securityhub_member.sample4", "account_id"),
            (
                "aws_securityhub_organization_admin_account.sample5",
                "admin_account_id",
            ),
        ],
    );
}

#[test]
fn verified_policy_target_preserves_the_specific_organization_root() {
    use serde_json::{Value, json};
    let mut v: Value = serde_json::from_str(include_str!(
        "../../fixtures/securityhub-organization-plan.json"
    ))
    .unwrap();
    let cfg = &mut v["configuration"]["root_module"]["resources"];
    let org = "aws_organizations_organization.main";
    cfg[0]["expressions"]["target_id"] = json!({"references":[format!("{org}.roots[1].id"),format!("{org}.roots[1]"),format!("{org}.roots"),org]});
    cfg.as_array_mut().unwrap().push(json!({"address":org,"mode":"managed","type":"aws_organizations_organization","name":"main","provider_config_key":"aws","expressions":{}}));
    let changes = &mut v["resource_changes"];
    changes[0]["change"]["after"]["target_id"] = json!("r-second");
    let mut row = changes[2].clone();
    row["address"] = json!(org);
    row["name"] = json!("main");
    row["type"] = json!("aws_organizations_organization");
    row["change"]["after"] = json!({"roots":[{"id":"r-first"},{"id":"r-second"}]});
    changes.as_array_mut().unwrap().push(row);
    let check = |v: &Value| {
        let graph = crate::semantic::transform(&crate::plan::parse(&v.to_string()).unwrap());
        graph
            .nodes
            .iter()
            .find(|n| n.address == "aws_securityhub_configuration_policy_association.sample0")
            .unwrap()
            .references
            .iter()
            .any(|r| {
                r.attribute == "target_id"
                    && r.complete
                    && r.kind == crate::model::ReferenceKind::OrganizationRoot(1)
            })
    };
    assert!(check(&v));
    v["resource_changes"][0]["change"]["after"]["target_id"] = json!("r-first");
    assert!(!check(&v));
}
