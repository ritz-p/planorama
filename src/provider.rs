pub(crate) mod aws;
pub(crate) mod cloudflare;
pub(crate) mod terraform;

pub(crate) fn has_indexed_service_references(
    provider: &crate::model::ProviderIdentity,
    resource: &str,
) -> bool {
    provider.is_aws()
        && (resource == "aws_dsql_cluster"
            || aws::emr::resource(resource)
            || aws::dms_serverless::resource(resource)
            || aws::dms_migration::resource(resource)
            || aws::redshift_helpers::has_indexed(resource)
            || aws::dms_events::resource(resource)
            || aws::dms_references::resource(resource)
            || aws::agentcore_services::resource(resource)
            || aws::agentcore_gateway::resource(resource)
            || aws::agentcore_runtime::resource(resource)
            || aws::datazone::resource(resource)
            || aws::cognito::resource(resource)
            || aws::directory_service::resource(resource)
            || aws::network_monitors::resource(resource)
            || aws::networkfirewall_extended::resource(resource)
            || aws::imagebuilder_distribution::resource(resource)
            || aws::imagebuilder::resource(resource)
            || aws::transfer_webapp::resource(resource)
            || aws::transfer_as2::resource(resource)
            || aws::rolesanywhere::resource(resource)
            || aws::bedrock_agents::resource(resource)
            || aws::bedrock_models::resource(resource)
            || aws::backup_restore::resource(resource)
            || aws::quicksight::resource(resource)
            || aws::sagemaker_studio::resource(resource)
            || aws::sagemaker_jobs::resource(resource)
            || aws::sagemaker_catalog::resource(resource)
            || aws::sagemaker_workforce::resource(resource)
            || aws::msk_connect::resource(resource)
            || aws::msk_delivery::resource(resource)
            || aws::firehose_sources::resource(resource)
            || aws::timestream_influx::resource(resource)
            || aws::timestream_queries::resource(resource)
            || aws::athena_queries::resource(resource)
            || aws::glue_encryption::resource(resource)
            || aws::security_lake::resource(resource)
            || aws::securityhub_integrations::resource(resource)
            || aws::securityhub_organization::resource(resource)
            || aws::macie_classification::resource(resource)
            || aws::inspector_assessments::resource(resource)
            || aws::guardduty_destinations::resource(resource)
            || aws::guardduty_members::resource(resource)
            || aws::detective_graphs::resource(resource)
            || aws::config_delivery::resource(resource)
            || aws::config_rules::resource(resource)
            || aws::config_aggregation::resource(resource)
            || aws::audit_manager::resource(resource)
            || aws::access_analyzer::resource(resource)
            || aws::verified_access::resource(resource)
            || aws::transit_multicast::resource(resource)
            || aws::transit_policies::resource(resource)
            || aws::transit_routes::resource(resource)
            || aws::transit_attachments::resource(resource)
            || aws::client_vpn::resource(resource)
            || aws::local_gateways::resource(resource)
            || aws::site_vpn::resource(resource)
            || aws::template_launch::resource(resource)
            || aws::instance_launch::resource(resource)
            || aws::ec2_fleets::resource(resource)
            || aws::ec2_capacity::resource(resource)
            || aws::ebs_provenance::resource(resource)
            || aws::ec2_images::resource(resource)
            || aws::finspace_kx_configuration::resource(resource)
            || aws::ai_service_configuration::resource(resource)
            || aws::datapipeline_configuration::resource(resource)
            || aws::dataexchange_configuration::resource(resource)
            || aws::cleanrooms_configuration::resource(resource)
            || aws::ecr_configuration::resource(resource)
            || aws::glacier_configuration::resource(resource)
            || aws::flink_configuration::resource(resource)
            || aws::mwaa_configuration::resource(resource)
            || aws::transfer_workflow_configuration::resource(resource)
            || aws::lightsail_configuration::resource(resource)
            || aws::recovery_configuration::resource(resource)
            || aws::batch_configuration::resource(resource)
            || aws::signer_configuration::resource(resource)
            || aws::paymentcryptography_configuration::resource(resource)
            || aws::cloudhsm_configuration::resource(resource)
            || aws::accountaccess_configuration::resource(resource)
            || aws::verifiedpermissions::resource(resource)
            || aws::network_insights::resource(resource)
            || aws::appmesh_configuration::resource(resource)
            || aws::keyspaces_configuration::resource(resource)
            || aws::dynamodb_transfer::resource(resource)
            || aws::lustre_sources::resource(resource)
            || aws::storagegateway::resource(resource)
            || aws::kendra::resource(resource)
            || aws::qbusiness::resource(resource)
            || aws::beanstalk_configuration::resource(resource)
            || aws::apprunner_configuration::resource(resource)
            || aws::cloudwatch_omni::resource(resource)
            || aws::observability_logs::resource(resource)
            || aws::observability_monitors::resource(resource)
            || aws::lakeformation_selectors::resource(resource)
            || aws::backup_reports::resource(resource)
            || aws::private_ca_references::resource(resource)
            || aws::glue_workloads::resource(resource)
            || aws::glue_catalog::resource(resource)
            || aws::s3vectors::resource(resource)
            || aws::vpclattice::resource(resource)
            || aws::networkmanager::resource(resource)
            || aws::transfer_server::resource(resource)
            || aws::classic_elb::resource(resource)
            || aws::synthetics::resource(resource)
            || aws::rum::resource(resource)
            || aws::s3tables::resource(resource)
            || aws::secrets_manager::resource(resource)
            || aws::cloudfront::resource(resource)
            || aws::cloudfront_security::resource(resource)
            || aws::elbv2_cognito::block(resource).is_some()
            || aws::s3::indexed_resource(resource)
            || resource == "aws_networkfirewall_logging_configuration"
            || matches!(resource, "aws_appsync_graphql_api" | "aws_appsync_api")
            || resource == "aws_s3control_object_lambda_access_point"
            || matches!(
                resource,
                "aws_s3control_multi_region_access_point"
                    | "aws_s3control_multi_region_access_point_routes"
            )
            || aws::elbv2_network::resource(resource)
            || matches!(
                resource,
                "aws_route53_zone"
                    | "aws_route53_records_exclusive"
                    | "aws_route53_resolver_endpoint"
            ))
}

