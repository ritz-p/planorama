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
fn graphql_authentication_and_roles_require_discriminators_and_known_values() {
    for (path, target, output, mode, expected) in [
        (
            "lambda_authorizer_config.authorizer_uri",
            "aws_lambda_function",
            "arn",
            "authentication_type",
            "AWS_LAMBDA",
        ),
        (
            "user_pool_config.user_pool_id",
            "aws_cognito_user_pool",
            "id",
            "authentication_type",
            "AMAZON_COGNITO_USER_POOLS",
        ),
        (
            "merged_api_execution_role_arn",
            "aws_iam_role",
            "arn",
            "api_type",
            "MERGED",
        ),
        (
            "log_config.cloudwatch_logs_role_arn",
            "aws_iam_role",
            "arn",
            "authentication_type",
            "API_KEY",
        ),
    ] {
        let mut v = fixture(super::API, path, target, output);
        if let Some((block, field)) = path.split_once('.') {
            v["resource_changes"][1]["change"]["after"] = json!({block:[{field:"known"}]});
            v["configuration"]["root_module"]["resources"][1]["expressions"] =
                json!({block:[{field:{"references":[format!("{target}.main[0].{output}")]}}]});
        }
        v["resource_changes"][1]["change"]["after"][mode] = json!(expected);
        assert!(proven(&v, path), "{path}");
        for case in 0..6 {
            let mut bad = v.clone();
            match case {
                0 => bad["resource_changes"][0]["change"]["after"][output] = json!("other"),
                1 => bad["resource_changes"][0]["change"]["after_unknown"] = json!({output:true}),
                2 => bad["resource_changes"][0]["change"]["after_sensitive"] = json!({output:true}),
                3 => bad["resource_changes"][0]["mode"] = json!("data"),
                4 => bad["resource_changes"][0]["deposed"] = json!("old"),
                _ => bad["resource_changes"][0]["type"] = json!("aws_sqs_queue"),
            }
            assert!(!proven(&bad, path), "{path} case {case}");
        }
        if !path.starts_with("log_config") {
            v["resource_changes"][1]["change"]["after"][mode] = json!("wrong");
            assert!(!proven(&v, path));
        }
    }
}

#[test]
fn source_associations_require_a_distinct_merged_api_and_exclusive_identifiers() {
    let mut v = fixture(super::ASSOCIATION, "merged_api_id", super::API, "id");
    v["resource_changes"][0]["change"]["after"]["api_type"] = json!("MERGED");
    v["resource_changes"][1]["change"]["after"]["source_api_id"] = json!("source");
    let mut source = v["resource_changes"][0].clone();
    source["address"] = json!("aws_appsync_graphql_api.main[1]");
    source["change"]["after"] = json!({"id":"source","api_type":"GRAPHQL"});
    v["resource_changes"].as_array_mut().unwrap().push(source);
    v["configuration"]["root_module"]["resources"][1]["expressions"]["source_api_id"] =
        json!({"references":["aws_appsync_graphql_api.main[1].id"]});
    assert!(proven(&v, "merged_api_id"));
    assert!(proven(&v, "source_api_id"));
    v["resource_changes"][1]["change"]["after"]["merged_api_arn"] = json!("also-set");
    assert!(!proven(&v, "source_api_id"));
    v["resource_changes"][1]["change"]["after"]["merged_api_arn"] = Value::Null;
    v["resource_changes"][0]["change"]["after"]["api_type"] = json!("GRAPHQL");
    assert!(!proven(&v, "merged_api_id"));
    assert!(!proven(&v, "source_api_id"));
}

