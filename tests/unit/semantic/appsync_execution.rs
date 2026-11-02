use serde_json::{Value, json};

fn input() -> Value {
    let resources = [
        (
            "aws_appsync_graphql_api.main",
            json!({"id":"api"}),
            json!({}),
        ),
        (
            "aws_appsync_datasource.main",
            json!({"api_id":"api","name":"backend","type":"NONE"}),
            json!({"api_id":{"references":["aws_appsync_graphql_api.main.id"]}}),
        ),
        (
            "aws_appsync_function.a",
            json!({"api_id":"api","function_id":"fn-a","name":"same","data_source":"backend"}),
            json!({"api_id":{"references":["aws_appsync_graphql_api.main.id"]},"data_source":{"references":["aws_appsync_datasource.main.name"]}}),
        ),
        (
            "aws_appsync_function.b",
            json!({"api_id":"api","function_id":"fn-b","name":"same","data_source":"backend"}),
            json!({"api_id":{"references":["aws_appsync_graphql_api.main.id"]},"data_source":{"references":["aws_appsync_datasource.main.name"]}}),
        ),
        (
            "aws_appsync_resolver.main",
            json!({"api_id":"api","kind":"PIPELINE","type":"Query","field":"example","pipeline_config":[{"functions":["fn-b","fn-a"]}],"request_template":"PRIVATE_TEMPLATE"}),
            json!({"api_id":{"references":["aws_appsync_graphql_api.main.id"]},"pipeline_config":[{"functions":{"references":["aws_appsync_function.a.function_id","aws_appsync_function.b.function_id"]}}]}),
        ),
    ];
    json!({"format_version":"1.2","resource_changes":resources.iter().map(|(a,v,_)|json!({"address":a,"type":a.split('.').next().unwrap(),"change":{"actions":["create"],"after":v}})).collect::<Vec<_>>(),
    "configuration":{"provider_config":{"aws":{"full_name":"registry.terraform.io/hashicorp/aws","expressions":{"region":{"constant_value":"us-east-1"}}}},"root_module":{"resources":resources.iter().map(|(a,_,e)|json!({"address":a,"provider_config_key":"aws","expressions":e})).collect::<Vec<_>>()}}})
}

fn targets(v: &Value, address: &str, path: &str) -> Vec<String> {
    let g = crate::semantic::transform(&crate::plan::parse(&v.to_string()).unwrap());
    let n = g.nodes.iter().find(|n| n.address == address).unwrap();
    n.references
        .iter()
        .find(|r| {
            r.attribute == path
                && Some(r.label) == super::label(&n.resource_type, path)
                && r.complete
        })
        .map_or_else(Vec::new, |r| {
            r.targets.iter().map(|t| t.id.address.clone()).collect()
        })
}

#[test]
fn pipeline_uses_complete_ordered_function_ids_and_shared_api() {
    let v = input();
    assert_eq!(
        targets(&v, "aws_appsync_resolver.main", super::PIPELINE),
        ["aws_appsync_function.b", "aws_appsync_function.a"]
    );
    let mut repeated = v.clone();
    repeated["resource_changes"][4]["change"]["after"]["pipeline_config"][0]["functions"] =
        json!(["fn-b", "fn-a", "fn-b"]);
    repeated["resource_changes"][4]["change"]["after_unknown"] =
        json!({"pipeline_config":[{"functions":[false,false,false]}]});
    assert_eq!(
        targets(&repeated, "aws_appsync_resolver.main", super::PIPELINE),
        [
            "aws_appsync_function.b",
            "aws_appsync_function.a",
            "aws_appsync_function.b"
        ]
    );
    for case in 0..14 {
        let mut bad = v.clone();
        match case {
            0 => {
                bad["resource_changes"][4]["change"]["after"]["pipeline_config"][0]["functions"] =
                    json!(["fn-a", "missing"])
            }
            1 => bad["resource_changes"][4]["change"]["after"]["kind"] = json!("UNIT"),
            2 => bad["resource_changes"][3]["change"]["after"]["api_id"] = json!("other"),
            3 => {
                bad["resource_changes"][3]["change"]["after_unknown"] = json!({"function_id":true})
            }
            4 => {
                bad["resource_changes"][4]["change"]["after_sensitive"] =
                    json!({"pipeline_config":[{"functions":true}]})
            }
            5 => bad["resource_changes"][3]["mode"] = json!("data"),
            6 => bad["resource_changes"][3]["deposed"] = json!("old"),
            7 => bad["resource_changes"][4]["change"]["after"]["data_source"] = json!("backend"),
            8 => {
                bad["configuration"]["root_module"]["resources"][4]["expressions"]["pipeline_config"]
                    [0]["functions"] = json!({"references":["aws_appsync_function.a.function_id"]})
            }
            9 => {
                bad["configuration"]["root_module"]["resources"][3]["expressions"]["api_id"] =
                    json!({"constant_value":"api"})
            }
            10 => bad["resource_changes"][3]["change"]["actions"] = json!(["delete"]),
            11 => {
                bad["resource_changes"].as_array_mut().unwrap().remove(3);
            }
            12 => bad["resource_changes"][3]["change"]["after"]["region"] = json!("us-west-2"),
            _ => {
                bad["configuration"]["root_module"]["resources"][3]["provider_config_key"] =
                    json!("aws.missing")
            }
        }
        assert!(
            targets(&bad, "aws_appsync_resolver.main", super::PIPELINE).is_empty(),
            "case {case}"
        );
    }
}

