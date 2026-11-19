use super::configured_links::{self, Evidence, Rule};
use crate::model::{AttributeReference, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;

fn rule(kind: &str, path: &str) -> Option<Rule> {
    Some(match (kind, path) {
        ("aws_ec2_client_vpn_endpoint", "server_certificate_arn") => Rule::new(
            "Server certificate",
            &[("aws_acm_certificate", &["arn"])],
            false,
            &[],
        ),
        ("aws_ec2_client_vpn_endpoint", "vpc_id") => {
            Rule::new("Client VPN VPC", &[("aws_vpc", &["id"])], false, &[])
        }
        ("aws_ec2_client_vpn_endpoint", "security_group_ids") => Rule::new(
            "Client VPN security groups",
            &[("aws_security_group", &["id"])],
            true,
            &[],
        ),
        ("aws_ec2_client_vpn_endpoint", "transit_gateway_configuration.transit_gateway_id") => {
            Rule::new(
                "Transit gateway",
                &[("aws_ec2_transit_gateway", &["id"])],
                false,
                &[],
            )
        }
        ("aws_ec2_client_vpn_endpoint", "authentication_options.root_certificate_chain_arn") => {
            Rule::new(
                "Client root certificate",
                &[("aws_acm_certificate", &["arn"])],
                false,
                &["authentication_options"],
            )
        }
        ("aws_ec2_client_vpn_endpoint", "authentication_options.active_directory_id") => Rule::new(
            "Authentication directory",
            &[("aws_directory_service_directory", &["id"])],
            false,
            &["authentication_options"],
        ),
        ("aws_ec2_client_vpn_endpoint", "authentication_options.saml_provider_arn") => Rule::new(
            "Authentication SAML provider",
            &[("aws_iam_saml_provider", &["arn"])],
            false,
            &["authentication_options"],
        ),
        (
            "aws_ec2_client_vpn_endpoint",
            "authentication_options.self_service_saml_provider_arn",
        ) => Rule::new(
            "Self-service SAML provider",
            &[("aws_iam_saml_provider", &["arn"])],
            false,
            &["authentication_options"],
        ),
        ("aws_ec2_client_vpn_endpoint", "client_connect_options.lambda_function_arn") => Rule::new(
            "Client connect function",
            &[("aws_lambda_function", &["arn"])],
            false,
            &[],
        ),
        ("aws_ec2_client_vpn_endpoint", "connection_log_options.cloudwatch_log_group") => {
            Rule::new(
                "Connection log group",
                &[("aws_cloudwatch_log_group", &["name"])],
                false,
                &[],
            )
        }
        ("aws_ec2_client_vpn_endpoint", "connection_log_options.cloudwatch_log_stream") => {
            Rule::new(
                "Connection log stream",
                &[("aws_cloudwatch_log_stream", &["name"])],
                false,
                &[],
            )
        }
        (
            "aws_ec2_client_vpn_authorization_rule"
            | "aws_ec2_client_vpn_route"
            | "aws_ec2_client_vpn_network_association",
            "client_vpn_endpoint_id",
        ) => Rule::new(
            "Client VPN endpoint",
            &[("aws_ec2_client_vpn_endpoint", &["id"])],
            false,
            &[],
        ),
        ("aws_ec2_client_vpn_network_association", "subnet_id") => {
            Rule::new("Associated subnet", &[("aws_subnet", &["id"])], false, &[])
        }
        ("aws_ec2_client_vpn_route", "target_vpc_subnet_id") => Rule::new(
            "Route target subnet",
            &[("aws_subnet", &["id"])],
            false,
            &[],
        ),
        _ => return None,
    })
}
pub(crate) fn resource(kind: &str) -> bool {
    matches!(
        kind,
        "aws_ec2_client_vpn_endpoint"
            | "aws_ec2_client_vpn_authorization_rule"
            | "aws_ec2_client_vpn_route"
            | "aws_ec2_client_vpn_network_association"
    )
}
pub(crate) fn indexed(kind: &str, path: &str) -> bool {
    resource(kind) && configured_links::indexed(kind, path, rule)
}
pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    rule(kind, path).map(|r| r.label)
}
pub(super) fn bindings(raw: &Input, r: &AttributeReference) -> bool {
    configured_links::bindings(raw, r, rule)
}
pub(super) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    values: &[(&Value, &Value)],
) {
    configured_links::annotate(nodes, refs, values, rule, qualifies)
}
fn qualifies(e: &Evidence<'_>, r: &AttributeReference, source: usize, path: &str) -> bool {
    let node = r.target;
    let kind = e.nodes[node].resource_type.as_str();
    if kind == "aws_ec2_client_vpn_network_association"
        || (kind == "aws_ec2_client_vpn_route" && r.attribute == "target_vpc_subnet_id")
    {
        let Some(endpoint) = e.configured_endpoint(
            node,
            "client_vpn_endpoint_id",
            "aws_ec2_client_vpn_endpoint",
            &["id"],
        ) else {
            return false;
        };
        if !e.absent(endpoint, "transit_gateway_configuration") {
            return false;
        }
        let field = if kind == "aws_ec2_client_vpn_route" {
            "target_vpc_subnet_id"
        } else {
            "subnet_id"
        };
        let Some(subnet) = e.configured_endpoint(node, field, "aws_subnet", &["id"]) else {
            return false;
        };
        let Some(vpc) = super::lambda_core::parent(e.nodes, e.refs, e.values, subnet) else {
            return false;
        };
        if !e.optional_vpc_matches(endpoint, vpc) {
            return false;
        }
        return kind != "aws_ec2_client_vpn_route"
            || e.refs.iter().any(|a| {
                a.attribute == "subnet_id"
                    && e.nodes[a.target].resource_type == "aws_ec2_client_vpn_network_association"
                    && e.configured_endpoint(a.target, "subnet_id", "aws_subnet", &["id"])
                        == Some(subnet)
                    && e.configured_endpoint(
                        a.target,
                        "client_vpn_endpoint_id",
                        "aws_ec2_client_vpn_endpoint",
                        &["id"],
                    ) == Some(endpoint)
            });
    }
    if kind != "aws_ec2_client_vpn_endpoint" {
        return true;
    }
    match r.attribute.as_str() {
        "vpc_id" | "security_group_ids" => {
            let Some(vpc) = e.configured_endpoint(node, "vpc_id", "aws_vpc", &["id"]) else {
                return false;
            };
            e.absent(node, "transit_gateway_configuration")
                && (!e.has_argument(node, "security_group_ids")
                    || e.absent(node, "security_group_ids")
                    || configured_links::network_context(
                        e,
                        node,
                        &[("security_group_ids", "aws_security_group")],
                    ) == Some(vpc))
        }
        "transit_gateway_configuration.transit_gateway_id" => {
            e.absent(node, "vpc_id") && e.absent(node, "security_group_ids")
        }
        p if p.starts_with("authentication_options.") => {
            path.rsplit_once('.').is_some_and(|(base, leaf)| {
                matches!(
                    (leaf, e.text(node, &format!("{base}.type"))),
                    (
                        "active_directory_id",
                        Some("directory-service-authentication")
                    ) | (
                        "root_certificate_chain_arn",
                        Some("certificate-authentication")
                    ) | (
                        "saml_provider_arn" | "self_service_saml_provider_arn",
                        Some("federated-authentication")
                    )
                )
            })
        }
        "client_connect_options.lambda_function_arn" => {
            e.value(node, "client_connect_options.0.enabled")
                .and_then(Value::as_bool)
                == Some(true)
        }
        p if p.starts_with("connection_log_options.") => {
            e.value(node, "connection_log_options.0.enabled")
                .and_then(Value::as_bool)
                == Some(true)
                && (p.ends_with("cloudwatch_log_group")
                    || e.same_endpoint(
                        node,
                        "connection_log_options.0.cloudwatch_log_group",
                        source,
                        "log_group_name",
                        "aws_cloudwatch_log_group",
                        "name",
                    ))
        }
        _ => true,
    }
}
#[cfg(test)]
#[path = "../../../tests/unit/semantic/client_vpn.rs"]
mod tests;
