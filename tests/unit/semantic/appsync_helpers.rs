use serde_json::{Value, json};

fn input(kind: &str, api: &str, output: &str) -> Value {
    json!({"format_version":"1.2","resource_changes":[
        {"address":format!("{api}.main"),"type":api,"change":{"actions":["create"],"after":{output:"api"}}},
        {"address":format!("{kind}.main"),"type":kind,"change":{"actions":["create"],"after":{"api_id":"api","api_key_id":"PRIVATE_KEY_ID","key":"PRIVATE_KEY","id":"api:PRIVATE_COMPOSITE","definition":"PRIVATE_DEFINITION","format":"SDL","name":"Example"}}}
    ],"configuration":{"provider_config":{"aws":{"full_name":"registry.terraform.io/hashicorp/aws","expressions":{"region":{"constant_value":"us-east-1"}}}},"root_module":{"resources":[
        {"address":format!("{api}.main"),"provider_config_key":"aws","expressions":{}},
        {"address":format!("{kind}.main"),"provider_config_key":"aws","expressions":{"api_id":{"references":[format!("{api}.main.{output}")]}}}
    ]}}})
}

fn proven(v: &Value) -> bool {
    let g = crate::semantic::transform(&crate::plan::parse(&v.to_string()).unwrap());
    g.nodes.iter().any(|n| {
        n.references.iter().any(|r| {
            Some(r.label) == super::label(&n.resource_type, "api_id")
                && r.complete
                && !r.targets.is_empty()
        })
    })
}

#[test]
fn helpers_require_exact_graphql_or_event_api_output_contract() {
    for (kind, api, output) in [
        ("aws_appsync_api_cache", super::super::appsync::API, "id"),
        ("aws_appsync_type", super::super::appsync::API, "id"),
        (super::KEY, super::super::appsync::API, "id"),
        (super::KEY, "aws_appsync_api", "api_id"),
    ] {
        let v = input(kind, api, output);
        assert!(proven(&v), "{kind}/{api}");
        for case in 0..12 {
            let mut bad = v.clone();
            match case {
                0 => bad["resource_changes"][1]["change"]["after"]["api_id"] = json!("wrong"),
                1 => bad["resource_changes"][0]["change"]["after_unknown"] = json!({output:true}),
                2 => {
                    bad["resource_changes"][1]["change"]["after_sensitive"] = json!({"api_id":true})
                }
                3 => bad["resource_changes"][0]["mode"] = json!("data"),
                4 => bad["resource_changes"][0]["deposed"] = json!("old"),
                5 => bad["resource_changes"][0]["change"]["actions"] = json!(["delete"]),
                6 => bad["resource_changes"][1]["change"]["actions"] = json!(["delete"]),
                7 => {
                    bad["resource_changes"][1]["change"]["after"]["region"] = json!("us-west-2");
                    bad["configuration"]["root_module"]["resources"][1]["expressions"]["region"] =
                        json!({"constant_value":"us-west-2"});
                }
                8 => {
                    bad["configuration"]["root_module"]["resources"][1]["expressions"]["api_id"] =
                        json!({"constant_value":"api"})
                }
                9 => {
                    bad["configuration"]["root_module"]["resources"][1]["provider_config_key"] =
                        json!("aws.missing")
                }
                10 => {
                    bad["configuration"]["root_module"]["resources"][1]["expressions"]["api_id"] =
                        json!({"references":[format!("{api}.main.arn")]})
                }
                _ => {
                    bad["resource_changes"].as_array_mut().unwrap().remove(0);
                }
            }
            assert!(!proven(&bad), "{kind}/{api} case {case}");
        }
    }
    assert!(!proven(&input(super::KEY, "aws_appsync_api", "id")));
    assert!(!proven(&input(
        "aws_appsync_api_cache",
        "aws_appsync_api",
        "api_id"
    )));
    assert!(!proven(&input(
        "aws_appsync_type",
        "aws_appsync_api",
        "api_id"
    )));
}

#[test]
fn helper_coverage_and_named_states_never_expose_key_or_definition_values() {
    for kind in [super::KEY, "aws_appsync_type", "aws_appsync_api_cache"] {
        let mut v = input(kind, super::super::appsync::API, "id");
        if kind == super::KEY {
            v["resource_changes"][1]["change"]["after"]
                .as_object_mut()
                .unwrap()
                .remove("name");
        }
        let raw = crate::plan::parse(&v.to_string()).unwrap();
        let g = crate::semantic::transform(&raw);
        assert_eq!(g.coverage.selected.resources, 2);
        assert!(
            g.edges
                .iter()
                .all(|e| e.kind != crate::model::EdgeKind::Containment)
        );
        let mut filtered = v.clone();
        filtered["resource_changes"]
            .as_array_mut()
            .unwrap()
            .remove(0);
        assert!(!proven(&filtered));
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
            assert!(!out.contains("PRIVATE_"), "{kind}");
            assert!(out.contains("data-state-id=\"two\""));
        }
    }
}

#[test]
fn event_key_preserves_known_account_ownership() {
    let mut v = input(super::KEY, "aws_appsync_api", "api_id");
    v["configuration"]["provider_config"]["aws"]["expressions"]["assume_role"] =
        json!([{"role_arn":{"constant_value":"arn:aws:iam::111111111111:role/Test"}}]);
    let raw = crate::plan::evidence::tests::parse_fixture(&v.to_string()).unwrap();
    let graph = crate::semantic::transform(&raw);
    let key = graph
        .nodes
        .iter()
        .find(|n| n.resource_type == super::KEY)
        .unwrap();
    assert!(key.member_owner.is_some());
    assert_eq!(graph.coverage.selected.resources, 2);
    v["configuration"]["provider_config"]["aws.other"] =
        v["configuration"]["provider_config"]["aws"].clone();
    v["configuration"]["provider_config"]["aws.other"]["expressions"]["assume_role"] =
        json!([{"role_arn":{"constant_value":"arn:aws:iam::222222222222:role/Test"}}]);
    v["configuration"]["root_module"]["resources"][1]["provider_config_key"] = json!("aws.other");
    let raw = crate::plan::evidence::tests::parse_fixture(&v.to_string()).unwrap();
    let graph = crate::semantic::transform(&raw);
    assert!(
        graph
            .nodes
            .iter()
            .find(|n| n.resource_type == super::KEY)
            .unwrap()
            .member_owner
            .is_none()
    );
}
