use super::configured_links::{self, Evidence, Rule};
use crate::model::{AttributeReference, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;

pub(crate) fn resource(kind: &str) -> bool {
    matches!(
        kind,
        "aws_sagemaker_device"
            | "aws_sagemaker_device_fleet"
            | "aws_sagemaker_feature_group"
            | "aws_sagemaker_hub_content_reference"
            | "aws_sagemaker_inference_component"
            | "aws_sagemaker_mlflow_app"
            | "aws_sagemaker_mlflow_tracking_server"
            | "aws_sagemaker_model_card"
            | "aws_sagemaker_model_card_export_job"
            | "aws_sagemaker_model_package_group_policy"
            | "aws_sagemaker_project"
    )
}
fn rule(kind: &str, path: &str) -> Option<Rule> {
    Some(match (kind, path) {
        ("aws_sagemaker_device", "device_fleet_name") => Rule::new(
            "Device fleet",
            &[("aws_sagemaker_device_fleet", &["device_fleet_name"])],
            false,
            &[],
        ),
        ("aws_sagemaker_device", "device.iot_thing_name") => {
            Rule::new("IoT thing", &[("aws_iot_thing", &["name"])], false, &[])
        }
        ("aws_sagemaker_device_fleet", "role_arn") => {
            Rule::new("Fleet role", &[("aws_iam_role", &["arn"])], false, &[])
        }
        ("aws_sagemaker_device_fleet", "output_config.kms_key_id") => Rule::new(
            "Fleet output key",
            &[("aws_kms_key", &["arn", "key_id"])],
            false,
            &[],
        ),
        ("aws_sagemaker_feature_group", "role_arn") => Rule::new(
            "Feature group role",
            &[("aws_iam_role", &["arn"])],
            false,
            &[],
        ),
        ("aws_sagemaker_feature_group", "offline_store_config.data_catalog_config.database") => {
            Rule::new(
                "Offline store database",
                &[("aws_glue_catalog_database", &["name"])],
                false,
                &[],
            )
        }
        ("aws_sagemaker_feature_group", "offline_store_config.data_catalog_config.table_name") => {
            Rule::new(
                "Offline store table",
                &[("aws_glue_catalog_table", &["name"])],
                false,
                &[],
            )
        }
        (
            "aws_sagemaker_feature_group",
            "offline_store_config.s3_storage_config.kms_key_id"
            | "online_store_config.security_config.kms_key_id",
        ) => Rule::new(
            "Feature store key",
            &[("aws_kms_key", &["arn", "key_id"])],
            false,
            &[],
        ),
        ("aws_sagemaker_hub_content_reference", "hub_name") => Rule::new(
            "Private hub",
            &[("aws_sagemaker_hub", &["hub_name"])],
            false,
            &[],
        ),
        ("aws_sagemaker_inference_component", "endpoint_name") => Rule::new(
            "Inference endpoint",
            &[("aws_sagemaker_endpoint", &["name"])],
            false,
            &[],
        ),
        ("aws_sagemaker_inference_component", "specification.base_inference_component_name") => {
            Rule::new(
                "Base inference component",
                &[("aws_sagemaker_inference_component", &["name"])],
                false,
                &["specification"],
            )
        }
        ("aws_sagemaker_inference_component", "specification.model_name") => Rule::new(
            "Inference model",
            &[("aws_sagemaker_model", &["name"])],
            false,
            &["specification"],
        ),
        ("aws_sagemaker_inference_component", "specifications.base_inference_component_name") => {
            Rule::new(
                "Base inference component",
                &[("aws_sagemaker_inference_component", &["name"])],
                false,
                &["specifications"],
            )
        }
        ("aws_sagemaker_inference_component", "specifications.model_name") => Rule::new(
            "Inference model",
            &[("aws_sagemaker_model", &["name"])],
            false,
            &["specifications"],
        ),
        ("aws_sagemaker_mlflow_app", "role_arn") => Rule::new(
            "MLflow execution role",
            &[("aws_iam_role", &["arn"])],
            false,
            &[],
        ),
        ("aws_sagemaker_mlflow_tracking_server", "role_arn") => Rule::new(
            "MLflow execution role",
            &[("aws_iam_role", &["arn"])],
            false,
            &[],
        ),
        ("aws_sagemaker_model_card", "security_config.kms_key_id") => Rule::new(
            "Model card key",
            &[("aws_kms_key", &["arn"])],
            false,
            &["security_config"],
        ),
        ("aws_sagemaker_model_card_export_job", "model_card_name") => Rule::new(
            "Exported model card",
            &[("aws_sagemaker_model_card", &["model_card_name"])],
            false,
            &[],
        ),
        ("aws_sagemaker_model_package_group_policy", "model_package_group_name") => Rule::new(
            "Model package group",
            &[(
                "aws_sagemaker_model_package_group",
                &["model_package_group_name"],
            )],
            false,
            &[],
        ),
        ("aws_sagemaker_project", "service_catalog_provisioning_details.product_id") => Rule::new(
            "Service Catalog product",
            &[("aws_servicecatalog_product", &["id"])],
            false,
            &[],
        ),
        (
            "aws_sagemaker_project",
            "service_catalog_provisioning_details.provisioning_artifact_id",
        ) => Rule::new(
            "Product provisioning artifact",
            &[(
                "aws_servicecatalog_provisioning_artifact",
                &["provisioning_artifact_id", "id"],
            )],
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
    configured_links::annotate(nodes, refs, values, rule, qualifies)
}
fn qualifies(e: &Evidence<'_>, r: &AttributeReference, s: usize, path: &str) -> bool {
    match (
        e.nodes[r.target].resource_type.as_str(),
        r.attribute.as_str(),
    ) {
        ("aws_sagemaker_hub_content_reference", "hub_name") => e
            .text(r.target, "sagemaker_public_hub_content_arn")
            .is_some_and(|a| {
                a.starts_with("arn:")
                    && a.contains(":hub-content/SageMakerPublicHub/")
                    && !a.contains("/version/")
                    && a.split('/').count() == 4
            }),
        ("aws_sagemaker_feature_group", "offline_store_config.data_catalog_config.database") => {
            catalog_matches(e, r.target, s)
        }
        ("aws_sagemaker_feature_group", "offline_store_config.data_catalog_config.table_name") => {
            catalog_matches(e, r.target, s)
                && super::lambda_core::unique(e.refs, s, "database_name").is_some_and(|reference| {
                    let [database] = reference.sources.as_slice() else {
                        return false;
                    };
                    catalog_matches(e, r.target, *database)
                })
                && e.same_endpoint(
                    r.target,
                    "offline_store_config.0.data_catalog_config.0.database",
                    s,
                    "database_name",
                    "aws_glue_catalog_database",
                    "name",
                )
        }
        ("aws_sagemaker_project", p) if p.ends_with(".provisioning_artifact_id") => e
            .same_endpoint(
                r.target,
                "service_catalog_provisioning_details.0.product_id",
                s,
                "product_id",
                "aws_servicecatalog_product",
                "id",
            ),
        ("aws_sagemaker_inference_component", p)
            if p.ends_with(".base_inference_component_name") =>
        {
            e.same_endpoint(
                r.target,
                "endpoint_name",
                s,
                "endpoint_name",
                "aws_sagemaker_endpoint",
                "name",
            ) && e
                .text(r.target, "variant_name")
                .is_some_and(|v| e.text(s, "variant_name") == Some(v))
        }
        ("aws_sagemaker_inference_component", p) if p.ends_with(".model_name") => {
            ((p.starts_with("specification.") && empty(e, r.target, "specifications"))
                || (p.starts_with("specifications.") && empty(e, r.target, "specification")))
                && path.rsplit_once('.').is_some_and(|(block, _)| {
                    e.value(r.target, &format!("{block}.base_inference_component_name"))
                        .is_some_and(Value::is_null)
                })
        }
        _ => true,
    }
}
fn catalog_matches(e: &Evidence<'_>, feature_group: usize, endpoint: usize) -> bool {
    e.text(
        feature_group,
        "offline_store_config.0.data_catalog_config.0.catalog",
    )
    .is_some_and(|catalog| e.text(endpoint, "catalog_id") == Some(catalog))
}

fn empty(e: &Evidence<'_>, i: usize, p: &str) -> bool {
    e.value(i, p)
        .is_some_and(|v| v.is_null() || v.as_array().is_some_and(Vec::is_empty))
}
#[cfg(test)]
#[path = "../../../tests/unit/semantic/sagemaker_catalog.rs"]
mod tests;
