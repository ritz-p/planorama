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
fn glue_encryption_requires_scoped_provenance() {
    let original: Value =
        serde_json::from_str(include_str!("../../fixtures/glue-encryption-plan.json")).unwrap();
    for (addr, path) in [
        (
            "aws_glue_security_configuration.sample1",
            "encryption_configuration.cloudwatch_encryption.kms_key_arn",
        ),
        (
            "aws_glue_data_catalog_encryption_settings.sample3",
            "data_catalog_encryption_settings.encryption_at_rest.sse_aws_kms_key_id",
        ),
        (
            "aws_glue_data_catalog_encryption_settings.sample3",
            "data_catalog_encryption_settings.encryption_at_rest.catalog_encryption_service_role",
        ),
    ] {
        assert!(proven(&original, addr, path), "{addr}/{path}");
    }
    {
        let mut v = original.clone();
        v["resource_changes"][1]["change"]["after"]["encryption_configuration"][0]["cloudwatch_encryption"]
            [0]["cloudwatch_encryption_mode"] = json!("DISABLED");
        assert!(
            !proven(
                &v,
                "aws_glue_security_configuration.sample1",
                "encryption_configuration.cloudwatch_encryption.kms_key_arn"
            ),
            "unexpected proof for aws_glue_security_configuration.sample1 encryption_configuration.cloudwatch_encryption.kms_key_arn"
        );
    }
    {
        let mut v = original.clone();
        v["resource_changes"][3]["change"]["after"]["data_catalog_encryption_settings"][0]["encryption_at_rest"]
            [0]["catalog_encryption_mode"] = json!("SSE-KMS");
        assert!(
            !proven(
                &v,
                "aws_glue_data_catalog_encryption_settings.sample3",
                "data_catalog_encryption_settings.encryption_at_rest.catalog_encryption_service_role"
            ),
            "unexpected proof for aws_glue_data_catalog_encryption_settings.sample3 data_catalog_encryption_settings.encryption_at_rest.catalog_encryption_service_role"
        );
    }
}
