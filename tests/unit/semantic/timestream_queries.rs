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
fn timestream_queries_requires_scoped_provenance() {
    let original: Value =
        serde_json::from_str(include_str!("../../fixtures/timestream-queries-plan.json")).unwrap();
    for (addr, path) in [
        ("aws_timestreamwrite_database.row1", "kms_key_id"),
        ("aws_timestreamwrite_table.row2", "database_name"),
        (
            "aws_timestreamquery_scheduled_query.row3",
            "target_configuration.timestream_configuration.database_name",
        ),
        (
            "aws_timestreamquery_scheduled_query.row3",
            "target_configuration.timestream_configuration.table_name",
        ),
    ] {
        assert!(proven(&original, addr, path), "{addr}/{path}");
    }
    {
        let mut v = original.clone();
        v["resource_changes"][2]["change"]["after"]["database_name"] = json!("other");
        assert!(
            !proven(
                &v,
                "aws_timestreamquery_scheduled_query.row3",
                "target_configuration.timestream_configuration.table_name"
            ),
            "unexpected proof for aws_timestreamquery_scheduled_query.row3 target_configuration.timestream_configuration.table_name"
        );
    }
    {
        let mut v = original.clone();
        v["resource_changes"][0]["deposed"] = json!("old");
        assert!(
            !proven(&v, "aws_timestreamwrite_database.row1", "kms_key_id"),
            "unexpected proof for aws_timestreamwrite_database.row1 kms_key_id"
        );
    }
}
