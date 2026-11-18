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
