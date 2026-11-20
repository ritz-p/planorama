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

#[test]
fn codeartifact_requires_current_typed_evidence() {
    crate::configured_service_links::check(
        include_str!("../../fixtures/codeartifact-references-plan.json"),
        crate::plan::parse,
        super::label,
        &[
            ("aws_codeartifact_domain.packages", "encryption_key"),
            (
                "aws_codeartifact_domain_permissions_policy.shared",
                "domain",
            ),
            ("aws_codeartifact_repository.delivery", "domain"),
            (
                "aws_codeartifact_repository.delivery",
                "upstream.repository_name",
            ),
            (
                "aws_codeartifact_repository_permissions_policy.read",
                "repository",
            ),
        ],
        ("aws_kms_key.packages", "arn"),
    );
}

#[test]
fn codebuild_requires_current_typed_evidence() {
    crate::configured_service_links::check(
        include_str!("../../fixtures/codebuild-references-plan.json"),
        crate::plan::parse,
        super::label,
        &[
            ("aws_codebuild_project.build", "service_role"),
            ("aws_codebuild_project.build", "encryption_key"),
            ("aws_codebuild_project.build", "environment.fleet.fleet_arn"),
            ("aws_codebuild_project.build", "vpc_config.vpc_id"),
            ("aws_codebuild_project.build", "vpc_config.subnets"),
            (
                "aws_codebuild_project.build",
                "vpc_config.security_group_ids",
            ),
            ("aws_codebuild_project.build", "artifacts.location"),
            (
                "aws_codebuild_project.build",
                "secondary_artifacts.location",
            ),
            (
                "aws_codebuild_project.build",
                "logs_config.cloudwatch_logs.group_name",
            ),
            (
                "aws_codebuild_project.build",
                "logs_config.s3_logs.location",
            ),
            ("aws_codebuild_fleet.build", "fleet_service_role"),
            ("aws_codebuild_fleet.build", "vpc_config.vpc_id"),
            ("aws_codebuild_fleet.build", "vpc_config.subnets"),
            ("aws_codebuild_fleet.build", "vpc_config.security_group_ids"),
            (
                "aws_codebuild_report_group.build",
                "export_config.s3_destination.bucket",
            ),
            (
                "aws_codebuild_report_group.build",
                "export_config.s3_destination.encryption_key",
            ),
            ("aws_codebuild_resource_policy.build", "resource_arn"),
            ("aws_codebuild_webhook.build", "project_name"),
        ],
        ("aws_iam_role.build", "arn"),
    );
}

#[test]
fn codecatalyst_requires_current_typed_evidence() {
    crate::configured_service_links::check(
        include_str!("../../fixtures/codecatalyst-references-plan.json"),
        crate::plan::parse,
        super::label,
        &[
            ("aws_codecatalyst_dev_environment.editor", "project_name"),
            (
                "aws_codecatalyst_dev_environment.editor",
                "repositories.repository_name",
            ),
            ("aws_codecatalyst_source_repository.app", "project_name"),
        ],
        ("aws_codecatalyst_project.delivery", "name"),
    );
}

#[test]
fn repositories_requires_current_typed_evidence() {
    crate::configured_service_links::check(
        include_str!("../../fixtures/repositories-references-plan.json"),
        crate::plan::parse,
        super::label,
        &[
            (
                "aws_codecommit_approval_rule_template_association.review",
                "approval_rule_template_name",
            ),
            (
                "aws_codecommit_approval_rule_template_association.review",
                "repository_name",
            ),
            ("aws_codecommit_repository.repo", "kms_key_id"),
            (
                "aws_codegurureviewer_repository_association.review",
                "kms_key_details.kms_key_id",
            ),
            (
                "aws_codegurureviewer_repository_association.review",
                "repository.codecommit.name",
            ),
        ],
        ("aws_codecommit_approval_rule_template.review", "name"),
    );
}
