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

#[test]
fn connections_requires_current_typed_evidence() {
    crate::configured_service_links::check(
        include_str!("../../fixtures/connections-references-plan.json"),
        crate::plan::parse,
        super::label,
        &[
            ("aws_codeconnections_connection.git", "host_arn"),
            ("aws_codeconnections_host.git", "vpc_configuration.vpc_id"),
            (
                "aws_codeconnections_host.git",
                "vpc_configuration.subnet_ids",
            ),
            (
                "aws_codeconnections_host.git",
                "vpc_configuration.security_group_ids",
            ),
            ("aws_codestarconnections_connection.git", "host_arn"),
            (
                "aws_codestarconnections_host.git",
                "vpc_configuration.vpc_id",
            ),
            (
                "aws_codestarconnections_host.git",
                "vpc_configuration.subnet_ids",
            ),
            (
                "aws_codestarconnections_host.git",
                "vpc_configuration.security_group_ids",
            ),
        ],
        ("aws_codeconnections_host.git", "arn"),
    );
}

#[test]
fn codedeploy_requires_current_typed_evidence() {
    crate::configured_service_links::check(
        include_str!("../../fixtures/codedeploy-references-plan.json"),
        crate::plan::parse,
        super::label,
        &[
            ("aws_codedeploy_deployment_group.shipping", "app_name"),
            (
                "aws_codedeploy_deployment_group.shipping",
                "deployment_config_name",
            ),
            (
                "aws_codedeploy_deployment_group.shipping",
                "service_role_arn",
            ),
            (
                "aws_codedeploy_deployment_group.shipping",
                "autoscaling_groups",
            ),
            (
                "aws_codedeploy_deployment_group.shipping",
                "ecs_service.cluster_name",
            ),
            (
                "aws_codedeploy_deployment_group.shipping",
                "ecs_service.service_name",
            ),
            (
                "aws_codedeploy_deployment_group.shipping",
                "load_balancer_info.target_group_pair_info.target_group.name",
            ),
            (
                "aws_codedeploy_deployment_group.shipping",
                "load_balancer_info.target_group_pair_info.prod_traffic_route.listener_arns",
            ),
            (
                "aws_codedeploy_deployment_group.shipping",
                "trigger_configuration.trigger_target_arn",
            ),
        ],
        ("aws_codedeploy_app.shipping", "name"),
    );
}

#[test]
fn pipeline_requires_current_typed_evidence() {
    crate::configured_service_links::check(
        include_str!("../../fixtures/pipeline-references-plan.json"),
        crate::plan::evidence::tests::parse_fixture,
        super::label,
        &[
            ("aws_codepipeline.delivery", "role_arn"),
            ("aws_codepipeline.delivery", "artifact_store.location"),
            (
                "aws_codepipeline.delivery",
                "artifact_store.encryption_key.id",
            ),
            ("aws_codepipeline.delivery", "stage.action.role_arn"),
            ("aws_codepipeline_webhook.delivery", "target_pipeline"),
        ],
        ("aws_iam_role.pipeline", "arn"),
    );
}

#[test]
fn delivery_notifications_requires_current_typed_evidence() {
    crate::configured_service_links::check(
        include_str!("../../fixtures/delivery-notifications-references-plan.json"),
        crate::plan::parse,
        super::label,
        &[
            ("aws_codecommit_trigger.delivery", "repository_name"),
            ("aws_codecommit_trigger.delivery", "trigger.destination_arn"),
            (
                "aws_codestarnotifications_notification_rule.delivery",
                "resource",
            ),
            (
                "aws_codestarnotifications_notification_rule.delivery",
                "target.address",
            ),
        ],
        ("aws_codecommit_repository.delivery", "repository_name"),
    );
}

#[test]
fn devicefarm_requires_current_typed_evidence() {
    crate::configured_service_links::check(
        include_str!("../../fixtures/devicefarm-references-plan.json"),
        crate::plan::parse,
        super::label,
        &[
            ("aws_devicefarm_device_pool.test", "project_arn"),
            ("aws_devicefarm_network_profile.test", "project_arn"),
            ("aws_devicefarm_upload.test", "project_arn"),
            ("aws_devicefarm_test_grid_project.test", "vpc_config.vpc_id"),
            (
                "aws_devicefarm_test_grid_project.test",
                "vpc_config.subnet_ids",
            ),
            (
                "aws_devicefarm_test_grid_project.test",
                "vpc_config.security_group_ids",
            ),
        ],
        ("aws_devicefarm_project.test", "arn"),
    );
}

