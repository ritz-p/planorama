#[test]
fn cloud9_requires_current_typed_evidence() {
    crate::configured_service_links::check(
        include_str!("../../fixtures/cloud9-references-plan.json"),
        crate::plan::parse,
        super::label,
        &[
            ("aws_cloud9_environment_ec2.workspace", "subnet_id"),
            ("aws_cloud9_environment_ec2.workspace", "owner_arn"),
            ("aws_cloud9_environment_membership.member", "environment_id"),
            ("aws_cloud9_environment_membership.member", "user_arn"),
        ],
        ("aws_subnet.workspace", "id"),
    );
}

#[test]
fn cloud9_qualifies_provider_scope() {
    let (consumer, path) = ("aws_cloud9_environment_ec2.workspace", "subnet_id");
    crate::configured_service_links::check_provider_alias(
        include_str!("../../fixtures/cloud9-references-plan.json"),
        consumer,
        path,
        "aws_subnet.workspace",
        super::label(consumer.split('.').next().unwrap(), path).unwrap(),
        true,
    );
}
