use super::configured_links::{self, Evidence, Rule};
use crate::model::{AttributeReference, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;

fn rule(kind: &str, path: &str) -> Option<Rule> {
    Some(match (kind, path) {
        ("aws_cloudtrail", "sns_topic_name") => Rule::new(
            "Trail notification topic",
            &[("aws_sns_topic", &["name"])],
            false,
            &[],
        ),
        ("aws_cloudtrail_event_data_store", "kms_key_id") => Rule::new(
            "Event data store encryption key",
            &[
                ("aws_kms_key", &["arn", "key_id", "id"]),
                ("aws_kms_alias", &["arn", "name"]),
            ],
            false,
            &[],
        ),
        ("aws_controltower_baseline", "target_identifier") => Rule::new(
            "Control Tower target OU",
            &[("aws_organizations_organizational_unit", &["arn"])],
            false,
            &[],
        ),
        ("aws_controltower_control", "target_identifier") => Rule::new(
            "Control Tower target OU",
            &[("aws_organizations_organizational_unit", &["arn"])],
            false,
            &[],
        ),
        ("aws_licensemanager_association", "license_configuration_arn") => Rule::new(
            "License configuration",
            &[("aws_licensemanager_license_configuration", &["arn"])],
            false,
            &[],
        ),
        ("aws_licensemanager_association", "resource_arn") => Rule::new(
            "License assignment target",
            &[
                ("aws_instance", &["arn"]),
                ("aws_ec2_host", &["arn"]),
                ("aws_ami", &["arn"]),
                ("aws_ami_copy", &["arn"]),
                ("aws_ami_from_instance", &["arn"]),
            ],
            false,
            &[],
        ),
        ("aws_licensemanager_grant", "principal") => Rule::new(
            "License grant recipient",
            &[("aws_organizations_account", &["id"])],
            false,
            &[],
        )
        .with_explicit_destination()
        .decoded(account_root),
        ("aws_licensemanager_grant_accepter", "grant_arn") => Rule::new(
            "Accepted license grant",
            &[("aws_licensemanager_grant", &["arn"])],
            false,
            &[],
        )
        .with_explicit_destination(),
        ("aws_ram_principal_association", "resource_share_arn") => Rule::new(
            "RAM resource share",
            &[("aws_ram_resource_share", &["arn"])],
            false,
            &[],
        ),
        ("aws_ram_resource_association", "resource_share_arn") => Rule::new(
            "RAM resource share",
            &[("aws_ram_resource_share", &["arn"])],
            false,
            &[],
        ),
        ("aws_ram_resource_share_associations_exclusive", "resource_share_arn") => Rule::new(
            "RAM resource share",
            &[("aws_ram_resource_share", &["arn"])],
            false,
            &[],
        ),
        ("aws_ram_principal_association", "principal") => Rule::new(
            "RAM recipient",
            &[
                ("aws_organizations_account", &["id"]),
                ("aws_organizations_organization", &["arn"]),
                ("aws_organizations_organizational_unit", &["arn"]),
                ("aws_iam_role", &["arn"]),
                ("aws_iam_user", &["arn"]),
            ],
            false,
            &[],
        )
        .with_explicit_destination(),
        ("aws_ram_resource_association", "resource_arn") => Rule::new(
            "Shared resource",
            &[
                ("aws_subnet", &["arn"]),
                ("aws_ec2_transit_gateway", &["arn"]),
                ("aws_ec2_transit_gateway_route_table", &["arn"]),
                ("aws_ec2_managed_prefix_list", &["arn"]),
                ("aws_route53_resolver_rule", &["arn"]),
                ("aws_route53_resolver_firewall_rule_group", &["arn"]),
                ("aws_acmpca_certificate_authority", &["arn"]),
                ("aws_networkmanager_core_network", &["arn"]),
                ("aws_resourcegroups_group", &["arn"]),
                ("aws_ec2_capacity_reservation", &["arn"]),
                ("aws_servicecatalogappregistry_application", &["arn"]),
                ("aws_vpclattice_service", &["arn"]),
                ("aws_vpclattice_service_network", &["arn"]),
                ("aws_glue_catalog_database", &["arn"]),
                ("aws_glue_catalog_table", &["arn"]),
                ("aws_backup_vault", &["arn"]),
            ],
            false,
            &[],
        ),
        ("aws_ram_resource_share", "permission_arns") => Rule::new(
            "RAM custom permissions",
            &[("aws_ram_permission", &["arn"])],
            true,
            &[],
        ),
        ("aws_ram_resource_share_accepter", "share_arn") => Rule::new(
            "Accepted RAM resource share",
            &[("aws_ram_resource_share", &["arn"])],
            false,
            &[],
        )
        .with_explicit_destination(),
        ("aws_ram_resource_share_associations_exclusive", "resource_arns") => Rule::new(
            "Shared resources",
            &[
                ("aws_subnet", &["arn"]),
                ("aws_ec2_transit_gateway", &["arn"]),
                ("aws_ec2_transit_gateway_route_table", &["arn"]),
                ("aws_ec2_managed_prefix_list", &["arn"]),
                ("aws_route53_resolver_rule", &["arn"]),
                ("aws_route53_resolver_firewall_rule_group", &["arn"]),
                ("aws_acmpca_certificate_authority", &["arn"]),
                ("aws_networkmanager_core_network", &["arn"]),
                ("aws_resourcegroups_group", &["arn"]),
                ("aws_ec2_capacity_reservation", &["arn"]),
                ("aws_servicecatalogappregistry_application", &["arn"]),
                ("aws_vpclattice_service", &["arn"]),
                ("aws_vpclattice_service_network", &["arn"]),
                ("aws_glue_catalog_database", &["arn"]),
                ("aws_glue_catalog_table", &["arn"]),
                ("aws_backup_vault", &["arn"]),
            ],
            true,
            &[],
        ),
        ("aws_ram_resource_share_associations_exclusive", "principals") => Rule::new(
            "RAM recipients",
            &[
                ("aws_organizations_account", &["id"]),
                ("aws_organizations_organization", &["arn"]),
                ("aws_organizations_organizational_unit", &["arn"]),
                ("aws_iam_role", &["arn"]),
                ("aws_iam_user", &["arn"]),
            ],
            true,
            &[],
        )
        .with_explicit_destination(),
        ("aws_resourceexplorer2_view", "scope") => Rule::new(
            "Resource Explorer scope",
            &[
                ("aws_organizations_account", &["id"]),
                ("aws_organizations_organizational_unit", &["arn"]),
                ("aws_organizations_organization", &["arn"]),
            ],
            false,
            &[],
        )
        .with_explicit_destination()
        .decoded(explorer_scope),
        ("aws_resourcegroups_resource", "group_arn") => Rule::new(
            "Resource group",
            &[("aws_resourcegroups_group", &["arn"])],
            false,
            &[],
        ),
        ("aws_resourcegroups_resource", "resource_arn") => Rule::new(
            "Grouped resource",
            &[
                ("aws_instance", &["arn"]),
                ("aws_vpc", &["arn"]),
                ("aws_subnet", &["arn"]),
                ("aws_s3_bucket", &["arn"]),
                ("aws_lambda_function", &["arn"]),
                ("aws_dynamodb_table", &["arn"]),
                ("aws_sns_topic", &["arn"]),
                ("aws_sqs_queue", &["arn"]),
                ("aws_rds_cluster", &["arn"]),
                ("aws_db_instance", &["arn"]),
                ("aws_cloudwatch_log_group", &["arn"]),
                ("aws_ecs_cluster", &["arn"]),
                ("aws_eks_cluster", &["arn"]),
                ("aws_kms_key", &["arn"]),
            ],
            false,
            &[],
        ),
        ("aws_servicecatalogappregistry_attribute_group_association", "application_id") => {
            Rule::new(
                "AppRegistry application",
                &[("aws_servicecatalogappregistry_application", &["id"])],
                false,
                &[],
            )
        }
        ("aws_servicecatalogappregistry_attribute_group_association", "attribute_group_id") => {
            Rule::new(
                "AppRegistry attribute group",
                &[("aws_servicecatalogappregistry_attribute_group", &["id"])],
                false,
                &[],
            )
        }
        ("aws_servicecatalog_budget_resource_association", "budget_name") => Rule::new(
            "Associated budget",
            &[("aws_budgets_budget", &["name"])],
            false,
            &[],
        )
        .in_control_plane_account(),
        ("aws_servicecatalog_budget_resource_association", "resource_id") => Rule::new(
            "Budgeted catalog resource",
            &[
                ("aws_servicecatalog_portfolio", &["id"]),
                ("aws_servicecatalog_product", &["id"]),
            ],
            false,
            &[],
        ),
        ("aws_servicecatalog_constraint", "portfolio_id") => Rule::new(
            "Service Catalog portfolio",
            &[("aws_servicecatalog_portfolio", &["id"])],
            false,
            &[],
        ),
        ("aws_servicecatalog_portfolio_share", "portfolio_id") => Rule::new(
            "Service Catalog portfolio",
            &[("aws_servicecatalog_portfolio", &["id"])],
            false,
            &[],
        ),
        ("aws_servicecatalog_principal_portfolio_association", "portfolio_id") => Rule::new(
            "Service Catalog portfolio",
            &[("aws_servicecatalog_portfolio", &["id"])],
            false,
            &[],
        ),
        ("aws_servicecatalog_product_portfolio_association", "portfolio_id") => Rule::new(
            "Service Catalog portfolio",
            &[("aws_servicecatalog_portfolio", &["id"])],
            false,
            &[],
        ),
        ("aws_servicecatalog_constraint", "product_id") => Rule::new(
            "Service Catalog product",
            &[("aws_servicecatalog_product", &["id"])],
            false,
            &[],
        ),
        ("aws_servicecatalog_product_portfolio_association", "product_id") => Rule::new(
            "Service Catalog product",
            &[("aws_servicecatalog_product", &["id"])],
            false,
            &[],
        ),
        ("aws_servicecatalog_portfolio_share", "principal_id") => Rule::new(
            "Portfolio share recipient",
            &[
                ("aws_organizations_account", &["id"]),
                ("aws_organizations_organization", &["arn", "id"]),
                ("aws_organizations_organizational_unit", &["arn", "id"]),
            ],
            false,
            &[],
        )
        .with_explicit_destination(),
        ("aws_servicecatalog_principal_portfolio_association", "principal_arn") => Rule::new(
            "Portfolio IAM principal",
            &[
                ("aws_iam_role", &["arn"]),
                ("aws_iam_user", &["arn"]),
                ("aws_iam_group", &["arn"]),
            ],
            false,
            &[],
        )
        .in_control_plane_account(),
        ("aws_servicecatalog_product_portfolio_association", "source_portfolio_id") => Rule::new(
            "Source portfolio",
            &[("aws_servicecatalog_portfolio", &["id"])],
            false,
            &[],
        ),
        ("aws_servicecatalog_tag_option_resource_association", "tag_option_id") => Rule::new(
            "Service Catalog tag option",
            &[("aws_servicecatalog_tag_option", &["id"])],
            false,
            &[],
        ),
        ("aws_servicecatalog_tag_option_resource_association", "resource_id") => Rule::new(
            "Tagged catalog resource",
            &[
                ("aws_servicecatalog_portfolio", &["id"]),
                ("aws_servicecatalog_product", &["id"]),
            ],
            false,
            &[],
        ),
        ("aws_servicecatalog_provisioned_product", "product_id") => Rule::new(
            "Provisioned product definition",
            &[("aws_servicecatalog_product", &["id"])],
            false,
            &[],
        ),
        ("aws_servicecatalog_provisioned_product", "product_name") => Rule::new(
            "Provisioned product definition",
            &[("aws_servicecatalog_product", &["name"])],
            false,
            &[],
        ),
        ("aws_servicecatalog_provisioned_product", "provisioning_artifact_id") => Rule::new(
            "Provisioned product artifact",
            &[(
                "aws_servicecatalog_provisioning_artifact",
                &["provisioning_artifact_id"],
            )],
            false,
            &[],
        ),
        ("aws_servicecatalog_provisioned_product", "provisioning_artifact_name") => Rule::new(
            "Provisioned product artifact",
            &[("aws_servicecatalog_provisioning_artifact", &["name"])],
            false,
            &[],
        ),
        ("aws_servicecatalog_provisioned_product", "notification_arns") => Rule::new(
            "Provisioned product notification topics",
            &[("aws_sns_topic", &["arn"])],
            true,
            &[],
        ),
        ("aws_servicecatalog_provisioning_artifact", "product_id") => Rule::new(
            "Artifact product",
            &[("aws_servicecatalog_product", &["id"])],
            false,
            &[],
        ),
        ("aws_servicecatalog_service_action", "definition.name") => Rule::new(
            "Service action SSM document",
            &[("aws_ssm_document", &["name"])],
            false,
            &[],
        ),
        ("aws_servicecatalog_service_action", "definition.assume_role") => Rule::new(
            "Service action role",
            &[("aws_iam_role", &["arn"])],
            false,
            &[],
        ),
        ("aws_servicequotas_auto_management", "notification_arn") => Rule::new(
            "Quota management notification configuration",
            &[("aws_notifications_notification_configuration", &["arn"])],
            false,
            &[],
        )
        .in_control_plane_account(),
        ("aws_ssm_activation", "iam_role") => Rule::new(
            "Activation IAM role",
            &[("aws_iam_role", &["name"])],
            false,
            &[],
        ),
        ("aws_ssm_association", "name") => Rule::new(
            "Association document",
            &[("aws_ssm_document", &["name"])],
            false,
            &[],
        ),
        ("aws_ssm_association", "output_location.s3_bucket_name") => Rule::new(
            "Association output bucket",
            &[("aws_s3_bucket", &["bucket", "id"])],
            false,
            &[],
        ),
        ("aws_ssm_parameter", "key_id") => Rule::new(
            "SecureString customer key",
            &[
                ("aws_kms_key", &["arn", "id", "key_id"]),
                ("aws_kms_alias", &["arn", "name"]),
            ],
            false,
            &[],
        ),
        ("aws_ssm_resource_data_sync", "s3_destination.bucket_name") => Rule::new(
            "Inventory destination bucket",
            &[("aws_s3_bucket", &["bucket", "id"])],
            false,
            &[],
        )
        .with_explicit_destination(),
        ("aws_ssm_resource_data_sync", "s3_destination.kms_key_arn") => Rule::new(
            "Inventory destination key",
            &[("aws_kms_key", &["arn"])],
            false,
            &[],
        )
        .with_explicit_destination(),
        (
            "aws_ssmquicksetup_configuration_manager",
            "configuration_definition.local_deployment_administration_role_arn",
        ) => Rule::new(
            "Quick Setup administration role",
            &[("aws_iam_role", &["arn"])],
            false,
            &["configuration_definition"],
        )
        .in_control_plane_account(),
        (
            "aws_ssmquicksetup_configuration_manager",
            "configuration_definition.local_deployment_execution_role_name",
        ) => Rule::new(
            "Quick Setup execution role",
            &[("aws_iam_role", &["name"])],
            false,
            &["configuration_definition"],
        )
        .in_control_plane_account(),
        ("aws_ssm_default_patch_baseline", "baseline_id") => Rule::new(
            "SSM patch baseline",
            &[("aws_ssm_patch_baseline", &["id"])],
            false,
            &[],
        ),
        ("aws_ssm_patch_group", "baseline_id") => Rule::new(
            "SSM patch baseline",
            &[("aws_ssm_patch_baseline", &["id"])],
            false,
            &[],
        ),
        _ => return None,
    })
}

pub(crate) fn has_indexed(kind: &str) -> bool {
    matches!(
        kind,
        "aws_servicecatalog_service_action"
            | "aws_ssm_association"
            | "aws_ssm_resource_data_sync"
            | "aws_ssmquicksetup_configuration_manager"
    )
}

pub(crate) fn indexed(kind: &str, path: &str) -> bool {
    configured_links::indexed(kind, path, rule)
}

pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    rule(kind, path).map(|rule| rule.label)
}

