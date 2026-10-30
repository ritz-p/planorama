use crate::model::{
    Action, AttributeReference, Diagnostic, DiagnosticReason, EntityMode, Graph, ReferenceTarget,
    ResourceReference, TerraformEntity,
};
use crate::semantic::Input;
use serde_json::Value;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/service_references.rs"]
pub(super) mod tests;

pub(super) fn label(resource: &str, attribute: &str) -> Option<&'static str> {
    if let Some(label) = super::cloudfront::reference_label(resource, attribute) {
        return Some(label);
    }
    if let Some(label) = super::lambda_events::label(resource, attribute) {
        return Some(label);
    }
    if let Some(label) = super::lambda_config::label(resource, attribute) {
        return Some(label);
    }
    if let Some(label) = super::elbv2_cognito::label(resource, attribute) {
        return Some(label);
    }
    if let Some(label) = super::elbv2_network::label(resource, attribute) {
        return Some(label);
    }
    if let Some(label) = super::elbv2_tls::label(resource, attribute) {
        return Some(label);
    }
    if let Some(label) = super::elbv2::label(resource, attribute) {
        return Some(label);
    }
    if let Some(label) = super::security_group_rules::label(resource, attribute) {
        return Some(label);
    }
    if let Some(label) = super::privatelink::label(resource, attribute) {
        return Some(label);
    }
    if let Some(label) = super::eni::label(resource, attribute) {
        return Some(label);
    }
    if let Some(label) = super::peering::label(resource, attribute) {
        return Some(label);
    }
    if let Some(label) = super::vpc_gateways::label(resource, attribute) {
        return Some(label);
    }
    if let Some(label) = super::vpc_routes::label(resource, attribute) {
        return Some(label);
    }
    if let Some(label) = super::autoscaling_network::label(resource, attribute) {
        return Some(label);
    }
    if let Some(label) = super::sagemaker::label(resource, attribute) {
        return Some(label);
    }
    if let Some(label) = super::fsx::label(resource, attribute) {
        return Some(label);
    }
    if let Some(label) = super::batch::label(resource, attribute) {
        return Some(label);
    }
    if let Some(label) = super::opensearch_serverless::label(resource, attribute) {
        return Some(label);
    }
    if let Some(label) = super::networkfirewall::label(resource, attribute) {
        return Some(label);
    }
    if let Some(label) = super::apprunner::label(resource, attribute) {
        return Some(label);
    }
    let resource = super::aliases::canonical(resource);
    if let Some(label) = super::databases::label(resource, attribute) {
        return Some(label);
    }
    if let Some(label) = super::streaming::label(resource, attribute) {
        return Some(label);
    }
    if let Some(label) = super::eventbridge::label(resource, attribute) {
        return Some(label);
    }
    if let Some(label) = super::elasticache::label(resource, attribute) {
        return Some(label);
    }
    if let Some(label) = super::dynamodb::label(resource, attribute) {
        return Some(label);
    }
    if let Some(rule) = super::api_gateway::rule(resource, attribute) {
        return Some(rule.label);
    }
    if let Some(label) = super::lambda::label(resource, attribute) {
        return Some(label);
    }
    if let Some(label) = super::s3::label(resource, attribute) {
        return Some(label);
    }
    if let Some(label) = super::waf::label(resource, attribute) {
        return Some(label);
    }
    match (resource, attribute) {
        ("aws_cognito_user_pool_client", "user_pool_id") => Some("User pool reference"),
        ("aws_cognito_identity_pool", "cognito_identity_providers.client_id") => {
            Some("User pool client reference")
        }
        ("aws_cognito_identity_pool", "cognito_identity_providers.provider_name") => {
            Some("User pool provider reference")
        }
        ("aws_sfn_state_machine", "role_arn") => Some("Execution role reference"),
        ("aws_redshift_cluster", "cluster_subnet_group_name") => Some("Cluster subnet group"),
        ("aws_redshift_cluster", "cluster_parameter_group_name") => Some("Cluster parameter group"),
        ("aws_cloudtrail", "s3_bucket_name") => Some("Trail bucket reference"),
        ("aws_cloudtrail", "cloud_watch_logs_group_arn") => Some("Trail log group reference"),
        ("aws_cloudtrail", "cloud_watch_logs_role_arn") => Some("Logging role reference"),
        ("aws_cloudtrail", "kms_key_id") => Some("Encryption key reference"),
        ("aws_backup_plan", "rule.target_vault_name") => Some("Backup vault reference"),
        ("aws_backup_selection", "plan_id") => Some("Backup plan reference"),
        ("aws_backup_selection", "iam_role_arn") => Some("Backup role reference"),
        ("aws_backup_selection", "resources") => Some("Explicit backup resource reference"),
        ("aws_backup_vault", "kms_key_arn") => Some("Encryption key reference"),
        ("aws_rds_cluster", "global_cluster_identifier") => Some("Global Cluster membership"),
        ("aws_db_proxy_default_target_group", "db_proxy_name") => Some("Proxy owner"),
        ("aws_db_proxy_target", "db_proxy_name") => Some("Configured proxy"),
        ("aws_db_proxy_target", "target_group_name") => Some("Target group"),
        ("aws_db_proxy_target", "db_instance_identifier") => Some("DB instance target"),
        ("aws_db_proxy_target", "db_cluster_identifier") => Some("DB cluster target"),
        ("aws_db_proxy", "role_arn") => Some("Execution role reference"),
        ("aws_db_proxy", "auth.secret_arn") => Some("Authentication secret reference"),
        ("aws_eks_node_group" | "aws_eks_fargate_profile" | "aws_eks_addon", "cluster_name") => {
            Some("Cluster membership")
        }
        ("aws_eks_access_entry", "cluster_name") => Some("Access configuration cluster"),
        ("aws_eks_fargate_profile", "pod_execution_role_arn")
        | ("aws_eks_node_group", "node_role_arn")
        | ("aws_eks_cluster", "role_arn")
        | ("aws_eks_addon", "service_account_role_arn") => Some("Execution role reference"),
        ("aws_eks_access_entry", "principal_arn") => Some("Configured principal"),
        ("aws_ecr_repository_policy" | "aws_ecr_lifecycle_policy", "repository") => {
            Some("Configured repository")
        }
        ("aws_ecs_service", "cluster") => Some("Cluster membership"),
        ("aws_acmpca_certificate" | "aws_acm_certificate", "certificate_authority_arn") => {
            Some("Signing CA reference")
        }
        ("aws_acmpca_certificate_authority_certificate", "certificate_authority_arn") => {
            Some("Target CA reference")
        }
        ("aws_acmpca_certificate_authority_certificate", "certificate") => {
            Some("Installed certificate reference")
        }
        ("aws_glue_catalog_table", "database_name") => Some("Catalog database"),
        ("aws_glue_crawler", "role") | ("aws_glue_job", "role_arn") => {
            Some("Execution role reference")
        }
        ("aws_kinesis_firehose_delivery_stream", attribute) => super::firehose::label(attribute),
        ("aws_efs_mount_target" | "aws_efs_access_point", "file_system_id") => Some("File system"),
        ("aws_efs_mount_target", "subnet_id") => Some("Mount target subnet"),
        ("aws_volume_attachment", "instance_id") => Some("Attached instance"),
        ("aws_volume_attachment", "volume_id") => Some("Attached volume"),
        ("aws_secretsmanager_secret_version", "secret_id") => Some("Version secret"),
        ("aws_kms_alias", "target_key_id") => Some("Alias key"),
        ("aws_sqs_queue", "kms_master_key_id")
        | ("aws_db_instance" | "aws_rds_cluster", "kms_key_id")
        | (
            "aws_s3_bucket_server_side_encryption_configuration",
            "rule.apply_server_side_encryption_by_default.kms_master_key_id",
        ) => Some("Encryption key reference"),
        ("aws_s3_bucket_server_side_encryption_configuration", "bucket") => {
            Some("Configured bucket")
        }
        ("aws_acm_certificate_validation" | "aws_lb_listener", "certificate_arn")
        | ("aws_cloudfront_distribution", "viewer_certificate.acm_certificate_arn") => {
            Some("Certificate reference")
        }
        ("aws_route53_record", "zone_id") => Some("Hosted zone"),
        ("aws_route53_record", "alias.name") => Some("Alias target"),
        ("aws_route53_zone", "vpc.vpc_id") => Some("Associated VPC"),
        ("aws_cloudfront_distribution", attribute) => super::cloudfront::label(attribute),
        _ => super::iam::label(resource, attribute),
    }
}

