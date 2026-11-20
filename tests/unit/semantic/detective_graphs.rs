#[test]
fn configured_links_require_current_typed_evidence() {
    crate::configured_service_links::check(
        include_str!("../../fixtures/detective-graphs-plan.json"),
        crate::plan::evidence::tests::parse_fixture,
        super::label,
        &[
            ("aws_detective_invitation_accepter.sample0", "graph_arn"),
            ("aws_detective_member.sample2", "graph_arn"),
            (
                "aws_detective_organization_configuration.sample3",
                "graph_arn",
            ),
            ("aws_detective_member.sample2", "account_id"),
            (
                "aws_detective_organization_admin_account.sample5",
                "account_id",
            ),
        ],
        ("aws_detective_graph.sample1", "graph_arn"),
    );
}

#[test]
fn invited_graph_requires_verified_bindings_in_the_graph_region() {
    use serde_json::{Value, json};
    let mut fixture: Value =
        serde_json::from_str(include_str!("../../fixtures/detective-graphs-plan.json")).unwrap();
    let check = |v: &Value| {
        let parsed = crate::plan::evidence::tests::parse_fixture(&v.to_string()).unwrap();
        crate::configured_service_links::verified(
            &crate::semantic::transform(&parsed),
            "aws_detective_invitation_accepter.sample0",
            "graph_arn",
            super::label("aws_detective_invitation_accepter", "graph_arn"),
        )
    };
    let cfg = &mut fixture["configuration"];
    cfg["provider_config"]["aws.member"] = cfg["provider_config"]["aws"].clone();
    cfg["provider_config"]["aws.member"]["alias"] = json!("member");
    cfg["provider_config"]["aws.member"]["expressions"]["assume_role"][0]["role_arn"]["constant_value"] =
        json!("arn:aws:iam::210987654321:role/Member");
    cfg["root_module"]["resources"][0]["provider_config_key"] = json!("aws.member");
    assert!(check(&fixture));
    fixture["configuration"]["provider_config"]["aws.member"]["expressions"]["region"]["constant_value"] =
        json!("us-west-2");
    assert!(!check(&fixture));
}

#[test]
fn organization_targets_can_be_managed_in_a_separate_account() {
    crate::configured_service_links::organization_targets(
        include_str!("../../fixtures/detective-graphs-plan.json"),
        super::label,
        &[
            ("aws_detective_member.sample2", "account_id"),
            (
                "aws_detective_organization_admin_account.sample5",
                "account_id",
            ),
        ],
    );
}