pub(super) fn bindings(raw: &Input, reference: &AttributeReference) -> bool {
    configured_links::bindings(raw, reference, rule)
}

pub(super) fn annotate(
    nodes: &[TerraformEntity],
    references: &mut [AttributeReference],
    values: &[(&Value, &Value)],
) {
    configured_links::annotate(nodes, references, values, rule, qualifies);
}

fn qualifies(e: &Evidence<'_>, r: &AttributeReference, source: usize, path: &str) -> bool {
    match (
        e.nodes[r.target].resource_type.as_str(),
        r.attribute.as_str(),
    ) {
        ("aws_controltower_baseline" | "aws_controltower_control", "target_identifier") => e
            .text(r.target, path)
            .and_then(organizations_ou_account)
            .is_some_and(|account| {
                e.nodes[source]
                    .provider_configuration
                    .as_ref()
                    .and_then(|config| config.account.as_ref())
                    .is_none_or(|owner| owner.as_str() == account)
            }),
        ("aws_licensemanager_association", "license_configuration_arn") => {
            e.text(r.target, path).is_some_and(|v| {
                arn_resource(v, "license-manager", "license-configuration:").is_some()
            })
        }
        ("aws_licensemanager_association", "resource_arn") => {
            let prefix = match e.nodes[source].resource_type.as_str() {
                "aws_instance" => "instance/",
                "aws_ec2_host" => "dedicated-host/",
                _ => "image/",
            };
            e.text(r.target, path)
                .is_some_and(|v| arn_resource(v, "ec2", prefix).is_some())
        }
        ("aws_licensemanager_grant_accepter", "grant_arn") => e
            .text(r.target, path)
            .and_then(|v| arn_resource(v, "license-manager", "grant:"))
            .is_some_and(|(region, account, id)| {
                region.is_empty()
                    && !id.is_empty()
                    && endpoint_account(e, source, account)
                    && e.text(source, "home_region")
                        .is_some_and(|home| endpoint_region(e, r.target, home))
            }),
        ("aws_ram_principal_association", "principal")
        | ("aws_ram_resource_share_associations_exclusive", "principals") => {
            ram_principal(e, source)
        }
        ("aws_ram_resource_share", "permission_arns") => {
            ram_arn(e, r.target, source, "permission/")
        }
        ("aws_ram_resource_share_accepter", "share_arn") => {
            ram_arn(e, r.target, source, "resource-share/")
        }
        ("aws_resourceexplorer2_view", "scope") => match e.nodes[source].resource_type.as_str() {
            "aws_organizations_account" => e
                .text(source, "id")
                .is_some_and(|account| endpoint_account(e, r.target, account)),
            "aws_organizations_organizational_unit" | "aws_organizations_organization" => {
                ram_principal(e, source)
                    && configured_links::same_account(
                        e.nodes[r.target].provider_configuration.as_ref(),
                        e.nodes[source].provider_configuration.as_ref(),
                    )
            }
            _ => false,
        },
        ("aws_resourceexplorer2_view", _) => false,
        ("aws_servicecatalog_budget_resource_association", "budget_name") => {
            match e.value(source, "account_id") {
                Some(Value::Null) => true,
                Some(Value::String(account)) => endpoint_account(e, r.target, account),
                _ => false,
            }
        }
        ("aws_servicecatalog_portfolio_share", "principal_id") => {
            let recipient = e.text(r.target, path).unwrap_or_default();
            match (
                e.text(r.target, "type"),
                e.nodes[source].resource_type.as_str(),
            ) {
                (Some("ACCOUNT" | "ORGANIZATION_MEMBER_ACCOUNT"), "aws_organizations_account") => {
                    crate::model::AwsAccountScope::new(recipient).is_some()
                }
                (Some("ORGANIZATION"), "aws_organizations_organization") => {
                    recipient.starts_with("o-")
                        || principal_arn(e, source, "organizations", "organization/o-")
                }
                (Some("ORGANIZATIONAL_UNIT"), "aws_organizations_organizational_unit") => {
                    recipient.starts_with("ou-")
                        || organizations_ou_account(recipient)
                            .is_some_and(|account| endpoint_account(e, source, account))
                }
                _ => false,
            }
        }
        ("aws_servicecatalog_principal_portfolio_association", "principal_arn") => {
            let principal_type = e.text(r.target, "principal_type");
            (principal_type == Some("IAM") || e.absent(r.target, "principal_type"))
                && match e.nodes[source].resource_type.as_str() {
                    "aws_iam_group" => principal_arn(e, source, "iam", "group/"),
                    _ => ram_principal(e, source),
                }
        }
        (
            "aws_servicecatalog_provisioned_product",
            "provisioning_artifact_id" | "provisioning_artifact_name",
        ) => {
            let kind = "aws_servicecatalog_product";
            let product = if e.has_argument(r.target, "product_id") {
                e.configured_endpoint(r.target, "product_id", kind, &["id"])
            } else {
                e.configured_endpoint(r.target, "product_name", kind, &["name"])
            };
            product.is_some_and(|p| {
                e.configured_endpoint(source, "product_id", kind, &["id"]) == Some(p)
            })
        }
        ("aws_servicecatalog_service_action", "definition.name" | "definition.assume_role") => {
            path.rsplit_once('.').is_some_and(|(block, _)| {
                e.text(r.target, &format!("{block}.type")) == Some("SSM_AUTOMATION")
                    && (r.attribute == "definition.name"
                        || principal_arn(e, source, "iam", "role/"))
            })
        }
        ("aws_servicequotas_auto_management", "notification_arn") => {
            principal_arn(e, source, "notifications", "configuration/")
        }
        ("aws_ssm_parameter", "key_id") => {
            e.has_argument(r.target, path)
                && e.text(r.target, "type") == Some("SecureString")
                && e.text(r.target, path)
                    .is_some_and(|key| key != "alias/aws/ssm" && !key.ends_with(":alias/aws/ssm"))
        }
        (
            "aws_ssm_resource_data_sync",
            "s3_destination.bucket_name" | "s3_destination.kms_key_arn",
        ) => path.rsplit_once('.').is_some_and(|(block, _)| {
            e.source_region(r.target, &format!("{block}.region"), source)
                && configured_links::same_account(
                    e.nodes[r.target].provider_configuration.as_ref(),
                    e.nodes[source].provider_configuration.as_ref(),
                )
        }),
        ("aws_ssmquicksetup_configuration_manager", _) => {
            path.rsplit_once('.').is_some_and(|(block, _)| {
                e.text(r.target, &format!("{block}.type")).is_some()
                    && (r.attribute.ends_with("_name") || principal_arn(e, source, "iam", "role/"))
            })
        }
        ("aws_ssm_default_patch_baseline", "baseline_id") => e
            .text(r.target, "operating_system")
            .is_some_and(|os| e.text(source, "operating_system") == Some(os)),
        _ => true,
    }
}