fn source_fields(resource: &str, attribute: &str, source: &str) -> &'static [&'static str] {
    let dynamodb = super::dynamodb::source_fields(resource, attribute, source);
    if !dynamodb.is_empty() {
        return dynamodb;
    }
    let s3 = super::s3::source_fields(resource, attribute, source);
    if !s3.is_empty() {
        return s3;
    }
    let waf = super::waf::source_fields(resource, attribute, source);
    if !waf.is_empty() {
        return waf;
    }
    match (resource, attribute, source) {
        ("aws_cognito_user_pool_client", "user_pool_id", "aws_cognito_user_pool")
        | (
            "aws_cognito_identity_pool",
            "cognito_identity_providers.client_id",
            "aws_cognito_user_pool_client",
        ) => &["id"],
        (
            "aws_cognito_identity_pool",
            "cognito_identity_providers.provider_name",
            "aws_cognito_user_pool",
        ) => &["endpoint"],
        ("aws_sfn_state_machine", "role_arn", "aws_iam_role") => &["arn"],
        ("aws_redshift_cluster", "cluster_subnet_group_name", "aws_redshift_subnet_group")
        | (
            "aws_redshift_cluster",
            "cluster_parameter_group_name",
            "aws_redshift_parameter_group",
        ) => &["name", "id"],
        ("aws_cloudtrail", "s3_bucket_name", "aws_s3_bucket") => &["id", "bucket"],
        ("aws_cloudtrail", "cloud_watch_logs_group_arn", "aws_cloudwatch_log_group")
        | ("aws_cloudtrail", "cloud_watch_logs_role_arn", "aws_iam_role") => &["arn"],
        ("aws_cloudtrail", "kms_key_id", "aws_kms_key") => &["arn", "id", "key_id"],
        ("aws_backup_plan", "rule.target_vault_name", "aws_backup_vault") => &["name", "id"],
        ("aws_backup_selection", "plan_id", "aws_backup_plan") => &["id"],
        ("aws_backup_selection", "iam_role_arn", "aws_iam_role")
        | ("aws_backup_vault", "kms_key_arn", "aws_kms_key") => &["arn"],
        ("aws_rds_cluster", "global_cluster_identifier", "aws_rds_global_cluster") => {
            &["id", "global_cluster_identifier"]
        }
        (
            "aws_db_proxy_default_target_group" | "aws_db_proxy_target",
            "db_proxy_name",
            "aws_db_proxy",
        ) => &["name", "id"],
        ("aws_db_proxy_target", "target_group_name", "aws_db_proxy_default_target_group") => {
            &["name"]
        }
        ("aws_db_proxy_target", "db_instance_identifier", "aws_db_instance") => &["identifier"],
        ("aws_db_proxy_target", "db_cluster_identifier", "aws_rds_cluster") => {
            &["cluster_identifier"]
        }
        ("aws_db_proxy", "role_arn", "aws_iam_role")
        | ("aws_db_proxy", "auth.secret_arn", "aws_secretsmanager_secret") => &["arn"],
        (
            "aws_eks_node_group"
            | "aws_eks_fargate_profile"
            | "aws_eks_addon"
            | "aws_eks_access_entry",
            "cluster_name",
            "aws_eks_cluster",
        ) => &["name", "id"],
        ("aws_eks_fargate_profile", "pod_execution_role_arn", "aws_iam_role")
        | ("aws_eks_node_group", "node_role_arn", "aws_iam_role")
        | ("aws_eks_cluster", "role_arn", "aws_iam_role")
        | ("aws_eks_addon", "service_account_role_arn", "aws_iam_role")
        | ("aws_eks_access_entry", "principal_arn", "aws_iam_role" | "aws_iam_user") => &["arn"],
        (
            "aws_ecr_repository_policy" | "aws_ecr_lifecycle_policy",
            "repository",
            "aws_ecr_repository",
        ) => &["name", "id"],
        ("aws_ecs_service", "cluster", "aws_ecs_cluster") => &["id", "arn", "name"],
        (
            "aws_acmpca_certificate"
            | "aws_acm_certificate"
            | "aws_acmpca_certificate_authority_certificate",
            "certificate_authority_arn",
            "aws_acmpca_certificate_authority",
        ) => &["arn"],
        (
            "aws_acmpca_certificate_authority_certificate",
            "certificate",
            "aws_acmpca_certificate",
        ) => &["certificate"],
        ("aws_glue_catalog_table", "database_name", "aws_glue_catalog_database") => &["name"],
        ("aws_glue_crawler", "role", "aws_iam_role") => &["arn"],
        ("aws_glue_job", "role_arn", "aws_iam_role") => &["arn"],
        (
            "aws_kinesis_firehose_delivery_stream",
            "kinesis_source_configuration.kinesis_stream_arn",
            "aws_kinesis_stream",
        )
        | (
            "aws_kinesis_firehose_delivery_stream",
            "extended_s3_configuration.bucket_arn"
            | "extended_s3_configuration.s3_backup_configuration.bucket_arn",
            "aws_s3_bucket",
        ) => &["arn"],
        (
            "aws_efs_mount_target" | "aws_efs_access_point",
            "file_system_id",
            "aws_efs_file_system",
        )
        | ("aws_efs_mount_target", "subnet_id", "aws_subnet") => &["id"],
        ("aws_volume_attachment", "instance_id", "aws_instance")
        | ("aws_volume_attachment", "volume_id", "aws_ebs_volume") => &["id"],
        ("aws_secretsmanager_secret_version", "secret_id", "aws_secretsmanager_secret") => {
            &["id", "arn"]
        }
        ("aws_kms_alias", "target_key_id", "aws_kms_key")
        | ("aws_sqs_queue", "kms_master_key_id", "aws_kms_key")
        | ("aws_db_instance" | "aws_rds_cluster", "kms_key_id", "aws_kms_key")
        | (
            "aws_s3_bucket_server_side_encryption_configuration",
            "rule.apply_server_side_encryption_by_default.kms_master_key_id",
            "aws_kms_key",
        ) => &["id", "key_id", "arn"],
        ("aws_s3_bucket_server_side_encryption_configuration", "bucket", "aws_s3_bucket") => {
            &["id", "bucket"]
        }
        (
            "aws_acm_certificate_validation" | "aws_lb_listener",
            "certificate_arn",
            "aws_acm_certificate",
        )
        | (
            "aws_cloudfront_distribution",
            "viewer_certificate.acm_certificate_arn",
            "aws_acm_certificate",
        ) => &["arn", "id"],
        ("aws_lb_listener", "certificate_arn", "aws_acm_certificate_validation")
        | (
            "aws_cloudfront_distribution",
            "viewer_certificate.acm_certificate_arn",
            "aws_acm_certificate_validation",
        ) => &["certificate_arn"],
        ("aws_route53_record", "zone_id", "aws_route53_zone") => &["zone_id", "id"],
        ("aws_route53_record", "alias.name", "aws_lb") => &["dns_name"],
        ("aws_route53_record", "alias.name", "aws_cloudfront_distribution") => &["domain_name"],
        ("aws_route53_record", "alias.zone_id", "aws_lb") => &["zone_id"],
        ("aws_route53_record", "alias.zone_id", "aws_cloudfront_distribution") => {
            &["hosted_zone_id"]
        }
        ("aws_route53_zone", "vpc.vpc_id", "aws_vpc") => &["id"],
        ("aws_cloudfront_distribution", attribute, source) => {
            super::cloudfront::source_fields(attribute, source)
        }
        _ => &[],
    }
}

