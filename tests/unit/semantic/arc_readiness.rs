#[test]
fn configured_links_require_current_typed_evidence() {
    crate::configured_service_links::check(
        include_str!("../../fixtures/arc-readiness-plan.json"),
        crate::plan::evidence::tests::parse_fixture,
        super::label,
        &[
            (
                "aws_route53recoveryreadiness_readiness_check.sample0",
                "resource_set_name",
            ),
            ("aws_route53recoveryreadiness_cell.sample2", "cells"),
            (
                "aws_route53recoveryreadiness_recovery_group.sample3",
                "cells",
            ),
            (
                "aws_route53recoveryreadiness_resource_set.sample1",
                "resources.resource_arn",
            ),
            (
                "aws_route53recoveryreadiness_resource_set.sample1",
                "resources.readiness_scopes",
            ),
            (
                "aws_route53recoveryreadiness_resource_set.dns",
                "resources.dns_target_resource.hosted_zone_arn",
            ),
            (
                "aws_route53recoveryreadiness_resource_set.dns",
                "resources.dns_target_resource.target_resource.nlb_resource.arn",
            ),
        ],
        (
            "aws_route53recoveryreadiness_resource_set.sample1",
            "resource_set_name",
        ),
    );
}

#[test]
fn readiness_supports_application_and_network_load_balancers() {
    use serde_json::{Value, json};
    let mut f: Value =
        serde_json::from_str(include_str!("../../fixtures/arc-readiness-plan.json")).unwrap();
    let set = "aws_route53recoveryreadiness_resource_set.sample1";
    let lb = "aws_alb.sample6";
    let arn = f["resource_changes"][6]["change"]["after"]["arn"].clone();
    f["resource_changes"][1]["change"]["after"]["resource_set_type"] =
        json!("AWS::ElasticLoadBalancingV2::LoadBalancer");
    f["resource_changes"][1]["change"]["after"]["resources"][0]["resource_arn"] = arn;
    f["configuration"]["root_module"]["resources"][1]["expressions"]["resources"][0]["resource_arn"] =
        json!({"references":[format!("{lb}.arn")]});
    f["resource_changes"][6]["type"] = json!("aws_alb");
    f["resource_changes"][6]["address"] = json!(lb);
    f["configuration"]["root_module"]["resources"][6]["type"] = json!("aws_alb");
    f["configuration"]["root_module"]["resources"][6]["address"] = json!(lb);
    for (kind, expected) in [("network", true), ("application", true), ("gateway", false)] {
        f["resource_changes"][6]["change"]["after"]["load_balancer_type"] = json!(kind);
        let parsed = crate::plan::parse(&f.to_string()).unwrap();
        assert_eq!(
            crate::configured_service_links::verified(
                &crate::semantic::transform(&parsed),
                set,
                "resources.resource_arn",
                Some("Readiness resource")
            ),
            expected
        );
    }
}