#[test]
fn datasource_names_require_same_api_and_unit_kind() {
    let mut v = input();
    assert_eq!(
        targets(&v, "aws_appsync_function.a", "data_source"),
        ["aws_appsync_datasource.main"]
    );
    v["resource_changes"][4]["change"]["after"]["kind"] = json!("UNIT");
    v["resource_changes"][4]["change"]["after"]
        .as_object_mut()
        .unwrap()
        .remove("pipeline_config");
    v["resource_changes"][4]["change"]["after"]["data_source"] = json!("backend");
    v["configuration"]["root_module"]["resources"][4]["expressions"]["data_source"] =
        json!({"references":["aws_appsync_datasource.main.name"]});
    assert_eq!(
        targets(&v, "aws_appsync_resolver.main", "data_source"),
        ["aws_appsync_datasource.main"]
    );
    v["resource_changes"].as_array_mut().unwrap().push(json!({"address":"aws_appsync_graphql_api.other","type":"aws_appsync_graphql_api","change":{"actions":["create"],"after":{"id":"other"}}}));
    v["configuration"]["root_module"]["resources"].as_array_mut().unwrap().push(json!({"address":"aws_appsync_graphql_api.other","provider_config_key":"aws","expressions":{}}));
    v["resource_changes"][1]["change"]["after"]["api_id"] = json!("other");
    v["configuration"]["root_module"]["resources"][1]["expressions"]["api_id"] =
        json!({"references":["aws_appsync_graphql_api.other.id"]});
    assert!(targets(&v, "aws_appsync_resolver.main", "data_source").is_empty());
    assert!(targets(&v, "aws_appsync_function.a", "data_source").is_empty());
}

#[test]
fn conflict_handler_requires_lambda_discriminator_and_current_endpoint() {
    for index in [2, 4] {
        let mut v = input();
        v["resource_changes"].as_array_mut().unwrap().push(json!({"address":"aws_lambda_function.handler","type":"aws_lambda_function","change":{"actions":["create"],"after":{"arn":"handler"}}}));
        v["configuration"]["root_module"]["resources"].as_array_mut().unwrap().push(json!({"address":"aws_lambda_function.handler","provider_config_key":"aws","expressions":{}}));
        v["resource_changes"][index]["change"]["after"]["sync_config"] = json!([{"conflict_handler":"LAMBDA","lambda_conflict_handler_config":[{"lambda_conflict_handler_arn":"handler"}]}]);
        v["configuration"]["root_module"]["resources"][index]["expressions"]["sync_config"] = json!([{"lambda_conflict_handler_config":[{"lambda_conflict_handler_arn":{"references":["aws_lambda_function.handler.arn"]}}]}]);
        let address = v["resource_changes"][index]["address"]
            .as_str()
            .unwrap()
            .to_owned();
        assert_eq!(
            targets(&v, &address, super::CONFLICT),
            ["aws_lambda_function.handler"]
        );
        v["resource_changes"][index]["change"]["after"]["sync_config"][0]["conflict_handler"] =
            json!("AUTOMERGE");
        assert!(targets(&v, &address, super::CONFLICT).is_empty());
    }
}

#[test]
fn pipeline_rendering_keeps_states_and_templates_separate() {
    let v = input();
    let mut filtered = v.clone();
    filtered["resource_changes"]
        .as_array_mut()
        .unwrap()
        .remove(3);
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
        assert!(!out.contains("PRIVATE_TEMPLATE"));
        assert!(out.contains("data-state-id=\"two\""));
    }
}
