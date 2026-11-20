#[test]
fn configured_links_require_current_typed_evidence() {
    crate::configured_service_links::check(
        include_str!("../../fixtures/resilience-hub-plan.json"),
        crate::plan::parse,
        super::label,
        &[
            ("aws_resiliencehubv2_assertion.sample0", "service_arn"),
            ("aws_resiliencehubv2_input_source.sample2", "service_arn"),
            (
                "aws_resiliencehubv2_service_function.sample3",
                "service_arn",
            ),
            (
                "aws_resiliencehubv2_input_source.sample2",
                "resource_configuration.cfn_stack_arn",
            ),
            (
                "aws_resiliencehubv2_input_source.sample2",
                "resource_configuration.eks.cluster_arn",
            ),
            ("aws_resiliencehubv2_service.sample1", "kms_key_id"),
            ("aws_resiliencehubv2_system.sample7", "kms_key_id"),
            ("aws_resiliencehubv2_policy.sample8", "kms_key_id"),
            ("aws_resiliencehubv2_service.sample1", "policy_arn"),
            ("aws_resiliencehubv2_user_journey.sample9", "policy_arn"),
            (
                "aws_resiliencehubv2_service.sample1",
                "associated_system.system_arn",
            ),
            (
                "aws_resiliencehubv2_service.sample1",
                "associated_system.user_journey_ids",
            ),
            (
                "aws_resiliencehubv2_service.sample1",
                "permission_model.invoker_role_name",
            ),
            (
                "aws_resiliencehubv2_service.sample1",
                "permission_model.cross_account_role.cross_account_role_arn",
            ),
            ("aws_resiliencehubv2_user_journey.sample9", "system_arn"),
        ],
        ("aws_resiliencehubv2_service.sample1", "arn"),
    );
}

#[test]
fn service_reference_requires_the_same_deployment_scope() {
    crate::configured_service_links::check_provider_alias(
        include_str!("../../fixtures/resilience-hub-plan.json"),
        "aws_resiliencehubv2_assertion.sample0",
        "service_arn",
        "aws_resiliencehubv2_service.sample1",
        "Resilience Hub service",
        true,
    );
}
