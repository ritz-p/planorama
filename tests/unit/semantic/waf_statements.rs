#[test]
fn configured_links_require_current_typed_evidence() {
    crate::configured_service_links::check(
        include_str!("../../fixtures/waf-statements-plan.json"),
        crate::plan::parse,
        super::label,
        &[
            ("aws_wafv2_web_acl_rule.sample0", "web_acl_arn"),
            (
                "aws_wafv2_web_acl_rule_group_association.sample2",
                "web_acl_arn",
            ),
            (
                "aws_wafv2_web_acl_rule_group_association.sample2",
                "rule_group_reference.arn",
            ),
            (
                "aws_wafv2_web_acl_rule.sample0",
                "statement.rule_group_reference_statement.arn",
            ),
            (
                "aws_wafv2_web_acl.sample1",
                "rule.statement.ip_set_reference_statement.arn",
            ),
            (
                "aws_wafv2_rule_group.sample3",
                "rule.statement.ip_set_reference_statement.arn",
            ),
            (
                "aws_wafv2_web_acl.sample1",
                "rule.statement.regex_pattern_set_reference_statement.arn",
            ),
            (
                "aws_wafv2_rule_group.sample3",
                "rule.statement.regex_pattern_set_reference_statement.arn",
            ),
            (
                "aws_wafv2_web_acl_rule.ip_set_reference_statement",
                "statement.ip_set_reference_statement.arn",
            ),
            (
                "aws_wafv2_web_acl_rule.regex_pattern_set_reference_statement",
                "statement.regex_pattern_set_reference_statement.arn",
            ),
        ],
        ("aws_wafv2_web_acl.sample1", "arn"),
    );
}

#[test]
fn nested_sets_require_a_verified_parent_scope() {
    use serde_json::{Value, json};
    let mut f: Value =
        serde_json::from_str(include_str!("../../fixtures/waf-statements-plan.json")).unwrap();
    let address = "aws_wafv2_web_acl_rule.ip_set_reference_statement";
    let path = "statement.and_statement.statement.ip_set_reference_statement.arn";
    for row in f["configuration"]["root_module"]["resources"]
        .as_array_mut()
        .unwrap()
    {
        if row["address"] == address {
            let leaf = row["expressions"]["statement"][0].take();
            row["expressions"]["statement"] =
                json!([{"and_statement":[{"statement":[leaf,{"geo_match_statement":[{}]}]}]}]);
        }
    }
    let change = f["resource_changes"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|r| r["address"] == address)
        .unwrap();
    let leaf = change["change"]["after"]["statement"][0].take();
    change["change"]["after"]["statement"] = json!([{"and_statement":[{"statement":[leaf,{"geo_match_statement":[{"country_codes":["CA"]}]}]}]}]);
    for mask in [false, true] {
        let change = f["resource_changes"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|r| r["address"] == address)
            .unwrap();
        change["change"]["after_unknown"] = json!({"web_acl_arn":mask});
        let parsed = crate::plan::parse(&f.to_string()).unwrap();
        assert_eq!(
            crate::configured_service_links::verified(
                &crate::semantic::transform(&parsed),
                address,
                path,
                Some("IP set reference")
            ),
            !mask
        );
    }
}