#[test]
fn devops_insights_requires_current_typed_evidence() {
    crate::configured_service_links::check(
        include_str!("../../fixtures/devops-insights-references-plan.json"),
        crate::plan::parse,
        super::label,
        &[
            ("aws_devopsagent_agent_space.insights", "kms_key_arn"),
            (
                "aws_devopsguru_notification_channel.insights",
                "sns.topic_arn",
            ),
            (
                "aws_devopsguru_resource_collection.insights",
                "cloudformation.stack_names",
            ),
            (
                "aws_devopsguru_service_integration.insights",
                "kms_server_side_encryption.kms_key_id",
            ),
        ],
        ("aws_kms_key.insights", "arn"),
    );
}

#[test]
fn account_governance_requires_current_typed_evidence() {
    crate::configured_service_links::check(
        include_str!("../../fixtures/account-governance-references-plan.json"),
        crate::plan::parse,
        super::label,
        &[
            ("aws_account_alternate_contact.target", "account_id"),
            ("aws_account_primary_contact.target", "account_id"),
            ("aws_account_region.target", "account_id"),
            (
                "aws_cloudtrail_organization_delegated_admin_account.target",
                "account_id",
            ),
            (
                "aws_organizations_delegated_administrator.target",
                "account_id",
            ),
            ("aws_organizations_tag.target", "resource_id"),
        ],
        ("aws_organizations_account.target", "id"),
    );
}

#[test]
fn cloudformation_requires_current_typed_evidence() {
    crate::configured_service_links::check(
        include_str!("../../fixtures/cloudformation-references-plan.json"),
        crate::plan::parse,
        super::label,
        &[
            ("aws_cloudcontrolapi_resource.resource", "role_arn"),
            ("aws_cloudformation_stack.stack", "iam_role_arn"),
            ("aws_cloudformation_stack.stack", "notification_arns"),
            ("aws_cloudformation_type.type", "execution_role_arn"),
            (
                "aws_cloudformation_type.type",
                "logging_config.log_role_arn",
            ),
            (
                "aws_cloudformation_type.type",
                "logging_config.log_group_name",
            ),
        ],
        ("aws_iam_role.template", "arn"),
    );
}

#[test]
fn organizations_tag_accepts_roots_and_rejects_organization_ids() {
    use serde_json::{Value, json};
    for (attribute, value, expected) in [
        ("id", "o-abcdef0123", false),
        ("roots[0].id", "r-abcd", true),
    ] {
        let mut fixture: Value = serde_json::from_str(include_str!(
            "../../fixtures/account-governance-references-plan.json"
        ))
        .unwrap();
        let address = "aws_organizations_tag.target";
        fixture["resource_changes"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|row| row["address"] == address)
            .unwrap()["change"]["after"]["resource_id"] = json!(value);
        let config = fixture["configuration"]["root_module"]["resources"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|row| row["address"] == address)
            .unwrap();
        config["expressions"]["resource_id"]["references"] = json!([format!(
            "aws_organizations_organization.example.{attribute}"
        )]);
        let graph = crate::semantic::transform(&crate::plan::parse(&fixture.to_string()).unwrap());
        assert_eq!(
            crate::configured_service_links::verified(
                &graph,
                address,
                "resource_id",
                super::label("aws_organizations_tag", "resource_id"),
            ),
            expected,
            "{attribute}"
        );
    }
}

#[test]
fn cloud_control_role_keeps_target_placement_unresolved() {
    let fixture = include_str!("../../fixtures/cloudformation-references-plan.json");
    let plan = crate::plan::evidence::tests::parse_fixture(fixture).unwrap();
    let graph = crate::semantic::transform(&plan);
    let address = "aws_cloudcontrolapi_resource.resource";
    let target = graph
        .nodes
        .iter()
        .find(|node| node.address == address)
        .unwrap();
    assert!(
        target
            .provider_configuration
            .as_ref()
            .unwrap()
            .account
            .is_some()
    );
    assert!(target.entity.scope.is_none());
    assert!(crate::configured_service_links::verified(
        &graph,
        address,
        "role_arn",
        super::label("aws_cloudcontrolapi_resource", "role_arn"),
    ));
}