pub(super) fn cluster_binding_matches(
    consumer: Option<&crate::model::ProviderConfiguration>,
    endpoint: Option<&crate::model::ProviderConfiguration>,
    consumer_region: Option<&crate::model::RegionScope>,
    endpoint_region: Option<&crate::model::RegionScope>,
) -> bool {
    consumer.is_some_and(|config| {
        !config.account_conflict && config.region.is_some() && endpoint == Some(config)
    }) && consumer_region.is_some()
        && consumer_region == endpoint_region
}

pub(super) fn scalar<'a>(value: &'a Value, unknown: &Value, path: &[&str]) -> Option<&'a str> {
    if unknown == &Value::Bool(true) {
        return None;
    }
    if path.is_empty() {
        return value.as_str().filter(|s| !s.is_empty());
    }
    if let Some(values) = value.as_array() {
        let [value] = values.as_slice() else {
            return None;
        };
        if unknown.as_array().is_some_and(|u| u.len() != 1) {
            return None;
        }
        return scalar(value, &unknown[0], path);
    }
    scalar(&value[path[0]], &unknown[path[0]], &path[1..])
}

fn indexed_scalar<'a>(
    value: &'a Value,
    unknown: &Value,
    path: &[&str],
    lengths: &[usize],
) -> Option<&'a str> {
    if unknown == &Value::Bool(true) {
        return None;
    }
    if let Some(values) = value.as_array() {
        let (&length, remaining) = lengths.split_first()?;
        let (&index, path) = path.split_first()?;
        let index = index.parse::<usize>().ok()?;
        if !matches!(unknown, Value::Null | Value::Bool(false) | Value::Array(_))
            || values.len() != length
            || unknown
                .as_array()
                .is_some_and(|masks| masks.len() != length)
        {
            return None;
        }
        return indexed_scalar(values.get(index)?, &unknown[index], path, remaining);
    }
    if let Some((field, path)) = path.split_first() {
        if !matches!(unknown, Value::Null | Value::Bool(false) | Value::Object(_)) {
            return None;
        }
        return indexed_scalar(&value[*field], &unknown[*field], path, lengths);
    }
    (lengths.is_empty() && matches!(unknown, Value::Null | Value::Bool(false)))
        .then(|| value.as_str().filter(|s| !s.is_empty()))
        .flatten()
}