#[test]
fn additional_cognito_auth_requires_complete_typed_collection_and_known_region() {
    let mut v = fixture(super::API, super::ADDITIONAL, "aws_cognito_user_pool", "id");
    v["resource_changes"][1]["change"]["after"] = json!({"additional_authentication_provider":[{"authentication_type":"AMAZON_COGNITO_USER_POOLS","user_pool_config":[{"user_pool_id":"known","aws_region":"us-east-1"}]}]});
    v["configuration"]["root_module"]["resources"][1]["expressions"] = json!({"additional_authentication_provider":[{"user_pool_config":[{"user_pool_id":{"references":["aws_cognito_user_pool.main[0].id"]}}]}]});
    assert!(proven(&v, super::ADDITIONAL));
    v["resource_changes"][1]["change"]["after_unknown"] =
        json!({"additional_authentication_provider":[{"user_pool_config":[{"aws_region":true}]}]});
    assert!(!proven(&v, super::ADDITIONAL));
    v["resource_changes"][1]["change"]["after_unknown"] = Value::Null;
    let mut pool = v["resource_changes"][0].clone();
    pool["address"] = json!("aws_cognito_user_pool.main[1]");
    pool["change"]["after"]["id"] = json!("second");
    v["resource_changes"].as_array_mut().unwrap().push(pool);
    v["resource_changes"][1]["change"]["after"]["additional_authentication_provider"].as_array_mut().unwrap().push(json!({"authentication_type":"AMAZON_COGNITO_USER_POOLS","user_pool_config":[{"user_pool_id":"second","aws_region":"us-east-1"}]}));
    assert!(!proven(&v, super::ADDITIONAL));
    v["configuration"]["root_module"]["resources"][1]["expressions"]["additional_authentication_provider"].as_array_mut().unwrap().push(json!({"user_pool_config":[{"user_pool_id":{"references":["aws_cognito_user_pool.main[1].id"]}}]}));
    assert!(proven(&v, super::ADDITIONAL));
    v["resource_changes"][1]["change"]["after"]["additional_authentication_provider"]
        .as_array_mut()
        .unwrap()
        .swap(0, 1);
    assert!(!proven(&v, super::ADDITIONAL));
}

#[test]
fn association_bindings_verify_both_endpoints_and_allow_same_provider_arns() {
    for output in ["id", "arn"] {
        let merged_field = format!("merged_api_{output}");
        let source_field = format!("source_api_{output}");
        let mut v = fixture(super::ASSOCIATION, &merged_field, super::API, output);
        let merged = if output == "arn" {
            "arn:aws:appsync:us-east-1:111111111111:apis/merged"
        } else {
            "merged"
        };
        let source = if output == "arn" {
            "arn:aws:appsync:us-east-1:111111111111:apis/source"
        } else {
            "source"
        };
        v["resource_changes"][0]["change"]["after"] = json!({output:merged,"api_type":"MERGED"});
        v["resource_changes"][1]["change"]["after"] =
            json!({&merged_field:merged,&source_field:source});
        v["resource_changes"].as_array_mut().unwrap().push(json!({"address":"aws_appsync_graphql_api.source","type":super::API,"change":{"actions":["create"],"after":{output:source,"api_type":"GRAPHQL"}}}));
        v["configuration"]["root_module"]["resources"].as_array_mut().unwrap().push(json!({"address":"aws_appsync_graphql_api.source","provider_config_key":"aws","expressions":{}}));
        v["configuration"]["root_module"]["resources"][1]["expressions"][&source_field] =
            json!({"references":[format!("aws_appsync_graphql_api.source.{output}")]});
        assert!(proven(&v, &merged_field));
        assert!(proven(&v, &source_field));
        let verify = |v: &Value, expected: bool| {
            let raw = crate::plan::evidence::tests::parse_fixture(&v.to_string()).unwrap();
            let graph = crate::semantic::transform(&raw);
            for field in [&merged_field, &source_field] {
                assert_eq!(
                    graph
                        .nodes
                        .iter()
                        .flat_map(|n| &n.references)
                        .any(|r| r.attribute == *field
                            && r.complete
                            && Some(r.label) == super::label(super::ASSOCIATION, field)),
                    expected,
                    "{field}"
                );
            }
        };
        v["configuration"]["provider_config"]["aws"]["expressions"]["assume_role"] =
            json!([{"role_arn":{"constant_value":"arn:aws:iam::111111111111:role/Test"}}]);
        verify(&v, true);
        v["configuration"]["provider_config"]["aws.other"] =
            v["configuration"]["provider_config"]["aws"].clone();
        v["configuration"]["provider_config"]["aws.other"]["expressions"]["assume_role"] =
            json!([{"role_arn":{"constant_value":"arn:aws:iam::222222222222:role/Test"}}]);
        v["configuration"]["root_module"]["resources"][2]["provider_config_key"] =
            json!("aws.other");
        verify(&v, false);
        if output == "arn" {
            let other = "arn:aws:appsync:us-east-1:222222222222:apis/source";
            v["resource_changes"][2]["change"]["after"]["arn"] = json!(other);
            v["resource_changes"][1]["change"]["after"][&source_field] = json!(other);
            verify(&v, true);
        }
    }
}