pub(crate) fn indexed_service_reference(
    provider: &crate::model::ProviderIdentity,
    resource: &str,
    attribute: &str,
) -> bool {
    provider.is_aws()
        && ((resource == "aws_dsql_cluster" && attribute == "multi_region_properties.0.clusters")
            || aws::emr::indexed(resource, attribute)
            || aws::dms_serverless::indexed(resource, attribute)
            || aws::dms_migration::indexed(resource, attribute)
            || aws::redshift_helpers::indexed(resource, attribute)
            || aws::dms_events::indexed(resource, attribute)
            || aws::dms_references::indexed(resource, attribute)
            || aws::agentcore_services::indexed(resource, attribute)
            || aws::agentcore_gateway::indexed(resource, attribute)
            || aws::agentcore_runtime::indexed(resource, attribute)
            || aws::datazone::indexed(resource, attribute)
            || aws::cognito::indexed(resource, attribute)
            || aws::directory_service::indexed(resource, attribute)
            || aws::network_monitors::indexed(resource, attribute)
            || aws::networkfirewall_extended::indexed(resource, attribute)
            || aws::imagebuilder_distribution::indexed(resource, attribute)
            || aws::imagebuilder::indexed(resource, attribute)
            || aws::transfer_webapp::indexed(resource, attribute)
            || aws::transfer_as2::indexed(resource, attribute)
            || aws::rolesanywhere::indexed(resource, attribute)
            || aws::bedrock_agents::indexed(resource, attribute)
            || aws::bedrock_models::indexed(resource, attribute)
            || aws::backup_restore::indexed(resource, attribute)
            || aws::quicksight::indexed(resource, attribute)
            || aws::sagemaker_studio::indexed(resource, attribute)
            || aws::sagemaker_jobs::indexed(resource, attribute)
            || aws::sagemaker_catalog::indexed(resource, attribute)
            || aws::sagemaker_workforce::indexed(resource, attribute)
            || aws::msk_connect::indexed(resource, attribute)
            || aws::msk_delivery::indexed(resource, attribute)
            || aws::firehose_sources::indexed(resource, attribute)
            || aws::timestream_influx::indexed(resource, attribute)
            || aws::timestream_queries::indexed(resource, attribute)
            || aws::athena_queries::indexed(resource, attribute)
            || aws::glue_encryption::indexed(resource, attribute)
            || aws::security_lake::indexed(resource, attribute)
            || aws::securityhub_integrations::indexed(resource, attribute)
            || aws::securityhub_organization::indexed(resource, attribute)
            || aws::macie_classification::indexed(resource, attribute)
            || aws::inspector_assessments::indexed(resource, attribute)
            || aws::guardduty_destinations::indexed(resource, attribute)
            || aws::guardduty_members::indexed(resource, attribute)
            || aws::detective_graphs::indexed(resource, attribute)
            || aws::config_delivery::indexed(resource, attribute)
            || aws::config_rules::indexed(resource, attribute)
            || aws::config_aggregation::indexed(resource, attribute)
            || aws::audit_manager::indexed(resource, attribute)
            || aws::access_analyzer::indexed(resource, attribute)
            || aws::verified_access::indexed(resource, attribute)
            || aws::transit_multicast::indexed(resource, attribute)
            || aws::transit_policies::indexed(resource, attribute)
            || aws::transit_routes::indexed(resource, attribute)
            || aws::transit_attachments::indexed(resource, attribute)
            || aws::client_vpn::indexed(resource, attribute)
            || aws::local_gateways::indexed(resource, attribute)
            || aws::site_vpn::indexed(resource, attribute)
            || aws::template_launch::indexed(resource, attribute)
            || aws::instance_launch::indexed(resource, attribute)
            || aws::ec2_fleets::indexed(resource, attribute)
            || aws::ec2_capacity::indexed(resource, attribute)
            || aws::ebs_provenance::indexed(resource, attribute)
            || aws::ec2_images::indexed(resource, attribute)
            || aws::finspace_kx_configuration::indexed(resource, attribute)
            || aws::ai_service_configuration::indexed(resource, attribute)
            || aws::datapipeline_configuration::indexed(resource, attribute)
            || aws::dataexchange_configuration::indexed(resource, attribute)
            || aws::cleanrooms_configuration::indexed(resource, attribute)
            || aws::ecr_configuration::indexed(resource, attribute)
            || aws::glacier_configuration::indexed(resource, attribute)
            || aws::flink_configuration::indexed(resource, attribute)
            || aws::mwaa_configuration::indexed(resource, attribute)
            || aws::transfer_workflow_configuration::indexed(resource, attribute)
            || aws::lightsail_configuration::indexed(resource, attribute)
            || aws::recovery_configuration::indexed(resource, attribute)
            || aws::batch_configuration::indexed(resource, attribute)
            || aws::signer_configuration::indexed(resource, attribute)
            || aws::paymentcryptography_configuration::indexed(resource, attribute)
            || aws::cloudhsm_configuration::indexed(resource, attribute)
            || aws::accountaccess_configuration::indexed(resource, attribute)
            || aws::verifiedpermissions::indexed(resource, attribute)
            || aws::network_insights::indexed(resource, attribute)
            || aws::appmesh_configuration::indexed(resource, attribute)
            || aws::keyspaces_configuration::indexed(resource, attribute)
            || aws::dynamodb_transfer::indexed(resource, attribute)
            || aws::lustre_sources::indexed(resource, attribute)
            || aws::storagegateway::indexed(resource, attribute)
            || aws::kendra::indexed(resource, attribute)
            || aws::qbusiness::indexed(resource, attribute)
            || aws::beanstalk_configuration::indexed(resource, attribute)
            || aws::apprunner_configuration::indexed(resource, attribute)
            || aws::cloudwatch_omni::indexed(resource, attribute)
            || aws::observability_logs::indexed(resource, attribute)
            || aws::observability_monitors::indexed(resource, attribute)
            || aws::lakeformation_selectors::indexed(resource, attribute)
            || aws::backup_reports::indexed(resource, attribute)
            || aws::private_ca_references::indexed(resource, attribute)
            || aws::glue_workloads::indexed(resource, attribute)
            || aws::glue_catalog::indexed(resource, attribute)
            || aws::s3vectors::indexed(resource, attribute)
            || aws::vpclattice::indexed(resource, attribute)
            || aws::networkmanager::indexed(resource, attribute)
            || aws::transfer_server::indexed(resource, attribute)
            || aws::classic_elb::indexed(resource, attribute)
            || aws::synthetics::indexed(resource, attribute)
            || aws::rum::indexed(resource, attribute)
            || aws::s3tables::indexed(resource, attribute)
            || aws::secrets_manager::indexed(resource, attribute)
            || aws::cloudfront::indexed(resource, attribute)
            || aws::cloudfront_security::indexed(resource, attribute)
            || aws::appsync::indexed(resource, attribute)
            || aws::appsync_events::indexed(resource, attribute)
            || aws::s3_mrap::indexed(resource, attribute)
            || aws::s3_access::indexed(resource, attribute)
            || aws::elbv2_cognito::indexed(resource, attribute)
            || aws::s3::indexed_attribute(resource, attribute).is_some()
            || aws::networkfirewall::indexed_attribute(resource, attribute)
            || aws::elbv2_network::indexed(resource, attribute)
            || aws::route53::indexed(resource, attribute)
            || aws::resolver::indexed(resource, attribute)
            || aws::route53_records::indexed(resource, attribute))
}