pub(crate) fn annotate(
    nodes: &[TerraformEntity],
    references: &mut [AttributeReference],
    evidence: &[(&Value, &Value)],
) {
    for reference in references.iter_mut() {
        let node = &nodes[reference.target];
        if super::eventbridge::label(&node.resource_type, &reference.attribute).is_some() {
            reference.service_reference_verified =
                super::eventbridge::verify(nodes, reference, evidence);
            continue;
        }
        if super::elasticache::label(&node.resource_type, &reference.attribute).is_some() {
            reference.service_reference_verified =
                super::elasticache::verify(nodes, reference, evidence);
            continue;
        }
        if node.resource_type == "aws_backup_selection" && reference.attribute == "resources" {
            reference.service_reference_verified =
                super::backup::verified_resources(nodes, reference, evidence);
            continue;
        }
        if (super::organizations::resource(&node.resource_type)
            && reference.attribute == "parent_id")
            || node.resource_type == "aws_organizations_policy_attachment"
        {
            continue;
        }
        if super::network::endpoint_type(&node.resource_type, &reference.attribute).is_some() {
            reference.service_reference_verified =
                super::network::verified(nodes, reference, evidence);
            continue;
        }
        if super::iam::label(&node.resource_type, &reference.attribute).is_some() {
            reference.service_reference_verified = super::iam::verified(nodes, reference, evidence);
            continue;
        }
        let [source] = reference.sources.as_slice() else {
            continue;
        };
        let endpoint = &nodes[*source];
        if !reference.complete
            || (crate::provider::indexed_service_reference(
                &node.provider,
                &node.resource_type,
                &reference.attribute,
            ) && !super::s3::list_shape(&node.resource_type, &reference.indexed_block_lengths))
            || !reference.instances_explicit
            || !reference.issues.is_empty()
            || !node.provider.is_aws()
            || !endpoint.provider.is_aws()
            || (node.mode != EntityMode::Managed
                && node.resource_type != "aws_secretsmanager_secret_version")
            || endpoint.mode != EntityMode::Managed
            || node.deposed_key.is_some()
            || endpoint.deposed_key.is_some()
            || node.action == Action::Delete
            || endpoint.action == Action::Delete
        {
            continue;
        }
        let fields = source_fields(
            super::aliases::entity_type(node),
            &reference.attribute,
            super::aliases::entity_type(endpoint),
        );
        let (value, unknown) = evidence[reference.target];
        let (target, target_unknown) = evidence[*source];
        let path = reference.attribute.split('.').collect::<Vec<_>>();
        let actual = if reference.indexed_block_lengths.is_empty() {
            scalar(value, unknown, &path)
        } else {
            indexed_scalar(value, unknown, &path, &reference.indexed_block_lengths)
        };
        reference.service_reference_verified = fields.iter().any(|field| {
            reference.source_attributes.len() == 1
                && reference.source_attributes.contains(*field)
                && actual.is_some()
                && (actual == scalar(target, target_unknown, &[*field])
                    || (node.resource_type == "aws_cloudtrail"
                        && reference.attribute == "cloud_watch_logs_group_arn"
                        && scalar(target, target_unknown, &[*field]).is_some_and(|target| {
                            actual.and_then(|arn| arn.strip_suffix(":*")) == Some(target)
                        })))
        });
        reference.service_reference_verified &=
            super::waf::verify_scope(nodes, reference, evidence);
        reference.service_reference_verified &= super::dynamodb::verify(nodes, reference, evidence);
        if (node.resource_type == "aws_ecs_service" && reference.attribute == "cluster")
            || node.resource_type == "aws_redshift_cluster"
            || (node.resource_type == "aws_backup_plan"
                && reference.attribute == "rule.target_vault_name")
            || (node.resource_type == "aws_backup_selection" && reference.attribute == "plan_id")
            || matches!(
                node.resource_type.as_str(),
                "aws_db_proxy_default_target_group" | "aws_db_proxy_target"
            )
            || (node.resource_type.starts_with("aws_eks_") && reference.attribute == "cluster_name")
            || matches!(
                node.resource_type.as_str(),
                "aws_ecr_repository_policy" | "aws_ecr_lifecycle_policy"
            )
        {
            reference.service_reference_verified &= cluster_binding_matches(
                node.provider_configuration.as_ref(),
                endpoint.provider_configuration.as_ref(),
                super::scope::scope(node)
                    .as_ref()
                    .and_then(|s| s.region.as_ref()),
                super::scope::scope(endpoint)
                    .as_ref()
                    .and_then(|s| s.region.as_ref()),
            );
        }
        if node.resource_type == "aws_glue_catalog_table" && reference.attribute == "database_name"
        {
            reference.service_reference_verified &= super::glue::catalog_verified(
                node,
                endpoint,
                value,
                unknown,
                target,
                target_unknown,
            );
        }
        if node.resource_type == "aws_kinesis_firehose_delivery_stream" {
            reference.service_reference_verified &=
                super::firehose::configuration_verified(&reference.attribute, value, unknown);
        }
        if node.resource_type == "aws_cloudfront_distribution"
            && reference.attribute == "origin.domain_name"
            && super::aliases::entity_type(endpoint) == "aws_lb"
        {
            reference.service_reference_verified &=
                scalar(target, target_unknown, &["load_balancer_type"]) == Some("application");
        }
        if reference.service_reference_verified
            && reference.attribute == "alias.name"
            && super::aliases::entity_type(endpoint) == "aws_lb"
        {
            reference.service_reference_verified = matches!(
                scalar(target, target_unknown, &["load_balancer_type"]),
                Some("application" | "network")
            );
        }
    }
    super::streaming::annotate(nodes, references, evidence);
    super::vpc_routes::annotate(nodes, references, evidence);
    super::vpc_gateways::annotate(nodes, references, evidence);
    super::privatelink::annotate(nodes, references, evidence);
    super::security_group_rules::annotate(nodes, references, evidence);
    super::databases::annotate(nodes, references, evidence);
    super::apprunner::annotate(nodes, references, evidence);
    super::sagemaker::annotate(nodes, references, evidence);
    super::opensearch_serverless::annotate(nodes, references, evidence);
    super::batch::annotate(nodes, references, evidence);
    super::autoscaling_network::annotate(nodes, references, evidence);
    super::fsx::annotate(nodes, references, evidence);
    super::networkfirewall::annotate(nodes, references, evidence);
    super::lambda::annotate(nodes, references, evidence);
    super::api_gateway::annotate(nodes, references, evidence);
    super::rds_proxy::validate_targets(nodes, references, evidence);
    let aliases: Vec<_> = references
        .iter()
        .filter(|r| r.attribute == "alias.zone_id" && r.service_reference_verified)
        .map(|r| (r.target, r.sources.clone()))
        .collect();
    for reference in references
        .iter_mut()
        .filter(|r| r.attribute == "alias.name")
    {
        reference.service_reference_verified &= aliases
            .iter()
            .any(|(target, sources)| *target == reference.target && *sources == reference.sources);
    }
}

