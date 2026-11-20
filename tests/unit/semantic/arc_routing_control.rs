#[test]
fn configured_links_require_current_typed_evidence() {
    crate::configured_service_links::check(
        include_str!("../../fixtures/arc-routing-control-plan.json"),
        crate::plan::parse,
        super::label,
        &[
            (
                "aws_route53recoverycontrolconfig_control_panel.sample0",
                "cluster_arn",
            ),
            (
                "aws_route53recoverycontrolconfig_routing_control.sample2",
                "cluster_arn",
            ),
            (
                "aws_route53recoverycontrolconfig_routing_control.sample2",
                "control_panel_arn",
            ),
            (
                "aws_route53recoverycontrolconfig_safety_rule.sample3",
                "control_panel_arn",
            ),
            (
                "aws_route53recoverycontrolconfig_safety_rule.assertion",
                "asserted_controls",
            ),
            (
                "aws_route53recoverycontrolconfig_safety_rule.sample3",
                "gating_controls",
            ),
            (
                "aws_route53recoverycontrolconfig_safety_rule.sample3",
                "target_controls",
            ),
        ],
        ("aws_route53recoverycontrolconfig_cluster.sample1", "arn"),
    );
}

#[test]
fn global_cluster_uses_the_control_plane_account() {
    crate::configured_service_links::check_provider_alias(
        include_str!("../../fixtures/arc-routing-control-plan.json"),
        "aws_route53recoverycontrolconfig_control_panel.sample0",
        "cluster_arn",
        "aws_route53recoverycontrolconfig_cluster.sample1",
        "Routing control cluster",
        false,
    );
}
