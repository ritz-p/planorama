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
fn timestream_influx_requires_scoped_provenance() {
    let original: Value =
        serde_json::from_str(include_str!("../../fixtures/timestream-influx-plan.json")).unwrap();
    for (addr, path) in [
        ("aws_timestreaminfluxdb_db_cluster.row4", "vpc_subnet_ids"),
        (
            "aws_timestreaminfluxdb_db_cluster.row4",
            "vpc_security_group_ids",
        ),
        (
            "aws_timestreaminfluxdb_db_cluster.row4",
            "log_delivery_configuration.s3_configuration.bucket_name",
        ),
        ("aws_timestreaminfluxdb_db_instance.row5", "vpc_subnet_ids"),
        (
            "aws_timestreaminfluxdb_db_instance.row5",
            "vpc_security_group_ids",
        ),
        (
            "aws_timestreaminfluxdb_db_instance.row5",
            "log_delivery_configuration.s3_configuration.bucket_name",
        ),
    ] {
        assert!(proven(&original, addr, path), "{addr}/{path}");
    }
    {
        let mut v = original.clone();
        v["resource_changes"][4]["change"]["after"]["log_delivery_configuration"][0]["s3_configuration"]
            [0]["enabled"] = json!(false);
        assert!(
            !proven(
                &v,
                "aws_timestreaminfluxdb_db_cluster.row4",
                "log_delivery_configuration.s3_configuration.bucket_name"
            ),
            "unexpected proof for aws_timestreaminfluxdb_db_cluster.row4 log_delivery_configuration.s3_configuration.bucket_name"
        );
    }
    {
        let mut v = original.clone();
        v["resource_changes"][4]["change"]["after"]["vpc_subnet_ids"] =
            json!(["subnet-a", "subnet-other"]);
        assert!(
            !proven(
                &v,
                "aws_timestreaminfluxdb_db_cluster.row4",
                "vpc_subnet_ids"
            ),
            "unexpected proof for aws_timestreaminfluxdb_db_cluster.row4 vpc_subnet_ids"
        );
    }
    {
        let mut v = original.clone();
        v["resource_changes"][5]["change"]["after"]["log_delivery_configuration"][0]["s3_configuration"]
            [0]["enabled"] = json!(false);
        assert!(
            !proven(
                &v,
                "aws_timestreaminfluxdb_db_instance.row5",
                "log_delivery_configuration.s3_configuration.bucket_name"
            ),
            "unexpected proof for aws_timestreaminfluxdb_db_instance.row5 log_delivery_configuration.s3_configuration.bucket_name"
        );
    }
    {
        let mut v = original.clone();
        v["resource_changes"][5]["change"]["after"]["vpc_subnet_ids"] =
            json!(["subnet-a", "subnet-other"]);
        assert!(
            !proven(
                &v,
                "aws_timestreaminfluxdb_db_instance.row5",
                "vpc_subnet_ids"
            ),
            "unexpected proof for aws_timestreaminfluxdb_db_instance.row5 vpc_subnet_ids"
        );
    }
}