fn organizations_ou_account(arn: &str) -> Option<&str> {
    let mut fields = arn.splitn(6, ':');
    match (
        fields.next(),
        fields.next(),
        fields.next(),
        fields.next(),
        fields.next(),
        fields.next(),
    ) {
        (
            Some("arn"),
            Some("aws" | "aws-us-gov" | "aws-cn"),
            Some("organizations"),
            Some(""),
            Some(account),
            Some(resource),
        ) if crate::model::AwsAccountScope::new(account).is_some()
            && resource
                .strip_prefix("ou/o-")
                .and_then(|path| path.split_once("/ou-"))
                .is_some_and(|(org, ou)| {
                    !org.is_empty() && !ou.is_empty() && !ou.contains('/')
                }) =>
        {
            Some(account)
        }
        _ => None,
    }
}

fn account_root<'a>(_: &str, value: &'a Value) -> Option<std::collections::BTreeSet<&'a str>> {
    let (region, account, identity) = arn_resource(value.as_str()?, "iam", "root")?;
    (region.is_empty()
        && identity.is_empty()
        && crate::model::AwsAccountScope::new(account).is_some())
    .then(|| [account].into())
}

fn arn_resource<'a>(
    value: &'a str,
    service: &str,
    prefix: &str,
) -> Option<(&'a str, &'a str, &'a str)> {
    let mut fields = value.splitn(6, ':');
    if fields.next()? != "arn"
        || !matches!(fields.next()?, "aws" | "aws-cn" | "aws-us-gov")
        || fields.next()? != service
    {
        return None;
    }
    let region = fields.next()?;
    let account = fields.next()?;
    let identity = fields.next()?.strip_prefix(prefix)?;
    Some((region, account, identity))
}

