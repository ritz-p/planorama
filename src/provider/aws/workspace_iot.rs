use super::configured_links::{self, Rule};
use crate::model::{AttributeReference, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;

fn rule(kind: &str, path: &str) -> Option<Rule> {
    Some(match (kind, path) {
        (
            "aws_appstream_directory_config",
            "certificate_based_auth_properties.certificate_authority_arn",
        ) => Rule::new(
            "Directory certificate authority",
            &[("aws_acmpca_certificate_authority", &["arn"])],
            false,
            &["certificate_based_auth_properties"],
        ),
        ("aws_appstream_fleet", "iam_role_arn") => {
            Rule::new("Fleet IAM role", &[("aws_iam_role", &["arn"])], false, &[])
        }
        ("aws_appstream_fleet", "vpc_config.subnet_ids") => Rule::new(
            "AppStream subnets",
            &[("aws_subnet", &["id"])],
            true,
            &["vpc_config"],
        ),
        ("aws_appstream_fleet", "vpc_config.security_group_ids") => Rule::new(
            "AppStream security groups",
            &[("aws_security_group", &["id"])],
            true,
            &["vpc_config"],
        ),
        ("aws_appstream_fleet", "domain_join_info.directory_name") => Rule::new(
            "AppStream directory configuration",
            &[("aws_appstream_directory_config", &["directory_name"])],
            false,
            &["domain_join_info"],
        ),
        ("aws_appstream_fleet_stack_association", "fleet_name") => Rule::new(
            "AppStream fleet",
            &[("aws_appstream_fleet", &["name"])],
            false,
            &[],
        ),
        ("aws_appstream_fleet_stack_association", "stack_name") => Rule::new(
            "AppStream stack",
            &[("aws_appstream_stack", &["name"])],
            false,
            &[],
        ),
        ("aws_appstream_image_builder", "iam_role_arn") => Rule::new(
            "Image builder IAM role",
            &[("aws_iam_role", &["arn"])],
            false,
            &[],
        ),
        ("aws_appstream_image_builder", "vpc_config.subnet_ids") => Rule::new(
            "AppStream subnets",
            &[("aws_subnet", &["id"])],
            true,
            &["vpc_config"],
        ),
        ("aws_appstream_image_builder", "access_endpoint.vpce_id") => Rule::new(
            "AppStream interface endpoint",
            &[("aws_vpc_endpoint", &["id"])],
            false,
            &[],
        )
        .unordered(&["access_endpoint"]),
        ("aws_appstream_stack", "access_endpoints.vpce_id") => Rule::new(
            "AppStream interface endpoint",
            &[("aws_vpc_endpoint", &["id"])],
            false,
            &[],
        )
        .unordered(&["access_endpoints"]),
        ("aws_appstream_user_stack_association", "stack_name") => Rule::new(
            "AppStream stack",
            &[("aws_appstream_stack", &["name"])],
            false,
            &[],
        ),
        ("aws_appstream_user_stack_association", "user_name") => Rule::new(
            "AppStream user",
            &[("aws_appstream_user", &["user_name"])],
            false,
            &[],
        ),
        ("aws_iot_authorizer", "authorizer_function_arn") => Rule::new(
            "IoT authorizer Lambda function",
            &[("aws_lambda_function", &["arn"])],
            false,
            &[],
        ),
        ("aws_iot_logging_options", "role_arn") => Rule::new(
            "IoT logging role",
            &[("aws_iam_role", &["arn"])],
            false,
            &[],
        ),
        ("aws_iot_provisioning_template", "provisioning_role_arn") => Rule::new(
            "IoT provisioning role",
            &[("aws_iam_role", &["arn"])],
            false,
            &[],
        ),
        ("aws_iot_provisioning_template", "pre_provisioning_hook.target_arn") => Rule::new(
            "IoT provisioning hook",
            &[("aws_lambda_function", &["arn"])],
            false,
            &["pre_provisioning_hook"],
        ),
        ("aws_iot_role_alias", "role_arn") => Rule::new(
            "IoT aliased role",
            &[("aws_iam_role", &["arn"])],
            false,
            &[],
        ),
        ("aws_iot_topic_rule", "lambda.function_arn") => Rule::new(
            "IoT Lambda action",
            &[("aws_lambda_function", &["arn"])],
            false,
            &[],
        )
        .unordered(&["lambda"]),
        ("aws_iot_topic_rule", "s3.bucket_name") => Rule::new(
            "IoT S3 action",
            &[("aws_s3_bucket", &["bucket", "id"])],
            false,
            &[],
        )
        .unordered(&["s3"]),
        ("aws_iot_topic_rule", "sns.target_arn") => {
            Rule::new("IoT SNS action", &[("aws_sns_topic", &["arn"])], false, &[])
                .unordered(&["sns"])
        }
        ("aws_iot_topic_rule", "error_action.lambda.function_arn") => Rule::new(
            "IoT Lambda error action",
            &[("aws_lambda_function", &["arn"])],
            false,
            &["error_action", "error_action.lambda"],
        ),
        ("aws_iot_topic_rule_destination", "vpc_configuration.role_arn") => Rule::new(
            "IoT VPC destination role",
            &[("aws_iam_role", &["arn"])],
            false,
            &["vpc_configuration"],
        ),
        ("aws_iot_topic_rule_destination", "vpc_configuration.subnet_ids") => Rule::new(
            "IoT VPC destination subnets",
            &[("aws_subnet", &["id"])],
            true,
            &["vpc_configuration"],
        ),
        ("aws_iot_topic_rule_destination", "vpc_configuration.vpc_id") => Rule::new(
            "IoT destination VPC",
            &[("aws_vpc", &["id"])],
            false,
            &["vpc_configuration"],
        ),
        ("aws_iot_ca_certificate", "registration_config.role_arn") => Rule::new(
            "IoT CA registration role",
            &[("aws_iam_role", &["arn"])],
            false,
            &["registration_config"],
        ),
        ("aws_iot_ca_certificate", "registration_config.template_name") => Rule::new(
            "IoT registration template",
            &[("aws_iot_provisioning_template", &["name"])],
            false,
            &["registration_config"],
        ),
        ("aws_iot_domain_configuration", "server_certificate_arns") => Rule::new(
            "IoT server certificates",
            &[("aws_acm_certificate", &["arn"])],
            true,
            &[],
        ),
        ("aws_iot_domain_configuration", "validation_certificate_arn") => Rule::new(
            "IoT validation certificate",
            &[("aws_acm_certificate", &["arn"])],
            false,
            &[],
        ),
        ("aws_iot_domain_configuration", "authorizer_config.default_authorizer_name") => Rule::new(
            "IoT default authorizer",
            &[("aws_iot_authorizer", &["name"])],
            false,
            &["authorizer_config"],
        ),
        ("aws_iot_policy_attachment", "policy") => {
            Rule::new("IoT policy", &[("aws_iot_policy", &["name"])], false, &[])
        }
        ("aws_iot_policy_attachment", "target") => Rule::new(
            "IoT policy principal",
            &[
                ("aws_iot_certificate", &["arn"]),
                ("aws_iot_thing_group", &["arn"]),
            ],
            false,
            &[],
        ),
        ("aws_iot_thing", "thing_type_name") => Rule::new(
            "IoT thing type",
            &[("aws_iot_thing_type", &["name"])],
            false,
            &[],
        ),
        ("aws_iot_thing_group", "parent_group_name") => Rule::new(
            "Parent IoT thing group",
            &[("aws_iot_thing_group", &["name"])],
            false,
            &[],
        ),
        ("aws_iot_thing_group_membership", "thing_group_name") => Rule::new(
            "IoT thing group",
            &[("aws_iot_thing_group", &["name"])],
            false,
            &[],
        ),
        ("aws_iot_thing_group_membership", "thing_name") => {
            Rule::new("IoT thing", &[("aws_iot_thing", &["name"])], false, &[])
        }
        ("aws_iot_thing_principal_attachment", "thing") => {
            Rule::new("IoT thing", &[("aws_iot_thing", &["name"])], false, &[])
        }
        ("aws_iot_thing_principal_attachment", "principal") => Rule::new(
            "IoT certificate principal",
            &[("aws_iot_certificate", &["arn"])],
            false,
            &[],
        ),
        ("aws_workspaces_directory", "directory_id") => Rule::new(
            "Directory Service directory",
            &[("aws_directory_service_directory", &["id"])],
            false,
            &[],
        ),
        ("aws_workspaces_directory", "ip_group_ids") => Rule::new(
            "WorkSpaces IP groups",
            &[("aws_workspaces_ip_group", &["id"])],
            true,
            &[],
        ),
        ("aws_workspaces_directory", "subnet_ids") => {
            Rule::new("WorkSpaces subnets", &[("aws_subnet", &["id"])], true, &[])
        }
        ("aws_workspaces_pool", "directory_id") => Rule::new(
            "Registered WorkSpaces directory",
            &[("aws_workspaces_directory", &["directory_id", "id"])],
            false,
            &[],
        ),
        ("aws_workspaces_workspace", "directory_id") => Rule::new(
            "Registered WorkSpaces directory",
            &[("aws_workspaces_directory", &["directory_id", "id"])],
            false,
            &[],
        ),
        ("aws_workspacesweb_identity_provider", "portal_arn") => Rule::new(
            "Secure Browser portal",
            &[("aws_workspacesweb_portal", &["portal_arn"])],
            false,
            &[],
        ),
        ("aws_workspacesweb_network_settings", "vpc_id") => {
            Rule::new("Secure Browser VPC", &[("aws_vpc", &["id"])], false, &[])
        }
        ("aws_workspacesweb_network_settings", "subnet_ids") => Rule::new(
            "Secure Browser subnets",
            &[("aws_subnet", &["id"])],
            true,
            &[],
        ),
        ("aws_workspacesweb_network_settings", "security_group_ids") => Rule::new(
            "Secure Browser security groups",
            &[("aws_security_group", &["id"])],
            true,
            &[],
        ),
        ("aws_workspacesweb_portal", "customer_managed_key") => Rule::new(
            "Secure Browser encryption key",
            &[("aws_kms_key", &["arn"])],
            false,
            &[],
        ),
        ("aws_workspacesweb_session_logger", "log_configuration.s3.bucket") => Rule::new(
            "Session logging bucket",
            &[("aws_s3_bucket", &["bucket", "id"])],
            false,
            &["log_configuration", "log_configuration.s3"],
        ),
        ("aws_workspacesweb_user_access_logging_settings", "kinesis_stream_arn") => Rule::new(
            "User access logging stream",
            &[("aws_kinesis_stream", &["arn"])],
            false,
            &[],
        ),
        ("aws_workspacesweb_browser_settings_association", "portal_arn") => Rule::new(
            "Secure Browser portal",
            &[("aws_workspacesweb_portal", &["portal_arn"])],
            false,
            &[],
        ),
        ("aws_workspacesweb_browser_settings_association", "browser_settings_arn") => Rule::new(
            "Secure Browser browser settings",
            &[(
                "aws_workspacesweb_browser_settings",
                &["browser_settings_arn"],
            )],
            false,
            &[],
        ),
        ("aws_workspacesweb_data_protection_settings_association", "portal_arn") => Rule::new(
            "Secure Browser portal",
            &[("aws_workspacesweb_portal", &["portal_arn"])],
            false,
            &[],
        ),
        (
            "aws_workspacesweb_data_protection_settings_association",
            "data_protection_settings_arn",
        ) => Rule::new(
            "Secure Browser data protection settings",
            &[(
                "aws_workspacesweb_data_protection_settings",
                &["data_protection_settings_arn"],
            )],
            false,
            &[],
        ),
        ("aws_workspacesweb_ip_access_settings_association", "portal_arn") => Rule::new(
            "Secure Browser portal",
            &[("aws_workspacesweb_portal", &["portal_arn"])],
            false,
            &[],
        ),
        ("aws_workspacesweb_ip_access_settings_association", "ip_access_settings_arn") => {
            Rule::new(
                "Secure Browser ip access settings",
                &[(
                    "aws_workspacesweb_ip_access_settings",
                    &["ip_access_settings_arn"],
                )],
                false,
                &[],
            )
        }
        ("aws_workspacesweb_network_settings_association", "portal_arn") => Rule::new(
            "Secure Browser portal",
            &[("aws_workspacesweb_portal", &["portal_arn"])],
            false,
            &[],
        ),
        ("aws_workspacesweb_network_settings_association", "network_settings_arn") => Rule::new(
            "Secure Browser network settings",
            &[(
                "aws_workspacesweb_network_settings",
                &["network_settings_arn"],
            )],
            false,
            &[],
        ),
        ("aws_workspacesweb_session_logger_association", "portal_arn") => Rule::new(
            "Secure Browser portal",
            &[("aws_workspacesweb_portal", &["portal_arn"])],
            false,
            &[],
        ),
        ("aws_workspacesweb_session_logger_association", "session_logger_arn") => Rule::new(
            "Secure Browser session logger",
            &[("aws_workspacesweb_session_logger", &["session_logger_arn"])],
            false,
            &[],
        ),
        ("aws_workspacesweb_trust_store_association", "portal_arn") => Rule::new(
            "Secure Browser portal",
            &[("aws_workspacesweb_portal", &["portal_arn"])],
            false,
            &[],
        ),
        ("aws_workspacesweb_trust_store_association", "trust_store_arn") => Rule::new(
            "Secure Browser trust store",
            &[("aws_workspacesweb_trust_store", &["trust_store_arn"])],
            false,
            &[],
        ),
        ("aws_workspacesweb_user_access_logging_settings_association", "portal_arn") => Rule::new(
            "Secure Browser portal",
            &[("aws_workspacesweb_portal", &["portal_arn"])],
            false,
            &[],
        ),
        (
            "aws_workspacesweb_user_access_logging_settings_association",
            "user_access_logging_settings_arn",
        ) => Rule::new(
            "Secure Browser user access logging settings",
            &[(
                "aws_workspacesweb_user_access_logging_settings",
                &["user_access_logging_settings_arn"],
            )],
            false,
            &[],
        ),
        ("aws_workspacesweb_user_settings_association", "portal_arn") => Rule::new(
            "Secure Browser portal",
            &[("aws_workspacesweb_portal", &["portal_arn"])],
            false,
            &[],
        ),
        ("aws_workspacesweb_user_settings_association", "user_settings_arn") => Rule::new(
            "Secure Browser user settings",
            &[("aws_workspacesweb_user_settings", &["user_settings_arn"])],
            false,
            &[],
        ),
        _ => return None,
    })
}

pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    rule(kind, path).map(|r| r.label)
}
pub(crate) fn indexed(kind: &str, path: &str) -> bool {
    configured_links::indexed(kind, path, rule)
}
pub(super) fn bindings(raw: &Input, reference: &AttributeReference) -> bool {
    configured_links::bindings(raw, reference, rule)
}
pub(super) fn annotate(
    nodes: &[TerraformEntity],
    references: &mut [AttributeReference],
    values: &[(&Value, &Value)],
) {
    configured_links::annotate(nodes, references, values, rule, qualifies)
}