pub(super) fn verified(raw: &Input, reference: &AttributeReference) -> bool {
    reference.service_reference_verified
        && (super::lambda_events::label(
            &raw.nodes[reference.target].resource_type,
            &reference.attribute,
        )
        .is_none()
            || super::streaming::bindings(raw, reference))
        && (super::lambda_config::label(
            &raw.nodes[reference.target].resource_type,
            &reference.attribute,
        )
        .is_none()
            || super::lambda_config::bindings(raw, reference))
        && (super::elbv2_cognito::label(
            &raw.nodes[reference.target].resource_type,
            &reference.attribute,
        )
        .is_none()
            || super::streaming::bindings(raw, reference))
        && (super::elbv2_network::label(
            &raw.nodes[reference.target].resource_type,
            &reference.attribute,
        )
        .is_none()
            || super::streaming::bindings(raw, reference))
        && (super::elbv2_tls::label(
            &raw.nodes[reference.target].resource_type,
            &reference.attribute,
        )
        .is_none()
            || super::elbv2_tls::bindings(raw, reference))
        && (super::elbv2::label(
            &raw.nodes[reference.target].resource_type,
            &reference.attribute,
        )
        .is_none()
            || super::streaming::bindings(raw, reference))
        && (super::security_group_rules::label(
            &raw.nodes[reference.target].resource_type,
            &reference.attribute,
        )
        .is_none()
            || super::security_group_rules::bindings(raw, reference))
        && (super::privatelink::label(
            &raw.nodes[reference.target].resource_type,
            &reference.attribute,
        )
        .is_none()
            || super::privatelink::bindings(raw, reference))
        && (super::eni::label(
            &raw.nodes[reference.target].resource_type,
            &reference.attribute,
        )
        .is_none()
            || super::vpc_routes::bindings(raw, reference))
        && (super::peering::label(
            &raw.nodes[reference.target].resource_type,
            &reference.attribute,
        )
        .is_none()
            || super::peering::bindings(raw, reference))
        && (super::vpc_gateways::label(
            &raw.nodes[reference.target].resource_type,
            &reference.attribute,
        )
        .is_none()
            || super::vpc_routes::bindings(raw, reference))
        && (super::vpc_routes::label(
            &raw.nodes[reference.target].resource_type,
            &reference.attribute,
        )
        .is_none()
            || super::vpc_routes::bindings(raw, reference))
        && (super::autoscaling_network::label(
            &raw.nodes[reference.target].resource_type,
            &reference.attribute,
        )
        .is_none()
            || super::autoscaling_network::bindings(raw, reference))
        && (super::sagemaker::label(
            &raw.nodes[reference.target].resource_type,
            &reference.attribute,
        )
        .is_none()
            || super::sagemaker::bindings(raw, reference))
        && (super::fsx::label(
            &raw.nodes[reference.target].resource_type,
            &reference.attribute,
        )
        .is_none()
            || super::apprunner::bindings(raw, reference))
        && (super::batch::label(
            &raw.nodes[reference.target].resource_type,
            &reference.attribute,
        )
        .is_none()
            || super::apprunner::bindings(raw, reference))
        && (super::opensearch_serverless::label(
            &raw.nodes[reference.target].resource_type,
            &reference.attribute,
        )
        .is_none()
            || super::streaming::bindings(raw, reference))
        && (super::networkfirewall::label(
            &raw.nodes[reference.target].resource_type,
            &reference.attribute,
        )
        .is_none()
            || super::apprunner::bindings(raw, reference))
        && (super::apprunner::label(
            &raw.nodes[reference.target].resource_type,
            &reference.attribute,
        )
        .is_none()
            || super::apprunner::bindings(raw, reference))
        && (super::databases::label(
            &raw.nodes[reference.target].resource_type,
            &reference.attribute,
        )
        .is_none()
            || super::databases::bindings(raw, reference))
        && (super::streaming::label(
            &raw.nodes[reference.target].resource_type,
            &reference.attribute,
        )
        .is_none()
            || super::streaming::bindings(raw, reference))
        && (super::eventbridge::label(
            &raw.nodes[reference.target].resource_type,
            &reference.attribute,
        )
        .is_none()
            || reference.sources.iter().all(|&source| {
                let node = &raw.nodes[reference.target];
                let endpoint = &raw.nodes[source];
                super::eventbridge::bindings(
                    node.provider_configuration.as_ref(),
                    endpoint.provider_configuration.as_ref(),
                    node.entity.scope.as_ref().and_then(|s| s.region.as_ref()),
                    endpoint
                        .entity
                        .scope
                        .as_ref()
                        .and_then(|s| s.region.as_ref()),
                    endpoint.resource_type == "aws_iam_role",
                )
            }))
        && (super::elasticache::label(
            &raw.nodes[reference.target].resource_type,
            &reference.attribute,
        )
        .is_none()
            || reference.sources.iter().all(|&source| {
                let node = &raw.nodes[reference.target];
                let endpoint = &raw.nodes[source];
                super::elasticache::bindings(
                    &reference.attribute,
                    node.provider_configuration.as_ref(),
                    endpoint.provider_configuration.as_ref(),
                    node.entity.scope.as_ref().and_then(|s| s.region.as_ref()),
                    endpoint
                        .entity
                        .scope
                        .as_ref()
                        .and_then(|s| s.region.as_ref()),
                )
            }))
        && (super::dynamodb::label(
            &raw.nodes[reference.target].resource_type,
            &reference.attribute,
        )
        .is_none()
            || reference.sources.iter().all(|&source| {
                let node = &raw.nodes[reference.target];
                let endpoint = &raw.nodes[source];
                super::dynamodb::bindings(
                    &node.resource_type,
                    node.provider_configuration.as_ref(),
                    endpoint.provider_configuration.as_ref(),
                    node.entity.scope.as_ref().and_then(|s| s.region.as_ref()),
                    endpoint
                        .entity
                        .scope
                        .as_ref()
                        .and_then(|s| s.region.as_ref()),
                )
            }))
        && super::iam::bindings_compatible(&raw.nodes, reference)
        && (raw.nodes[reference.target].resource_type != "aws_rds_cluster"
            || reference.attribute != "global_cluster_identifier"
            || reference.sources.iter().all(|&source| {
                let left = raw.nodes[reference.target].provider_configuration.as_ref();
                let right = raw.nodes[source].provider_configuration.as_ref();
                !left
                    .into_iter()
                    .chain(right)
                    .any(|config| config.account_conflict)
                    && left
                        .and_then(|config| config.account.as_ref())
                        .zip(right.and_then(|config| config.account.as_ref()))
                        .is_none_or(|(left, right)| left == right)
            }))
        && (!matches!(
            raw.nodes[reference.target].resource_type.as_str(),
            "aws_ecs_service" | "aws_ecr_repository_policy" | "aws_ecr_lifecycle_policy"
        ) || reference.sources.iter().all(|&source| {
            cluster_binding_matches(
                raw.nodes[reference.target].provider_configuration.as_ref(),
                raw.nodes[source].provider_configuration.as_ref(),
                raw.nodes[reference.target]
                    .entity
                    .scope
                    .as_ref()
                    .and_then(|s| s.region.as_ref()),
                raw.nodes[source]
                    .entity
                    .scope
                    .as_ref()
                    .and_then(|s| s.region.as_ref()),
            )
        }))
}