fn endpoint_account(e: &Evidence<'_>, node: usize, account: &str) -> bool {
    e.nodes[node]
        .provider_configuration
        .as_ref()
        .and_then(|config| config.account.as_ref())
        .is_some_and(|owner| owner.as_str() == account)
}

fn endpoint_region(e: &Evidence<'_>, node: usize, region: &str) -> bool {
    super::scope::scope(&e.nodes[node])
        .and_then(|scope| scope.region)
        .is_some_and(|effective| effective.0 == region)
}

fn ram_arn(e: &Evidence<'_>, target: usize, source: usize, prefix: &str) -> bool {
    e.text(source, "arn")
        .and_then(|v| arn_resource(v, "ram", prefix))
        .is_some_and(|(region, account, id)| {
            !id.is_empty()
                && endpoint_region(e, source, region)
                && endpoint_region(e, target, region)
                && endpoint_account(e, source, account)
        })
}

fn ram_principal(e: &Evidence<'_>, source: usize) -> bool {
    match e.nodes[source].resource_type.as_str() {
        "aws_organizations_account" => e
            .text(source, "id")
            .and_then(crate::model::AwsAccountScope::new)
            .is_some(),
        "aws_organizations_organizational_unit" => e
            .text(source, "arn")
            .and_then(organizations_ou_account)
            .is_some_and(|account| endpoint_account(e, source, account)),
        "aws_organizations_organization" => {
            principal_arn(e, source, "organizations", "organization/o-")
        }
        "aws_iam_role" => principal_arn(e, source, "iam", "role/"),
        "aws_iam_user" => principal_arn(e, source, "iam", "user/"),
        _ => false,
    }
}

fn principal_arn(e: &Evidence<'_>, source: usize, service: &str, prefix: &str) -> bool {
    e.text(source, "arn")
        .and_then(|arn| arn_resource(arn, service, prefix))
        .is_some_and(|(region, account, id)| {
            region.is_empty() && !id.is_empty() && endpoint_account(e, source, account)
        })
}

fn explorer_scope<'a>(path: &str, value: &'a Value) -> Option<std::collections::BTreeSet<&'a str>> {
    account_root(path, value).or_else(|| super::nested_references::leaf_values(path, value))
}

#[cfg(test)]
#[path = "../../../tests/unit/semantic/operations.rs"]
mod tests;
