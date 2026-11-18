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
fn athena_queries_requires_scoped_provenance() {
    let original: Value =
        serde_json::from_str(include_str!("../../fixtures/athena-queries-plan.json")).unwrap();
    for (addr, path) in [
        (
            "aws_athena_workgroup.sample2",
            "configuration.result_configuration.output_location",
        ),
        ("aws_athena_database.sample3", "workgroup"),
        (
            "aws_athena_database.sample3",
            "encryption_configuration.kms_key",
        ),
        ("aws_athena_named_query.sample4", "workgroup"),
        ("aws_athena_named_query.sample4", "database"),
        ("aws_athena_data_catalog.sample6", "parameters"),
    ] {
        assert!(proven(&original, addr, path), "{addr}/{path}");
    }
    {
        let mut v = original.clone();
        v["resource_changes"][3]["change"]["after"]["encryption_configuration"][0]["encryption_option"] =
            json!("SSE_S3");
        assert!(
            !proven(
                &v,
                "aws_athena_database.sample3",
                "encryption_configuration.kms_key"
            ),
            "unexpected proof for aws_athena_database.sample3 encryption_configuration.kms_key"
        );
    }
    {
        let mut v = original.clone();
        v["resource_changes"][6]["change"]["after"]["type"] = json!("GLUE");
        assert!(
            !proven(&v, "aws_athena_data_catalog.sample6", "parameters"),
            "unexpected proof for aws_athena_data_catalog.sample6 parameters"
        );
    }
    {
        let mut v = original.clone();
        v["resource_changes"][2]["change"]["after"]["configuration"][0]["managed_query_results_configuration"] =
            json!([{"enabled":true}]);
        assert!(
            !proven(
                &v,
                "aws_athena_workgroup.sample2",
                "configuration.result_configuration.output_location"
            ),
            "unexpected proof for aws_athena_workgroup.sample2 configuration.result_configuration.output_location"
        );
    }
}