pub(super) fn present(raw: &Input, graph: &mut Graph) {
    let spatial_targets: std::collections::BTreeSet<_> = raw
        .nodes
        .iter()
        .filter(|node| {
            node.provider.is_aws()
                && matches!(
                    node.resource_type.as_str(),
                    "aws_cloudwatch_log_group"
                        | "aws_ecr_repository"
                        | "aws_eks_cluster"
                        | "aws_ecs_cluster"
                        | "aws_athena_workgroup"
                        | "aws_athena_data_catalog"
                        | "aws_glue_job"
                        | "aws_glue_crawler"
                        | "aws_glue_catalog_database"
                        | "aws_glue_catalog_table"
                )
        })
        .flat_map(|node| &node.entity.provenance)
        .collect();
    for node in &mut graph.nodes {
        let mut split_references = Vec::new();
        for reference in &mut node.references {
            if node.provider.is_aws()
                && matches!(
                    node.resource_type.as_str(),
                    "aws_athena_workgroup"
                        | "aws_athena_data_catalog"
                        | "aws_glue_job"
                        | "aws_glue_crawler"
                        | "aws_glue_catalog_database"
                        | "aws_glue_catalog_table"
                )
            {
                reference.show_spatial = true;
            }
            if reference.show_spatial
                || !reference
                    .targets
                    .iter()
                    .any(|target| spatial_targets.contains(&target.id))
            {
                continue;
            }
            if reference
                .targets
                .iter()
                .all(|target| spatial_targets.contains(&target.id))
            {
                reference.show_spatial = true;
                continue;
            }
            let (spatial, ordinary): (Vec<_>, Vec<_>) = reference
                .targets
                .drain(..)
                .partition(|target| spatial_targets.contains(&target.id));
            let mut split = reference.clone();
            split.targets = spatial;
            split.show_spatial = true;
            reference.targets = ordinary;
            split_references.push(split);
        }
        node.references.extend(split_references);
    }
    for reference in &raw.attributes {
        let node = &raw.nodes[reference.target];
        if !node.provider.is_aws() {
            continue;
        }
        if super::cloudfront::indexed(&node.resource_type, &reference.attribute)
            || super::elbv2_cognito::indexed(&node.resource_type, &reference.attribute)
            || super::elbv2_network::indexed(&node.resource_type, &reference.attribute)
            || super::s3::aggregate(raw, reference)
            || super::networkfirewall::aggregate(raw, reference)
        {
            graph.nodes[reference.target]
                .references
                .retain(|r| r.dependency || r.attribute != reference.attribute);
            continue;
        }
        let Some(label) = label(&node.resource_type, &reference.attribute) else {
            continue;
        };
        let verified = verified(raw, reference);
        if verified {
            graph.nodes[reference.target]
                .references
                .retain(|r| r.dependency || r.attribute != reference.attribute);
        }
        graph.nodes[reference.target]
            .references
            .push(ResourceReference {
                kind: if verified
                    && (matches!(
                        label,
                        "Cluster membership"
                            | "Global Cluster membership"
                            | "V2 table membership"
                            | "Legacy V1 table membership"
                            | "Global datastore membership"
                            | "Cache replication group membership"
                            | "Cache user membership"
                            | "Schedule group membership"
                    ) || matches!(
                        node.resource_type.as_str(),
                        "aws_iam_group_membership" | "aws_iam_user_group_membership"
                    )) {
                    crate::model::ReferenceKind::Membership
                } else {
                    crate::model::ReferenceKind::Configured
                },
                preserve_spatial: false,
                show_spatial: true,
                label,
                attribute: reference.attribute.clone(),
                targets: if verified {
                    reference
                        .sources
                        .iter()
                        .map(|&source| ReferenceTarget {
                            account_scope: None,
                            state: None,
                            id: raw.nodes[source].entity.provenance[0].clone(),
                            placement: raw.nodes[source].placement,
                        })
                        .collect()
                } else {
                    Vec::new()
                },
                complete: verified,
                dependency: false,
                issues: if verified {
                    Default::default()
                } else {
                    [DiagnosticReason::UnprovenEndpointValue].into()
                },
            });
    }
}

pub(super) fn diagnostics(raw: &Input) -> Vec<Diagnostic> {
    raw.attributes
        .iter()
        .filter_map(|reference| {
            let node = &raw.nodes[reference.target];
            (node.provider.is_aws()
                && !super::s3::aggregate(raw, reference)
                && !super::networkfirewall::aggregate(raw, reference)
                && label(&node.resource_type, &reference.attribute).is_some()
                && !verified(raw, reference))
            .then(|| Diagnostic {
                address: node.address.clone(),
                deposed_key: node.deposed_key.clone(),
                attribute: reference.attribute.clone(),
                reason: DiagnosticReason::UnprovenEndpointValue,
            })
        })
        .collect()
}
