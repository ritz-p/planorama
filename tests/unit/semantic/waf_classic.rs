#[test]
fn configured_links_require_current_typed_evidence() {
    crate::configured_service_links::check(
        include_str!("../../fixtures/waf-classic-plan.json"),
        crate::plan::parse,
        super::label,
        &[
            ("aws_waf_rule.sample0", "predicates.data_id"),
            ("aws_waf_rate_based_rule.sample2", "predicates.data_id"),
            (
                "aws_waf_regex_match_set.sample3",
                "regex_match_tuple.regex_pattern_set_id",
            ),
            ("aws_waf_rule_group.sample5", "activated_rule.rule_id"),
            ("aws_waf_web_acl.sample6", "rules.rule_id"),
            (
                "aws_waf_web_acl.sample6",
                "logging_configuration.log_destination",
            ),
            ("aws_wafregional_rule.sample8", "predicate.data_id"),
            (
                "aws_wafregional_rate_based_rule.sample10",
                "predicate.data_id",
            ),
            (
                "aws_wafregional_regex_match_set.sample11",
                "regex_match_tuple.regex_pattern_set_id",
            ),
            (
                "aws_wafregional_rule_group.sample13",
                "activated_rule.rule_id",
            ),
            ("aws_wafregional_web_acl.sample14", "rule.rule_id"),
            (
                "aws_wafregional_web_acl.sample14",
                "logging_configuration.log_destination",
            ),
            ("aws_wafregional_web_acl_association.sample15", "web_acl_id"),
            (
                "aws_wafregional_web_acl_association.sample15",
                "resource_arn",
            ),
        ],
        ("aws_waf_byte_match_set.sample1", "id"),
    );
}

#[test]
fn each_reference_requires_its_own_predicate_or_activated_rule_type() {
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("../../fixtures/waf-classic-plan.json")).unwrap();
    for (i, block, wrong, path) in [
        (0, "predicates", "IPMatch", "predicates.data_id"),
        (5, "activated_rule", "RATE_BASED", "activated_rule.rule_id"),
        (14, "rule", "GROUP", "rule.rule_id"),
    ] {
        let mut value = fixture.clone();
        let row = &mut value["resource_changes"][i];
        row["change"]["after"][block][0]["type"] = serde_json::json!(wrong);
        let address = row["address"].as_str().unwrap().to_owned();
        let label = super::label(row["type"].as_str().unwrap(), path);
        let parsed = crate::plan::parse(&value.to_string()).unwrap();
        assert!(!crate::configured_service_links::verified(
            &crate::semantic::transform(&parsed),
            &address,
            path,
            label
        ));
    }
}
