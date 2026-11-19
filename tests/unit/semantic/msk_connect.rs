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
fn msk_connect_requires_scoped_provenance() {
    let original: Value =
        serde_json::from_str(include_str!("../../fixtures/msk-connect-plan.json")).unwrap();
    for (addr, path) in [
        (
            "aws_mskconnect_custom_plugin.row1",
            "location.s3.bucket_arn",
        ),
        ("aws_mskconnect_connector.row3", "plugin.custom_plugin.arn"),
        ("aws_mskconnect_connector.row3", "worker_configuration.arn"),
    ] {
        assert!(proven(&original, addr, path), "{addr}/{path}");
    }
    {
        let mut v = original.clone();
        v["resource_changes"][3]["change"]["after"]["plugin"][0]["custom_plugin"][0]["revision"] =
            json!(4);
        assert!(
            !proven(
                &v,
                "aws_mskconnect_connector.row3",
                "plugin.custom_plugin.arn"
            ),
            "unexpected proof for aws_mskconnect_connector.row3 plugin.custom_plugin.arn"
        );
    }
    {
        let mut v = original.clone();
        v["resource_changes"][2]["change"]["after_unknown"] = json!({"latest_revision":true});
        assert!(
            !proven(
                &v,
                "aws_mskconnect_connector.row3",
                "worker_configuration.arn"
            ),
            "unexpected proof for aws_mskconnect_connector.row3 worker_configuration.arn"
        );
    }
}
