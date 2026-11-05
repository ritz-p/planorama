use crate::{plan, semantic};
use serde_json::{Value, json};

const FIXTURE: &str = include_str!("../../fixtures/appsync-events-plan.json");
fn input() -> Value {
    serde_json::from_str(FIXTURE).unwrap()
}
fn verified(v: &Value, path: &str) -> bool {
    let p = plan::parse(&v.to_string()).unwrap();
    let raw = semantic::Input::new(&p);
    raw.attributes.iter().any(|r| {
        r.attribute == path
            && super::label(&raw.nodes[r.target].resource_type, path).is_some()
            && super::super::service_references::verified(&raw, r)
    })
}

#[test]
fn event_api_collections_and_namespace_have_typed_qualified_endpoints() {
    let v = input();
    for path in [
        super::COGNITO,
        super::LAMBDA,
        super::LOG,
        "api_id",
        super::PUBLISH,
        super::SUBSCRIBE,
    ] {
        assert!(verified(&v, path), "{path}");
    }
    let graph = semantic::transform(&plan::parse(FIXTURE).unwrap());
    let api = graph
        .nodes
        .iter()
        .find(|n| n.resource_type == super::API)
        .unwrap();
    let namespace = graph
        .nodes
        .iter()
        .find(|n| n.resource_type == super::NAMESPACE)
        .unwrap();
    assert_eq!(api.placement, crate::model::Placement::Spatial);
    assert!(crate::view::routes::owner(&graph, namespace).is_some());
    assert_eq!(graph.coverage.selected.resources, 8);
    let selected = crate::view::single(
        &graph,
        &crate::view::Options {
            focus: Some(namespace.address.clone()),
            depth: Some(0),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(selected.coverage.selected.resources, 1);
    assert!(crate::view::routes::owner(&selected, &selected.nodes[0]).is_none());
}

#[test]
fn auth_collections_require_row_correlation_discriminators_and_known_shapes() {
    for case in 0..17 {
        let mut v = input();
        match case {
            0 => {
                v["resource_changes"][4]["change"]["after"]["event_config"][0]["auth_provider"][0]
                    ["auth_type"] = "AWS_IAM".into()
            }
            1 => {
                v["resource_changes"][4]["change"]["after"]["event_config"][0]["auth_provider"][0]
                    ["cognito_config"][0]["aws_region"] = "us-west-2".into()
            }
            2 => {
                v["resource_changes"][4]["change"]["after_unknown"] =
                    json!({"event_config":[{"auth_provider":[{"auth_type":true},{},{},{}]}]})
            }
            3 => {
                v["resource_changes"][4]["change"]["after_sensitive"] = json!({"event_config":[{"auth_provider":[{"cognito_config":[{"user_pool_id":true}]},{},{},{}]}]})
            }
            4 => v["configuration"]["root_module"]["resources"][4]["expressions"]["event_config"]
                [0]["auth_provider"]
                .as_array_mut()
                .unwrap()
                .swap(0, 1),
            5 => {
                v["configuration"]["root_module"]["resources"][4]["expressions"]["event_config"]
                    [0]["auth_provider"][0]["cognito_config"][0]["user_pool_id"] =
                    json!({"constant_value":"us-east-1_first"})
            }
            6 => {
                v["configuration"]["root_module"]["resources"][4]["expressions"]["event_config"]
                    [0]["auth_provider"][0]["cognito_config"][0]["user_pool_id"] =
                    json!({"references":["aws_cognito_user_pool.first.id","var.unknown"]})
            }
            7 => v["resource_changes"][0]["change"]["after_unknown"] = json!({"id":true}),
            8 => v["resource_changes"][0]["change"]["actions"] = json!(["delete"]),
            9 => v["resource_changes"][0]["deposed"] = "old".into(),
            10 => v["resource_changes"][0]["mode"] = "data".into(),
            11 => v["resource_changes"][0]["provider_name"] = "example.com/custom/aws".into(),
            12 => {
                v["resource_changes"][4]["change"]["after_unknown"] =
                    json!({"event_config":[{"auth_provider":[{}]}]})
            }
            13 => {
                v["resource_changes"][4]["change"]["after"]["event_config"][0]["auth_provider"][3]
                    ["auth_type"] = "UNKNOWN".into()
            }
            14 => {
                v["resource_changes"][4]["change"]["after"]["event_config"][0]["auth_provider"][0]
                    ["cognito_config"] = json!([])
            }
            15 => v["resource_changes"][4]["change"]["after"]["event_config"] = json!([]),
            _ => {
                let row = v["resource_changes"][4]["change"]["after"]["event_config"][0].clone();
                v["resource_changes"][4]["change"]["after"]["event_config"]
                    .as_array_mut()
                    .unwrap()
                    .push(row);
            }
        }
        assert!(!verified(&v, super::COGNITO), "case {case}");
    }
    let mut v = input();
    v["resource_changes"][4]["change"]["after"]["event_config"][0]["auth_provider"][2]["auth_type"] =
        "API_KEY".into();
    assert!(!verified(&v, super::LAMBDA));
    let mut v = input();
    v["resource_changes"][4]["change"]["after"]["event_config"][0]["log_config"] =
        json!({"cloudwatch_logs_role_arn":"arn:aws:iam::111111111111:role/event-logs"});
    assert!(!verified(&v, super::LOG));
}

#[test]
fn namespace_data_source_names_require_the_same_explicit_event_api() {
    for case in 0..11 {
        let mut v = input();
        match case {
            0 => v["resource_changes"][5]["change"]["after"]["api_id"] = "another-api".into(),
            1 => {
                v["configuration"]["root_module"]["resources"][5]["expressions"]["api_id"] =
                    json!({"constant_value":"event-api"})
            }
            2 => {
                v["configuration"]["root_module"]["resources"][5]["expressions"]["api_id"] =
                    json!({"references":["aws_appsync_graphql_api.graphql.id"]});
                v["resource_changes"][7]["change"]["after"]["id"] = "event-api".into();
            }
            3 => {
                v["configuration"]["root_module"]["resources"][6]["expressions"]["api_id"] =
                    json!({"references":["aws_appsync_api.events.api_arn"]})
            }
            4 => v["resource_changes"][5]["change"]["after"]["name"] = "wrong".into(),
            5 => {
                v["resource_changes"][6]["change"]["after_unknown"] = json!({"handler_configs":[{"on_publish":[{"integration":[{"data_source_name":true}]}]}]})
            }
            6 => v["resource_changes"][5]["change"]["actions"] = json!(["delete"]),
            7 => v["resource_changes"][4]["change"]["actions"] = json!(["delete"]),
            8 => {
                v["resource_changes"][6]["change"]["after"]["handler_configs"][0]["on_publish"][0]
                    ["behavior"] = "UNKNOWN".into()
            }
            9 => {
                v["resource_changes"][6]["change"]["after"]["handler_configs"] =
                    v["resource_changes"][6]["change"]["after"]["handler_configs"][0].clone()
            }
            _ => v["resource_changes"][5]["change"]["after_sensitive"] = json!({"api_id":true}),
        }
        assert!(!verified(&v, super::PUBLISH), "case {case}");
    }
}

#[test]
fn final_accounts_and_named_states_do_not_create_unproven_event_links() {
    let mut p = plan::parse(FIXTURE).unwrap();
    p.nodes
        .iter_mut()
        .find(|n| n.resource_type == "aws_appsync_datasource")
        .unwrap()
        .provider_configuration
        .as_mut()
        .unwrap()
        .account = crate::model::AwsAccountScope::new("222222222222");
    let raw = semantic::Input::new(&p);
    assert!(
        !raw.attributes.iter().any(|r| r.attribute == super::PUBLISH
            && super::super::service_references::verified(&raw, r))
    );
    let plans = plan::parse_inputs(
        ["one", "two"]
            .into_iter()
            .map(|name| crate::model::PlanInput {
                state_id: crate::model::StateId::new(name).unwrap(),
                plan_json: FIXTURE.into(),
            })
            .collect(),
    )
    .unwrap();
    assert!(semantic::cross_state::resolve(&plans, &[]).edges.is_empty());
    for format in [crate::OutputFormat::Svg, crate::OutputFormat::Html] {
        let out = crate::generate(
            FIXTURE,
            &crate::Options {
                format,
                ..Default::default()
            },
        )
        .unwrap()
        .content;
        assert!(!out.contains("PRIVATE_"));
    }
}

#[test]
fn cognito_region_selectors_require_independent_account_evidence() {
    let mut v = input();
    v["configuration"]["provider_config"]["aws"]["expressions"]["assume_role"] =
        json!([{"role_arn":{"constant_value":"arn:aws:iam::111111111111:role/Fixture"}}]);
    v["configuration"]["provider_config"]["aws.west"] =
        v["configuration"]["provider_config"]["aws"].clone();
    v["configuration"]["provider_config"]["aws.west"]["expressions"]["region"] =
        json!({"constant_value":"us-west-2"});
    v["configuration"]["root_module"]["resources"][0]["provider_config_key"] = "aws.west".into();
    v["resource_changes"][0]["change"]["after"]["id"] = "us-west-2_first".into();
    v["resource_changes"][4]["change"]["after"]["event_config"][0]["auth_provider"][0]["cognito_config"]
        [0] = json!({"user_pool_id":"us-west-2_first","aws_region":"us-west-2"});
    let check = |v: &Value| {
        let p = plan::evidence::tests::parse_fixture(&v.to_string()).unwrap();
        let raw = semantic::Input::new(&p);
        raw.attributes.iter().any(|r| {
            r.attribute == super::COGNITO && super::super::service_references::verified(&raw, r)
        })
    };
    assert!(check(&v));
    assert!(!verified(&v, super::COGNITO));
    v["configuration"]["provider_config"]["aws.west"]["expressions"]["assume_role"] =
        json!([{"role_arn":{"constant_value":"arn:aws:iam::222222222222:role/Fixture"}}]);
    assert!(!check(&v));
}
