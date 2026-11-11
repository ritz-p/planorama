pub(crate) mod aws;
pub(crate) mod terraform;

pub(crate) fn has_indexed_service_references(
    provider: &crate::model::ProviderIdentity,
    resource: &str,
) -> bool {
    provider.is_aws()
        && (aws::rolesanywhere::resource(resource)
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
        && (aws::rolesanywhere::indexed(resource, attribute)
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
    if entity.provider.is_aws() && entity.mode == crate::model::EntityMode::Data {
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
    } else {
        crate::model::ResourceRole::Unknown
    }
}
pub(crate) fn transform(input: crate::semantic::Input) -> crate::model::Graph {
    aws::transform(input)
}
pub(crate) fn diagnostics(plan: &crate::model::TerraformPlan) -> Vec<crate::model::Diagnostic> {
    aws::diagnostics::collect(plan)
}