pub(crate) fn reference_label(
    provider: &crate::model::ProviderIdentity,
    resource_type: &str,
    attribute: &str,
    dependency: bool,
) -> &'static str {
    if dependency {
        "Dependency"
    } else if provider.is_aws() {
        aws::reference_label(resource_type, attribute)
    } else {
        "References"
    }
}

pub(crate) fn placement(
    provider: &crate::model::ProviderIdentity,
    resource_type: &str,
) -> crate::model::Placement {
    if provider.is_aws() {
        aws::placement(resource_type)
    } else if provider.is_cloudflare() {
        cloudflare::placement(resource_type, crate::model::EntityMode::Managed)
    } else {
        crate::model::Placement::Unresolved
    }
}

pub(crate) fn scope(
    entity: &crate::model::TerraformEntity,
) -> Option<crate::model::architecture::DeploymentScope> {
    entity
        .provider
        .is_aws()
        .then(|| aws::scope(entity))
        .flatten()
}

pub(crate) fn entity_placement(entity: &crate::model::TerraformEntity) -> crate::model::Placement {
    if entity.provider.is_cloudflare() {
        cloudflare::placement(&entity.resource_type, entity.mode)
    } else if entity.provider.is_aws() && entity.mode == crate::model::EntityMode::Data {
        aws::data_placement(&entity.resource_type)
    } else {
        placement(&entity.provider, &entity.resource_type)
    }
}

pub(crate) fn classify(
    provider: &crate::model::ProviderIdentity,
    resource_type: &str,
) -> crate::model::ResourceRole {
    if provider.is_aws() {
        aws::classify(resource_type)
    } else if provider.is_cloudflare() {
        cloudflare::classify(resource_type, crate::model::EntityMode::Managed)
    } else {
        crate::model::ResourceRole::Unknown
    }
}
pub(crate) fn transform(input: crate::semantic::Input) -> crate::model::Graph {
    aws::transform(input)
}
pub(crate) fn diagnostics(plan: &crate::model::TerraformPlan) -> Vec<crate::model::Diagnostic> {
    let mut diagnostics = aws::diagnostics::collect(plan);
    diagnostics.extend(cloudflare::diagnostics(plan));
    diagnostics
}