fn qualifies(
    evidence: &configured_links::Evidence<'_>,
    reference: &AttributeReference,
    endpoint: usize,
    _: &str,
) -> bool {
    let (companion, configured_vpc) = match reference.attribute.as_str() {
        "vpc_config.subnet_ids" => (Some("vpc_config.security_group_ids"), None),
        "vpc_config.security_group_ids" => (Some("vpc_config.subnet_ids"), None),
        "vpc_configuration.subnet_ids" => (None, Some("vpc_configuration.0.vpc_id")),
        "subnet_ids" | "security_group_ids"
            if evidence.nodes[reference.target].resource_type
                == "aws_workspacesweb_network_settings" =>
        {
            let companion = if reference.attribute == "subnet_ids" {
                "security_group_ids"
            } else {
                "subnet_ids"
            };
            (Some(companion), Some("vpc_id"))
        }
        "subnet_ids" => (None, None),
        _ => return true,
    };
    let others = companion
        .and_then(|path| super::lambda_core::unique(evidence.refs, reference.target, path))
        .filter(|other| other.complete && other.issues.is_empty())
        .map_or(&[][..], |other| other.sources.as_slice());
    if configured_vpc.is_none() && reference.sources.len() + others.len() < 2 {
        return true;
    }
    evidence.text(endpoint, "vpc_id").is_some_and(|vpc| {
        configured_vpc.is_none_or(|path| evidence.text(reference.target, path) == Some(vpc))
            && reference
                .sources
                .iter()
                .chain(others)
                .all(|&source| evidence.text(source, "vpc_id") == Some(vpc))
    })
}
