use crate::{plan, semantic};
use serde_json::{Value, json};
fn proven(v: &Value, address: &str, path: &str) -> bool {
    let g = semantic::transform(&plan::parse(&v.to_string()).unwrap());
    g.nodes
        .iter()
        .find(|n| n.address == address)
        .is_some_and(|n| {
            n.references.iter().any(|r| {
                r.attribute == path
                    && r.complete
                    && !r.dependency
                    && !r.targets.is_empty()
                    && super::label(&n.resource_type, path) == Some(r.label)
            })
        })
}

#[test]
fn sagemaker_catalog_requires_scoped_provenance() {
    let original: Value =
        serde_json::from_str(include_str!("../../fixtures/sagemaker-catalog-plan.json")).unwrap();
    for (addr, path) in [
        ("aws_sagemaker_mlflow_tracking_server.server", "role_arn"),
        (
            "aws_sagemaker_model_card_export_job.export",
            "model_card_name",
        ),
        ("aws_sagemaker_device_fleet.devices", "role_arn"),
        ("aws_sagemaker_device.device", "device_fleet_name"),
    ] {
        assert!(proven(&original, addr, path), "{addr}/{path}");
    }
    {
        let mut v = original.clone();
        v["resource_changes"][4]["deposed"] = json!("old");
        assert!(
            !proven(&v, "aws_sagemaker_device.device", "device_fleet_name"),
            "unexpected proof for aws_sagemaker_device.device device_fleet_name"
        );
    }
    {
        let mut v = original.clone();
        v["resource_changes"][2]["change"]["after_sensitive"] = json!({"model_card_name":true});
        assert!(
            !proven(
                &v,
                "aws_sagemaker_model_card_export_job.export",
                "model_card_name"
            ),
            "unexpected proof for aws_sagemaker_model_card_export_job.export model_card_name"
        );
    }
    {
        let mut v = original.clone();
        v["resource_changes"][0]["change"]["actions"] = json!(["delete"]);
        assert!(
            !proven(
                &v,
                "aws_sagemaker_mlflow_tracking_server.server",
                "role_arn"
            ),
            "unexpected proof for aws_sagemaker_mlflow_tracking_server.server role_arn"
        );
    }
}

#[test]
fn feature_store_requires_matching_glue_catalog_for_database_and_table() {
    let mut input: Value =
        serde_json::from_str(include_str!("../../fixtures/sagemaker-catalog-plan.json")).unwrap();
    let mut add = |kind: &str, after, expressions| {
        let address = format!("{kind}.offline");
        input["resource_changes"].as_array_mut().unwrap().push(json!({
            "address":address,"mode":"managed","type":kind,"name":"offline",
            "provider_name":"registry.terraform.io/hashicorp/aws","change":{"actions":["create"],"after":after}
        }));
        input["configuration"]["root_module"]["resources"]
            .as_array_mut()
            .unwrap()
            .push(json!({
                "address":address,"mode":"managed","type":kind,"name":"offline",
                "provider_config_key":"aws","expressions":expressions
            }));
    };
    add(
        "aws_glue_catalog_database",
        json!({"name":"features","catalog_id":"123456789012"}),
        json!({}),
    );
    add(
        "aws_glue_catalog_table",
        json!({"name":"measurements","database_name":"features","catalog_id":"123456789012"}),
        json!({"database_name":{"references":["aws_glue_catalog_database.offline.name"]}}),
    );
    add(
        "aws_sagemaker_feature_group",
        json!({"offline_store_config":[{"data_catalog_config":[{"catalog":"123456789012","database":"features","table_name":"measurements"}]}]}),
        json!({"offline_store_config":[{"data_catalog_config":[
            {"database":{"references":["aws_glue_catalog_database.offline.name"]},
             "table_name":{"references":["aws_glue_catalog_table.offline.name"]}}
        ]}]}),
    );
    let paths = [
        "offline_store_config.data_catalog_config.database",
        "offline_store_config.data_catalog_config.table_name",
    ];
    for path in paths {
        assert!(
            proven(&input, "aws_sagemaker_feature_group.offline", path),
            "{path}"
        );
    }
    let changes = input["resource_changes"].as_array_mut().unwrap();
    let db = changes
        .iter_mut()
        .find(|r| r["address"] == "aws_glue_catalog_database.offline")
        .unwrap();
    db["change"]["after"]["catalog_id"] = json!("987654321098");
    for path in paths {
        assert!(
            !proven(&input, "aws_sagemaker_feature_group.offline", path),
            "{path}"
        );
    }
}
