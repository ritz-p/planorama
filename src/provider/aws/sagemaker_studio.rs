use super::configured_links::{self, Evidence, Rule};
use crate::model::{AttributeReference, Graph, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;

pub(crate) fn resource(kind: &str) -> bool {
    matches!(
        kind,
        "aws_sagemaker_app"
            | "aws_sagemaker_code_repository"
            | "aws_sagemaker_domain"
            | "aws_sagemaker_image"
            | "aws_sagemaker_image_version"
            | "aws_sagemaker_notebook_instance"
            | "aws_sagemaker_space"
            | "aws_sagemaker_user_profile"
    )
}
fn rule(kind: &str, path: &str) -> Option<Rule> {
    Some(match (kind, path) {
        ("aws_sagemaker_app", "domain_id") => Rule::new(
            "Studio domain",
            &[("aws_sagemaker_domain", &["id"])],
            false,
            &[],
        ),
        ("aws_sagemaker_app", "user_profile_name") => Rule::new(
            "App user profile",
            &[("aws_sagemaker_user_profile", &["user_profile_name"])],
            false,
            &[],
        ),
        ("aws_sagemaker_app", "space_name") => Rule::new(
            "App space",
            &[("aws_sagemaker_space", &["space_name"])],
            false,
            &[],
        ),
        ("aws_sagemaker_code_repository", "git_config.secret_arn") => Rule::new(
            "Repository secret",
            &[("aws_secretsmanager_secret", &["arn"])],
            false,
            &[],
        ),
        ("aws_sagemaker_domain", "vpc_id") => {
            Rule::new("Domain VPC", &[("aws_vpc", &["id"])], false, &[])
        }
        ("aws_sagemaker_domain", "subnet_ids") => {
            Rule::new("Domain subnets", &[("aws_subnet", &["id"])], true, &[])
        }
        ("aws_sagemaker_domain", "default_user_settings.execution_role") => Rule::new(
            "Default user role",
            &[("aws_iam_role", &["arn"])],
            false,
            &[],
        ),
        ("aws_sagemaker_domain", "default_user_settings.security_groups") => Rule::new(
            "Default user security groups",
            &[("aws_security_group", &["id"])],
            true,
            &[],
        ),
        ("aws_sagemaker_image", "role_arn") => {
            Rule::new("Image role", &[("aws_iam_role", &["arn"])], false, &[])
        }
        ("aws_sagemaker_image_version", "image_name") => Rule::new(
            "Version image",
            &[("aws_sagemaker_image", &["image_name"])],
            false,
            &[],
        ),
        ("aws_sagemaker_notebook_instance", "role_arn") => {
            Rule::new("Notebook role", &[("aws_iam_role", &["arn"])], false, &[])
        }
        ("aws_sagemaker_notebook_instance", "subnet_id") => {
            Rule::new("Notebook subnet", &[("aws_subnet", &["id"])], false, &[])
        }
        ("aws_sagemaker_notebook_instance", "security_groups") => Rule::new(
            "Notebook security groups",
            &[("aws_security_group", &["id"])],
            true,
            &[],
        ),
        ("aws_sagemaker_notebook_instance", "kms_key_id") => Rule::new(
            "Notebook key",
            &[("aws_kms_key", &["arn", "key_id"])],
            false,
            &[],
        ),
        ("aws_sagemaker_notebook_instance", "lifecycle_config_name") => Rule::new(
            "Notebook lifecycle configuration",
            &[(
                "aws_sagemaker_notebook_instance_lifecycle_configuration",
                &["name"],
            )],
            false,
            &[],
        ),
        ("aws_sagemaker_notebook_instance", "default_code_repository") => Rule::new(
            "Default code repository",
            &[("aws_sagemaker_code_repository", &["code_repository_name"])],
            false,
            &[],
        ),
        ("aws_sagemaker_space", "domain_id") => Rule::new(
            "Studio domain",
            &[("aws_sagemaker_domain", &["id"])],
            false,
            &[],
        ),
        ("aws_sagemaker_space", "ownership_settings.owner_user_profile_name") => Rule::new(
            "Private space owner",
            &[("aws_sagemaker_user_profile", &["user_profile_name"])],
            false,
            &[],
        ),
        ("aws_sagemaker_user_profile", "domain_id") => Rule::new(
            "Studio domain",
            &[("aws_sagemaker_domain", &["id"])],
            false,
            &[],
        ),
        ("aws_sagemaker_user_profile", "user_settings.execution_role") => {
            Rule::new("Profile role", &[("aws_iam_role", &["arn"])], false, &[])
        }
        ("aws_sagemaker_user_profile", "user_settings.security_groups") => Rule::new(
            "Profile security groups",
            &[("aws_security_group", &["id"])],
            true,
            &[],
        ),
        ("aws_sagemaker_app", "resource_spec.lifecycle_config_arn") => Rule::new(
            "Studio lifecycle configuration",
            &[("aws_sagemaker_studio_lifecycle_config", &["arn"])],
            false,
            &[],
        ),
        ("aws_sagemaker_app", "resource_spec.sagemaker_image_arn") => Rule::new(
            "Studio image",
            &[("aws_sagemaker_image", &["arn"])],
            false,
            &[],
        ),
        ("aws_sagemaker_app", "resource_spec.sagemaker_image_version_arn") => Rule::new(
            "Studio image version",
            &[("aws_sagemaker_image_version", &["arn"])],
            false,
            &[],
        ),
        (
            "aws_sagemaker_domain",
            "default_space_settings.custom_file_system_config.efs_file_system_config.file_system_id",
        ) => Rule::new(
            "Configured EFS file system",
            &[("aws_efs_file_system", &["id"])],
            false,
            &["default_space_settings.custom_file_system_config"],
        ),
        (
            "aws_sagemaker_domain",
            "default_space_settings.jupyter_lab_app_settings.built_in_lifecycle_config_arn"
            | "default_user_settings.code_editor_app_settings.built_in_lifecycle_config_arn"
            | "default_user_settings.jupyter_lab_app_settings.built_in_lifecycle_config_arn",
        ) => Rule::new(
            "Built-in lifecycle configuration",
            &[("aws_sagemaker_studio_lifecycle_config", &["arn"])],
            false,
            &[],
        ),
        (
            "aws_sagemaker_domain",
            "default_space_settings.jupyter_lab_app_settings.lifecycle_config_arns"
            | "default_space_settings.jupyter_server_app_settings.lifecycle_config_arns"
            | "default_space_settings.kernel_gateway_app_settings.lifecycle_config_arns"
            | "default_user_settings.code_editor_app_settings.lifecycle_config_arns"
            | "default_user_settings.jupyter_lab_app_settings.lifecycle_config_arns"
            | "default_user_settings.jupyter_server_app_settings.lifecycle_config_arns"
            | "default_user_settings.kernel_gateway_app_settings.lifecycle_config_arns",
        ) => Rule::new(
            "Studio lifecycle configurations",
            &[("aws_sagemaker_studio_lifecycle_config", &["arn"])],
            true,
            &[],
        ),
        (
            "aws_sagemaker_domain",
            "default_space_settings.jupyter_lab_app_settings.default_resource_spec.lifecycle_config_arn"
            | "default_space_settings.jupyter_server_app_settings.default_resource_spec.lifecycle_config_arn"
            | "default_space_settings.kernel_gateway_app_settings.default_resource_spec.lifecycle_config_arn"
            | "default_user_settings.code_editor_app_settings.default_resource_spec.lifecycle_config_arn"
            | "default_user_settings.jupyter_lab_app_settings.default_resource_spec.lifecycle_config_arn"
            | "default_user_settings.jupyter_server_app_settings.default_resource_spec.lifecycle_config_arn"
            | "default_user_settings.kernel_gateway_app_settings.default_resource_spec.lifecycle_config_arn"
            | "default_user_settings.r_session_app_settings.default_resource_spec.lifecycle_config_arn"
            | "default_user_settings.tensor_board_app_settings.default_resource_spec.lifecycle_config_arn",
        ) => Rule::new(
            "Studio lifecycle configuration",
            &[("aws_sagemaker_studio_lifecycle_config", &["arn"])],
            false,
            &[],
        ),
        (
            "aws_sagemaker_domain",
            "default_space_settings.jupyter_lab_app_settings.default_resource_spec.sagemaker_image_arn"
            | "default_space_settings.jupyter_server_app_settings.default_resource_spec.sagemaker_image_arn"
            | "default_space_settings.kernel_gateway_app_settings.default_resource_spec.sagemaker_image_arn"
            | "default_user_settings.code_editor_app_settings.default_resource_spec.sagemaker_image_arn"
            | "default_user_settings.jupyter_lab_app_settings.default_resource_spec.sagemaker_image_arn"
            | "default_user_settings.jupyter_server_app_settings.default_resource_spec.sagemaker_image_arn"
            | "default_user_settings.kernel_gateway_app_settings.default_resource_spec.sagemaker_image_arn"
            | "default_user_settings.r_session_app_settings.default_resource_spec.sagemaker_image_arn"
            | "default_user_settings.tensor_board_app_settings.default_resource_spec.sagemaker_image_arn",
        ) => Rule::new(
            "Studio image",
            &[("aws_sagemaker_image", &["arn"])],
            false,
            &[],
        ),
        (
            "aws_sagemaker_domain",
            "default_space_settings.jupyter_lab_app_settings.default_resource_spec.sagemaker_image_version_arn"
            | "default_space_settings.jupyter_server_app_settings.default_resource_spec.sagemaker_image_version_arn"
            | "default_space_settings.kernel_gateway_app_settings.default_resource_spec.sagemaker_image_version_arn"
            | "default_user_settings.code_editor_app_settings.default_resource_spec.sagemaker_image_version_arn"
            | "default_user_settings.jupyter_lab_app_settings.default_resource_spec.sagemaker_image_version_arn"
            | "default_user_settings.jupyter_server_app_settings.default_resource_spec.sagemaker_image_version_arn"
            | "default_user_settings.kernel_gateway_app_settings.default_resource_spec.sagemaker_image_version_arn"
            | "default_user_settings.r_session_app_settings.default_resource_spec.sagemaker_image_version_arn"
            | "default_user_settings.tensor_board_app_settings.default_resource_spec.sagemaker_image_version_arn",
        ) => Rule::new(
            "Studio image version",
            &[("aws_sagemaker_image_version", &["arn"])],
            false,
            &[],
        ),
        (
            "aws_sagemaker_domain",
            "default_user_settings.custom_file_system_config.efs_file_system_config.file_system_id",
        ) => Rule::new(
            "Configured EFS file system",
            &[("aws_efs_file_system", &["id"])],
            false,
            &["default_user_settings.custom_file_system_config"],
        ),
        (
            "aws_sagemaker_space",
            "space_settings.code_editor_app_settings.default_resource_spec.lifecycle_config_arn"
            | "space_settings.jupyter_lab_app_settings.default_resource_spec.lifecycle_config_arn"
            | "space_settings.jupyter_server_app_settings.default_resource_spec.lifecycle_config_arn"
            | "space_settings.kernel_gateway_app_settings.default_resource_spec.lifecycle_config_arn",
        ) => Rule::new(
            "Studio lifecycle configuration",
            &[("aws_sagemaker_studio_lifecycle_config", &["arn"])],
            false,
            &[],
        ),
        (
            "aws_sagemaker_space",
            "space_settings.code_editor_app_settings.default_resource_spec.sagemaker_image_arn"
            | "space_settings.jupyter_lab_app_settings.default_resource_spec.sagemaker_image_arn"
            | "space_settings.jupyter_server_app_settings.default_resource_spec.sagemaker_image_arn"
            | "space_settings.kernel_gateway_app_settings.default_resource_spec.sagemaker_image_arn",
        ) => Rule::new(
            "Studio image",
            &[("aws_sagemaker_image", &["arn"])],
            false,
            &[],
        ),
        (
            "aws_sagemaker_space",
            "space_settings.code_editor_app_settings.default_resource_spec.sagemaker_image_version_arn"
            | "space_settings.jupyter_lab_app_settings.default_resource_spec.sagemaker_image_version_arn"
            | "space_settings.jupyter_server_app_settings.default_resource_spec.sagemaker_image_version_arn"
            | "space_settings.kernel_gateway_app_settings.default_resource_spec.sagemaker_image_version_arn",
        ) => Rule::new(
            "Studio image version",
            &[("aws_sagemaker_image_version", &["arn"])],
            false,
            &[],
        ),
        (
            "aws_sagemaker_space",
            "space_settings.custom_file_system.efs_file_system.file_system_id",
        ) => Rule::new(
            "Configured EFS file system",
            &[("aws_efs_file_system", &["id"])],
            false,
            &["space_settings.custom_file_system"],
        ),
        (
            "aws_sagemaker_space",
            "space_settings.jupyter_server_app_settings.lifecycle_config_arns"
            | "space_settings.kernel_gateway_app_settings.lifecycle_config_arns",
        ) => Rule::new(
            "Studio lifecycle configurations",
            &[("aws_sagemaker_studio_lifecycle_config", &["arn"])],
            true,
            &[],
        ),
        (
            "aws_sagemaker_user_profile",
            "user_settings.code_editor_app_settings.built_in_lifecycle_config_arn"
            | "user_settings.jupyter_lab_app_settings.built_in_lifecycle_config_arn",
        ) => Rule::new(
            "Built-in lifecycle configuration",
            &[("aws_sagemaker_studio_lifecycle_config", &["arn"])],
            false,
            &[],
        ),
        (
            "aws_sagemaker_user_profile",
            "user_settings.code_editor_app_settings.lifecycle_config_arns"
            | "user_settings.jupyter_lab_app_settings.lifecycle_config_arns"
            | "user_settings.jupyter_server_app_settings.lifecycle_config_arns"
            | "user_settings.kernel_gateway_app_settings.lifecycle_config_arns",
        ) => Rule::new(
            "Studio lifecycle configurations",
            &[("aws_sagemaker_studio_lifecycle_config", &["arn"])],
            true,
            &[],
        ),
        (
            "aws_sagemaker_user_profile",
            "user_settings.code_editor_app_settings.default_resource_spec.lifecycle_config_arn"
            | "user_settings.jupyter_lab_app_settings.default_resource_spec.lifecycle_config_arn"
            | "user_settings.jupyter_server_app_settings.default_resource_spec.lifecycle_config_arn"
            | "user_settings.kernel_gateway_app_settings.default_resource_spec.lifecycle_config_arn"
            | "user_settings.r_session_app_settings.default_resource_spec.lifecycle_config_arn"
            | "user_settings.tensor_board_app_settings.default_resource_spec.lifecycle_config_arn",
        ) => Rule::new(
            "Studio lifecycle configuration",
            &[("aws_sagemaker_studio_lifecycle_config", &["arn"])],
            false,
            &[],
        ),
        (
            "aws_sagemaker_user_profile",
            "user_settings.code_editor_app_settings.default_resource_spec.sagemaker_image_arn"
            | "user_settings.jupyter_lab_app_settings.default_resource_spec.sagemaker_image_arn"
            | "user_settings.jupyter_server_app_settings.default_resource_spec.sagemaker_image_arn"
            | "user_settings.kernel_gateway_app_settings.default_resource_spec.sagemaker_image_arn"
            | "user_settings.r_session_app_settings.default_resource_spec.sagemaker_image_arn"
            | "user_settings.tensor_board_app_settings.default_resource_spec.sagemaker_image_arn",
        ) => Rule::new(
            "Studio image",
            &[("aws_sagemaker_image", &["arn"])],
            false,
            &[],
        ),
        (
            "aws_sagemaker_user_profile",
            "user_settings.code_editor_app_settings.default_resource_spec.sagemaker_image_version_arn"
            | "user_settings.jupyter_lab_app_settings.default_resource_spec.sagemaker_image_version_arn"
            | "user_settings.jupyter_server_app_settings.default_resource_spec.sagemaker_image_version_arn"
            | "user_settings.kernel_gateway_app_settings.default_resource_spec.sagemaker_image_version_arn"
            | "user_settings.r_session_app_settings.default_resource_spec.sagemaker_image_version_arn"
            | "user_settings.tensor_board_app_settings.default_resource_spec.sagemaker_image_version_arn",
        ) => Rule::new(
            "Studio image version",
            &[("aws_sagemaker_image_version", &["arn"])],
            false,
            &[],
        ),
        (
            "aws_sagemaker_user_profile",
            "user_settings.custom_file_system_config.efs_file_system_config.file_system_id",
        ) => Rule::new(
            "Configured EFS file system",
            &[("aws_efs_file_system", &["id"])],
            false,
            &[
                "user_settings.custom_file_system_config",
                "user_settings.custom_file_system_config.efs_file_system_config",
            ],
        ),
        _ => return None,
    })
}
pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    rule(kind, path).map(|r| r.label)
}
pub(crate) fn indexed(kind: &str, path: &str) -> bool {
    resource(kind) && configured_links::indexed(kind, path, rule)
}
pub(super) fn bindings(raw: &Input, r: &AttributeReference) -> bool {
    configured_links::bindings(raw, r, rule)
}
pub(super) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    values: &[(&Value, &Value)],
) {
    configured_links::annotate(nodes, refs, values, rule, qualifies);
    for r in refs {
        if nodes[r.target].resource_type == "aws_sagemaker_domain"
            && r.attribute == "subnet_ids"
            && super::service_evidence::known(values, r.target, "app_network_access_type")
                != Some("VpcOnly")
        {
            r.endpoint_network_context.clear();
        }
    }
}
fn qualifies(e: &Evidence<'_>, r: &AttributeReference, s: usize, path: &str) -> bool {
    match (
        e.nodes[r.target].resource_type.as_str(),
        r.attribute.as_str(),
    ) {
        ("aws_sagemaker_app", "user_profile_name" | "space_name") => {
            let other = if r.attribute == "space_name" {
                "user_profile_name"
            } else {
                "space_name"
            };
            e.value(r.target, other)
                .is_some_and(|v| v.is_null() || v.as_str() == Some(""))
                && e.same_endpoint(
                    r.target,
                    "domain_id",
                    s,
                    "domain_id",
                    "aws_sagemaker_domain",
                    "id",
                )
        }
        ("aws_sagemaker_space", "ownership_settings.owner_user_profile_name") => {
            e.text(r.target, "space_sharing_settings.0.sharing_type") == Some("Private")
                && e.same_endpoint(
                    r.target,
                    "domain_id",
                    s,
                    "domain_id",
                    "aws_sagemaker_domain",
                    "id",
                )
        }
        ("aws_sagemaker_notebook_instance", "default_code_repository") => e
            .text(r.target, path)
            .is_some_and(|name| !name.contains(['/', ':'])),
        _ => true,
    }
}
pub(super) fn present(raw: &Input, graph: &mut Graph) {
    for (i, _) in raw.nodes.iter().enumerate().filter(|(_, n)| {
        n.resource_type == "aws_sagemaker_notebook_instance" && n.provider.is_aws()
    }) {
        let verified = |p| {
            super::lambda_core::unique(&raw.attributes, i, p)
                .filter(|r| r.service_reference_verified && bindings(raw, r))
        };
        let Some((subnets, groups)) = verified("subnet_id").zip(verified("security_groups")) else {
            continue;
        };
        let ([subnet], [vpc]) = (
            subnets.sources.as_slice(),
            subnets.endpoint_network_context.as_slice(),
        ) else {
            continue;
        };
        if groups.endpoint_network_context != [*vpc] {
            continue;
        }
        let mut evidence = vec![
            crate::semantic::relationships::reference(graph, *subnet, i),
            crate::semantic::relationships::reference(graph, *vpc, *subnet),
        ];
        for &sg in &groups.sources {
            evidence.push(crate::semantic::relationships::reference(graph, *vpc, sg));
            evidence.push(crate::semantic::relationships::reference(graph, sg, i));
        }
        crate::semantic::relationships::record(
            graph,
            *subnet,
            i,
            crate::model::EdgeKind::Containment,
            evidence,
        );
        graph.edges.push(crate::model::Edge {
            kind: crate::model::EdgeKind::Containment,
            ..crate::model::Edge::from((*subnet, i))
        });
    }
    super::lambda_core::present_network(
        raw,
        graph,
        "aws_sagemaker_domain",
        ("subnet_ids", "default_user_settings.security_groups"),
        |input, r| {
            bindings(input, r)
                && super::lambda_core::unique(&input.attributes, r.target, "vpc_id").is_some_and(
                    |v| v.service_reference_verified && v.sources == r.endpoint_network_context,
                )
        },
    );
}
#[cfg(test)]
#[path = "../../../tests/unit/semantic/sagemaker_studio.rs"]
mod tests;
