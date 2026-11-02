use serde_json::{Value, json};

fn fixture(kind: &str, field: &str, target: &str, output: &str) -> Value {
    let address = format!("{kind}.main");
    let source = format!("{target}.main[0]");
    json!({"format_version":"1.2", "resource_changes":[
        {"address":source,"type":target,"change":{"actions":["create"],"after":{output:"known"}}},
        {"address":address,"type":kind,"change":{"actions":["create"],"after":{field:"known"}}}
    ],"configuration":{"provider_config":{"aws":{"full_name":"registry.terraform.io/hashicorp/aws","expressions":{"region":{"constant_value":"us-east-1"}}}},"root_module":{"resources":[
        {"address":format!("{target}.main"),"provider_config_key":"aws","expressions":{}},
        {"address":address,"provider_config_key":"aws","expressions":{field:{"references":[format!("{source}.{output}")]}}}
    ]}}})
}

fn proven(v: &Value, field: &str) -> bool {
    let graph = crate::semantic::transform(&crate::plan::parse(&v.to_string()).unwrap());
    graph.nodes.iter().any(|n| {
        n.references.iter().any(|r| {
            Some(r.label) == super::label(&n.resource_type, field)
                && r.attribute == field
                && r.complete
                && !r.dependency
                && !r.targets.is_empty()
        })
    })
}

#[test]
fn access_points_distinguish_bucket_names_and_outposts_arns() {
    for (kind, field, target, output) in [
        (super::ACCESS, "bucket", "aws_s3_bucket", "bucket"),
        (super::ACCESS, "bucket", "aws_s3control_bucket", "arn"),
        (
            "aws_s3control_access_point_policy",
            "access_point_arn",
            super::ACCESS,
            "arn",
        ),
        (
            "aws_s3control_bucket_policy",
            "bucket",
            "aws_s3control_bucket",
            "arn",
        ),
        (
            "aws_s3control_bucket_lifecycle_configuration",
            "bucket",
            "aws_s3control_bucket",
            "id",
        ),
        (
            "aws_s3control_object_lambda_access_point_policy",
            "name",
            super::OBJECT,
            "name",
        ),
    ] {
        let mut v = fixture(kind, field, target, output);
        v["resource_changes"][1]["change"]["after"]["vpc_configuration"] =
            json!([{"vpc_id":"vpc-main"}]);
        if field == "name" {
            v["resource_changes"][0]["change"]["after"]["account_id"] = json!("111111111111");
            v["resource_changes"][1]["change"]["after"]["account_id"] = json!("111111111111");
            // JSON-only provider accounts cannot verify an account-qualified name.
            assert!(!proven(&v, field));
            continue;
        }
        assert!(proven(&v, field), "{kind} -> {target}");
        for case in 0..8 {
            let mut bad = v.clone();
            match case {
                0 => bad["resource_changes"][1]["change"]["after"][field] = json!("other"),
                1 => bad["resource_changes"][0]["change"]["after_sensitive"] = json!({output:true}),
                2 => bad["resource_changes"][1]["change"]["after_unknown"] = json!({field:true}),
                3 => bad["resource_changes"][0]["mode"] = json!("data"),
                4 => bad["resource_changes"][0]["deposed"] = json!("old"),
                5 => bad["resource_changes"][0]["type"] = json!("aws_sqs_queue"),
                6 => bad["resource_changes"][0]["change"]["actions"] = json!(["delete"]),
                _ => bad["configuration"]["root_module"]["resources"][1]["expressions"] = json!({}),
            }
            assert!(!proven(&bad, field), "{kind} case {case}");
        }
    }
}

#[test]
fn object_lambda_requires_complete_correlated_transformations() {
    let mut v = fixture(super::OBJECT, super::FUNCTION, "aws_lambda_function", "arn");
    v["resource_changes"][1]["change"]["after"] = json!({"configuration":[{"transformation_configuration":[{"content_transformation":[{"aws_lambda":[{"function_arn":"known"}]}]}]}]});
    v["configuration"]["root_module"]["resources"][1]["expressions"] = json!({"configuration":[{"transformation_configuration":[{"content_transformation":[{"aws_lambda":[{"function_arn":{"references":["aws_lambda_function.main[0].arn"]}}]}]}]}]});
    assert!(proven(&v, super::FUNCTION));
    let mut second = v["resource_changes"][0].clone();
    second["address"] = json!("aws_lambda_function.main[1]");
    second["change"]["after"]["arn"] = json!("second");
    v["resource_changes"].as_array_mut().unwrap().push(second);
    v["resource_changes"][1]["change"]["after"]["configuration"][0]["transformation_configuration"]
        .as_array_mut()
        .unwrap()
        .push(json!({"content_transformation":[{"aws_lambda":[{"function_arn":"second"}]}]}));
    assert!(!proven(&v, super::FUNCTION));
    v["configuration"]["root_module"]["resources"][1]["expressions"]["configuration"][0]["transformation_configuration"].as_array_mut().unwrap().push(json!({"content_transformation":[{"aws_lambda":[{"function_arn":{"references":["aws_lambda_function.main[1].arn"]}}]}]}));
    assert!(proven(&v, super::FUNCTION));
    v["resource_changes"][1]["change"]["after_sensitive"] =
        json!({"configuration":[{"transformation_configuration":[false,true]}]});
    assert!(!proven(&v, super::FUNCTION));
}

#[test]
fn outposts_endpoint_placement_requires_matching_subnet_security_group_and_outpost() {
    let mut v = fixture(super::ENDPOINT, "subnet_id", "aws_subnet", "id");
    v["resource_changes"][0]["change"]["after"]["vpc_id"] = json!("vpc-main");
    v["resource_changes"][0]["change"]["after"]["outpost_arn"] =
        json!("arn:aws:outposts:us-east-1:111111111111:outpost/op-main");
    v["resource_changes"][1]["change"]["after"]["outpost_id"] = json!("op-main");
    v["resource_changes"][1]["change"]["after"]["security_group_id"] = json!("sg-main");
    v["resource_changes"].as_array_mut().unwrap().push(json!({"address":"aws_security_group.main","type":"aws_security_group","change":{"actions":["create"],"after":{"id":"sg-main","vpc_id":"vpc-main"}}}));
    v["configuration"]["root_module"]["resources"][1]["expressions"]["security_group_id"] =
        json!({"references":["aws_security_group.main.id"]});
    v["configuration"]["root_module"]["resources"].as_array_mut().unwrap().push(json!({"address":"aws_security_group.main","provider_config_key":"aws","expressions":{}}));
    assert!(proven(&v, "subnet_id"));
    assert!(proven(&v, "security_group_id"));
    for case in 0..3 {
        let mut bad = v.clone();
        match case {
            0 => bad["resource_changes"][2]["change"]["after"]["vpc_id"] = json!("other"),
            1 => bad["resource_changes"][1]["change"]["after"]["outpost_id"] = json!("other"),
            _ => bad["resource_changes"][2]["change"]["after_sensitive"] = json!({"vpc_id":true}),
        }
        assert!(!proven(&bad, "subnet_id"));
        assert!(!proven(&bad, "security_group_id"));
    }
}
