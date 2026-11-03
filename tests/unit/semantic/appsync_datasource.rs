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

fn nested(path: &[&str], value: Value) -> Value {
    let (head, tail) = path.split_first().unwrap();
    json!({*head:if tail.is_empty(){value}else{json!([nested(tail,value)])}})
}

fn source_fixture(path: &str, target: &str, output: &str, typ: &str) -> Value {
    let mut v = fixture(super::SOURCE, path, target, output);
    v["resource_changes"][1]["change"]["after"] =
        nested(&path.split('.').collect::<Vec<_>>(), json!("known"));
    v["resource_changes"][1]["change"]["after"]["type"] = json!(typ);
    v["resource_changes"][1]["change"]["after"]["api_id"] = json!("api");
    v["resource_changes"][1]["change"]["after"]["name"] = json!("backend");
    v["configuration"]["root_module"]["resources"][1]["expressions"] = nested(
        &path.split('.').collect::<Vec<_>>(),
        json!({"references":[format!("{target}.main[0].{output}")]}),
    );
    v["configuration"]["root_module"]["resources"][1]["expressions"]["api_id"] =
        json!({"references":["aws_appsync_graphql_api.main.id"]});
    v["resource_changes"].as_array_mut().unwrap().push(json!({"address":"aws_appsync_graphql_api.main","type":"aws_appsync_graphql_api","change":{"actions":["create"],"after":{"id":"api"}}}));
    v["configuration"]["root_module"]["resources"].as_array_mut().unwrap().push(json!({"address":"aws_appsync_graphql_api.main","provider_config_key":"aws","expressions":{}}));
    v
}

#[test]
fn datasource_backends_require_matching_type_region_and_api_context() {
    for (path, target, output, typ) in [
        (
            "lambda_config.function_arn",
            "aws_lambda_function",
            "arn",
            "AWS_LAMBDA",
        ),
        (
            "dynamodb_config.table_name",
            "aws_dynamodb_table",
            "name",
            "AMAZON_DYNAMODB",
        ),
        (
            "event_bridge_config.event_bus_arn",
            "aws_cloudwatch_event_bus",
            "arn",
            "AMAZON_EVENTBRIDGE",
        ),
        ("service_role_arn", "aws_iam_role", "arn", "AWS_LAMBDA"),
        (
            "opensearchservice_config.endpoint",
            "aws_opensearch_domain",
            "endpoint",
            "AMAZON_OPENSEARCH_SERVICE",
        ),
        (
            "elasticsearch_config.endpoint",
            "aws_elasticsearch_domain",
            "endpoint",
            "AMAZON_ELASTICSEARCH",
        ),
    ] {
        let v = source_fixture(path, target, output, typ);
        assert!(proven(&v, path), "{path}");
        for case in 0..11 {
            let mut bad = v.clone();
            match case {
                0 => bad["resource_changes"][1]["change"]["after"]["type"] = json!("NONE"),
                1 => bad["resource_changes"][0]["change"]["after_sensitive"] = json!({output:true}),
                2 => bad["resource_changes"][0]["change"]["after_unknown"] = json!({output:true}),
                3 => bad["resource_changes"][0]["mode"] = json!("data"),
                4 => bad["resource_changes"][0]["deposed"] = json!("old"),
                5 => bad["resource_changes"][1]["change"]["after"]["api_id"] = json!("other"),
                6 => bad["resource_changes"][0]["type"] = json!("aws_sqs_queue"),
                7 => bad["resource_changes"][1]["change"]["actions"] = json!(["delete"]),
                8 => {
                    bad["configuration"]["root_module"]["resources"][1]["expressions"]["api_id"] =
                        json!({"constant_value":"api"})
                }
                9 => {
                    bad["configuration"]["root_module"]["resources"][1]["provider_config_key"] =
                        json!("aws.missing")
                }
                _ => {
                    bad["resource_changes"].as_array_mut().unwrap().remove(0);
                }
            }
            assert!(!proven(&bad, path), "{path} case {case}");
        }
    }
}

#[test]
fn datasource_named_states_and_formats_keep_payloads_private() {
    let path = "lambda_config.function_arn";
    let mut v = source_fixture(path, "aws_lambda_function", "arn", "AWS_LAMBDA");
    v["resource_changes"][1]["change"]["after"]["description"] = json!("PRIVATE_PAYLOAD");
    let mut filtered = v.clone();
    filtered["resource_changes"]
        .as_array_mut()
        .unwrap()
        .remove(0);
    assert!(!proven(&filtered, path));
    for format in [crate::OutputFormat::Svg, crate::OutputFormat::Html] {
        let out = crate::generate_named(
            &[
                crate::NamedInput {
                    state: "one".into(),
                    json: v.to_string(),
                },
                crate::NamedInput {
                    state: "two".into(),
                    json: filtered.to_string(),
                },
            ],
            &crate::Options {
                format,
                ..Default::default()
            },
        )
        .unwrap()
        .content;
        assert!(!out.contains("PRIVATE_PAYLOAD"));
        assert!(out.contains("data-state-id=\"two\""));
    }
}

#[test]
fn dynamodb_region_and_versioned_delta_are_checked() {
    let path = "dynamodb_config.delta_sync_config.delta_sync_table_name";
    let mut v = source_fixture(path, "aws_dynamodb_table", "name", "AMAZON_DYNAMODB");
    assert!(!proven(&v, path));
    v["resource_changes"][1]["change"]["after"]["dynamodb_config"][0]["versioned"] = json!(true);
    assert!(proven(&v, path));
    v["resource_changes"][1]["change"]["after"]["dynamodb_config"][0]["region"] =
        json!("eu-west-1");
    assert!(!proven(&v, path));
}

#[test]
fn rds_backend_requires_both_cluster_and_secret_under_http_discriminator() {
    let path = "relational_database_config.http_endpoint_config.db_cluster_identifier";
    let mut v = source_fixture(
        path,
        "aws_rds_cluster",
        "cluster_identifier",
        "RELATIONAL_DATABASE",
    );
    v["resource_changes"][1]["change"]["after"]["relational_database_config"][0]["source_type"] =
        json!("RDS_HTTP_ENDPOINT");
    assert!(!proven(&v, path));
    v["resource_changes"].as_array_mut().unwrap().push(json!({"address":"aws_secretsmanager_secret.main","type":"aws_secretsmanager_secret","change":{"actions":["create"],"after":{"arn":"secret"}}}));
    v["configuration"]["root_module"]["resources"].as_array_mut().unwrap().push(json!({"address":"aws_secretsmanager_secret.main","provider_config_key":"aws","expressions":{}}));
    v["resource_changes"][1]["change"]["after"]["relational_database_config"][0]["http_endpoint_config"]
        [0]["aws_secret_store_arn"] = json!("secret");
    v["configuration"]["root_module"]["resources"][1]["expressions"]["relational_database_config"]
        [0]["http_endpoint_config"][0]["aws_secret_store_arn"] =
        json!({"references":["aws_secretsmanager_secret.main.arn"]});
    assert!(proven(&v, path));
    v["resource_changes"][1]["change"]["after"]["relational_database_config"][0]["source_type"] =
        json!("other");
    assert!(!proven(&v, path));
}
