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
fn firehose_msk_source_requires_scoped_provenance() {
    let original: Value =
        serde_json::from_str(include_str!("../../fixtures/firehose-msk-source-plan.json")).unwrap();
    for (addr, path) in [
        (
            "aws_kinesis_firehose_delivery_stream.row2",
            "msk_source_configuration.msk_cluster_arn",
        ),
        (
            "aws_kinesis_firehose_delivery_stream.row2",
            "msk_source_configuration.authentication_configuration.role_arn",
        ),
        (
            "aws_kinesis_firehose_delivery_stream.row2",
            "extended_s3_configuration.s3_backup_configuration.role_arn",
        ),
    ] {
        assert!(proven(&original, addr, path), "{addr}/{path}");
    }
    {
        let mut v = original.clone();
        v["resource_changes"][2]["change"]["after"]["kinesis_source_configuration"] = json!([{}]);
        assert!(
            !proven(
                &v,
                "aws_kinesis_firehose_delivery_stream.row2",
                "msk_source_configuration.msk_cluster_arn"
            ),
            "unexpected proof for aws_kinesis_firehose_delivery_stream.row2 msk_source_configuration.msk_cluster_arn"
        );
    }
    {
        let mut v = original.clone();
        v["resource_changes"][2]["change"]["after"]["extended_s3_configuration"][0]["s3_backup_mode"] =
            json!("Disabled");
        assert!(
            !proven(
                &v,
                "aws_kinesis_firehose_delivery_stream.row2",
                "extended_s3_configuration.s3_backup_configuration.role_arn"
            ),
            "unexpected proof for aws_kinesis_firehose_delivery_stream.row2 extended_s3_configuration.s3_backup_configuration.role_arn"
        );
    }
}
