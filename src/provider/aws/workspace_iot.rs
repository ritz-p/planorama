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
