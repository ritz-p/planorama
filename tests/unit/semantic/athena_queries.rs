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

#[test]
fn lambda_catalog_accepts_one_function_in_both_roles() {
    let mut input: Value =
        serde_json::from_str(include_str!("../../fixtures/athena-queries-plan.json")).unwrap();
    let arn = input["resource_changes"][5]["change"]["after"]["arn"].clone();
    input["resource_changes"][6]["change"]["after"]["parameters"] =
        json!({"metadata-function": arn, "record-function": arn});
    assert!(proven(
        &input,
        "aws_athena_data_catalog.sample6",
        "parameters"
    ));
}

#[test]
fn scoped_named_query_retains_verified_workgroup_owner() {
    let mut input: Value =
        serde_json::from_str(include_str!("../../fixtures/athena-queries-plan.json")).unwrap();
    input["configuration"]["provider_config"]["aws"]["expressions"]["assume_role"] =
        json!([{"role_arn":{"constant_value":"arn:aws:iam::123456789012:role/fixture"}}]);
    let parsed = plan::evidence::tests::parse_fixture(&input.to_string()).unwrap();
    let graph = semantic::transform(&parsed);
    let query = graph
        .nodes
        .iter()
        .find(|node| node.address == "aws_athena_named_query.sample4")
        .unwrap();
    let workgroup = graph
        .nodes
        .iter()
        .find(|node| node.address == "aws_athena_workgroup.sample2")
        .unwrap();
    assert!(query.entity.scope.as_ref().unwrap().account.is_some());
    assert_eq!(query.entity.scope, workgroup.entity.scope);
    assert_eq!(
        query.member_owner.as_ref().unwrap().target,
        workgroup.entity.provenance[0]
    );
}
