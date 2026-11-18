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
fn msk_delivery_requires_scoped_provenance() {
    let original: Value =
        serde_json::from_str(include_str!("../../fixtures/msk-delivery-plan.json")).unwrap();
    for (addr, path) in [
        ("aws_msk_topic.row2", "cluster_arn"),
        ("aws_msk_channel.row4", "topic_configuration.topic_arn"),
        ("aws_msk_channel.row4", "s3_destination.storage.bucket_arn"),
        ("aws_msk_channel.row4", "cluster_arn"),
    ] {
        assert!(proven(&original, addr, path), "{addr}/{path}");
    }
    {
        let mut v = original.clone();
        v["resource_changes"][4]["change"]["after"]["cluster_arn"] =
            json!("arn:aws:kafka:us-east-1:123456789012:cluster/two/02");
        assert!(
            !proven(&v, "aws_msk_channel.row4", "topic_configuration.topic_arn"),
            "unexpected proof for aws_msk_channel.row4 topic_configuration.topic_arn"
        );
    }
    {
        let mut v = original.clone();
        v["resource_changes"][4]["change"]["after"]["iceberg_destination"] = json!([{}]);
        assert!(
            !proven(
                &v,
                "aws_msk_channel.row4",
                "s3_destination.storage.bucket_arn"
            ),
            "unexpected proof for aws_msk_channel.row4 s3_destination.storage.bucket_arn"
        );
    }
}
