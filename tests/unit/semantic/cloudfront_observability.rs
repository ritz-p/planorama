use crate::{plan, semantic};
use serde_json::{Value, json};

const FIXTURE: &str = include_str!("../../fixtures/cloudfront-observability-plan.json");
fn input() -> Value {
    serde_json::from_str(FIXTURE).unwrap()
}
fn verified(v: &Value, index: usize, path: &str) -> bool {
    let p = plan::parse(&v.to_string()).unwrap();
    let raw = semantic::Input::new(&p);
    raw.attributes.iter().any(|r| {
        raw.nodes[r.target].address == v["resource_changes"][index]["address"].as_str().unwrap()
            && r.attribute == path
            && super::super::service_references::verified(&raw, r)
    })
}

#[test]
fn subscription_embeds_under_proven_distribution_and_log_destinations_stay_shared() {
    let v = input();
    assert!(verified(&v, 5, "distribution_id"));
    for path in [super::ROLE, super::STREAM] {
        assert!(verified(&v, 2, path), "{path}");
    }
    for index in [3, 4] {
        assert!(verified(
            &v,
            index,
            "default_cache_behavior.realtime_log_config_arn"
        ));
    }
    let g = semantic::transform(&plan::parse(FIXTURE).unwrap());
    assert_eq!(g.coverage.selected.resources, 6);
    let helper = g
        .nodes
        .iter()
        .find(|n| n.resource_type == super::SUBSCRIPTION)
        .unwrap();
    assert_eq!(helper.placement, crate::model::Placement::Spatial);
    assert!(helper.member_owner.is_some());
    assert!(helper.entity.scope.as_ref().unwrap().global);
    let logs = g
        .nodes
        .iter()
        .find(|n| n.resource_type == super::LOG)
        .unwrap();
    assert_eq!(logs.placement, crate::model::Placement::Logical);
    assert!(logs.member_owner.is_none());
    assert!(logs.entity.scope.as_ref().unwrap().global);
    let stream = g
        .nodes
        .iter()
        .find(|n| n.resource_type == "aws_kinesis_stream")
        .unwrap();
    assert_eq!(
        stream
            .entity
            .scope
            .as_ref()
            .unwrap()
            .region
            .as_ref()
            .unwrap()
            .0,
        "eu-west-1"
    );
    assert!(
        !g.edges
            .iter()
            .any(|e| e.kind == crate::model::EdgeKind::Containment)
    );
    for status in [json!("Enabled"), json!("Disabled"), Value::Null] {
        let mut v = input();
        v["resource_changes"][5]["change"]["after"]["monitoring_subscription"][0]["realtime_metrics_subscription_config"]
            [0]["realtime_metrics_subscription_status"] = status;
        assert!(verified(&v, 5, "distribution_id"));
    }
}

#[test]
fn realtime_config_requires_exact_kinesis_type_singleton_shapes_and_known_typed_arns() {
    for case in 0..13 {
        let mut v = input();
        match case {
            0 => {
                v["resource_changes"][2]["change"]["after"]["endpoint"][0]["stream_type"] =
                    "Firehose".into()
            }
            1 => {
                v["resource_changes"][2]["change"]["after_unknown"] =
                    json!({"endpoint":[{"stream_type":true}]})
            }
            2 => {
                v["resource_changes"][2]["change"]["after_sensitive"] =
                    json!({"endpoint":[{"stream_type":true}]})
            }
            3 => {
                v["resource_changes"][2]["change"]["after"]["endpoint"][0]["stream_type"] =
                    Value::Null
            }
            4 => v["resource_changes"][2]["change"]["after"]["endpoint"] = json!([]),
            5 => {
                let row = v["resource_changes"][2]["change"]["after"]["endpoint"][0].clone();
                v["resource_changes"][2]["change"]["after"]["endpoint"]
                    .as_array_mut()
                    .unwrap()
                    .push(row);
            }
            6 => {
                v["resource_changes"][2]["change"]["after"]["endpoint"][0]["kinesis_stream_config"] =
                    json!([])
            }
            7 => {
                let row=v["resource_changes"][2]["change"]["after"]["endpoint"][0]["kinesis_stream_config"][0].clone();
                v["resource_changes"][2]["change"]["after"]["endpoint"][0]["kinesis_stream_config"]
                    .as_array_mut()
                    .unwrap()
                    .push(row);
            }
            8 => v["resource_changes"][2]["change"]["after_unknown"] = json!({"endpoint":[]}),
            9 => {
                v["resource_changes"][2]["change"]["after_sensitive"] =
                    json!({"endpoint":[{"kinesis_stream_config":true}]})
            }
            10 => v["resource_changes"][2]["change"]["actions"] = json!(["delete"]),
            11 => v["resource_changes"][2]["deposed"] = "old".into(),
            _ => v["resource_changes"][2]["provider_name"] = "example.com/custom/aws".into(),
        }
        for path in [super::ROLE, super::STREAM] {
            assert!(!verified(&v, 2, path), "case {case} {path}");
        }
    }
    for (source, field, path) in [
        (0, "role_arn", super::ROLE),
        (1, "stream_arn", super::STREAM),
    ] {
        for case in 0..10 {
            let mut v = input();
            match case {
                0 => v["resource_changes"][source]["change"]["after_unknown"] = json!({"arn":true}),
                1 => {
                    v["resource_changes"][source]["change"]["after_sensitive"] = json!({"arn":true})
                }
                2 => v["resource_changes"][source]["change"]["actions"] = json!(["delete"]),
                3 => v["resource_changes"][source]["deposed"] = "old".into(),
                4 => v["resource_changes"][source]["mode"] = "data".into(),
                5 => {
                    v["resource_changes"][source]["provider_name"] = "example.com/custom/aws".into()
                }
                6 => {
                    v["resource_changes"][2]["change"]["after"]["endpoint"][0]["kinesis_stream_config"]
                        [0][field] = "wrong-arn".into()
                }
                7 => {
                    v["configuration"]["root_module"]["resources"][2]["expressions"]["endpoint"]
                        [0]["kinesis_stream_config"][0][field] = json!({"constant_value":"literal"})
                }
                8 => {
                    let addr = v["resource_changes"][source]["address"].as_str().unwrap();
                    v["configuration"]["root_module"]["resources"][2]["expressions"]["endpoint"]
                        [0]["kinesis_stream_config"][0][field] =
                        json!({"references":[format!("{addr}.id")]});
                }
                _ => {
                    let addr = v["resource_changes"][source]["address"].as_str().unwrap();
                    v["configuration"]["root_module"]["resources"][2]["expressions"]["endpoint"]
                        [0]["kinesis_stream_config"][0][field] =
                        json!({"references":[format!("{addr}.arn"),"var.missing"]});
                }
            }
            assert!(!verified(&v, 2, path), "{field} case {case}");
        }
    }
    let mut v = input();
    v["configuration"]["root_module"]["resources"][2]["expressions"]["endpoint"][0]["kinesis_stream_config"]
        [0]["stream_arn"] = json!({"references":["aws_iam_role.logs.arn"]});
    assert!(!verified(&v, 2, super::STREAM));
}

#[test]
fn monitoring_owner_requires_unique_explicit_current_distribution_id() {
    for case in 0..10 {
        let mut v = input();
        match case {
            0 => {
                v["configuration"]["root_module"]["resources"][5]["expressions"]["distribution_id"] =
                    json!({"constant_value":"distribution-first"})
            }
            1 => {
                v["configuration"]["root_module"]["resources"][5]["expressions"]["distribution_id"] =
                    json!({"references":["aws_cloudfront_distribution.first.arn"]})
            }
            2 => {
                v["configuration"]["root_module"]["resources"][5]["expressions"]["distribution_id"] = json!({"references":["aws_cloudfront_distribution.first.id","aws_cloudfront_distribution.second.id"]})
            }
            3 => {
                v["resource_changes"][5]["change"]["after_unknown"] =
                    json!({"distribution_id":true})
            }
            4 => {
                v["resource_changes"][5]["change"]["after_sensitive"] =
                    json!({"distribution_id":true})
            }
            5 => v["resource_changes"][3]["change"]["after_unknown"] = json!({"id":true}),
            6 => v["resource_changes"][3]["change"]["actions"] = json!(["delete"]),
            7 => v["resource_changes"][5]["deposed"] = "old".into(),
            8 => v["resource_changes"][5]["change"]["after"]["distribution_id"] = "other".into(),
            _ => {
                v["configuration"]["root_module"]["resources"][5]["expressions"]["distribution_id"] =
                    json!({"references":["aws_kinesis_stream.logs.id"]})
            }
        }
        assert!(!verified(&v, 5, "distribution_id"), "case {case}");
        let g = semantic::transform(&plan::parse(&v.to_string()).unwrap());
        let helper = g
            .nodes
            .iter()
            .find(|n| n.resource_type == super::SUBSCRIPTION)
            .unwrap();
        assert!(helper.member_owner.is_none());
        assert_eq!(helper.placement, crate::model::Placement::Logical);
        assert_eq!(g.coverage.selected.resources, 6);
    }
}

#[test]
fn accounts_regions_filtering_named_states_and_payloads_remain_independent() {
    let mut v = input();
    v["configuration"]["provider_config"]["aws"]["expressions"]["assume_role"] =
        json!([{"role_arn":{"constant_value":"arn:aws:iam::111111111111:role/Fixture"}}]);
    let mut other = v["configuration"]["provider_config"]["aws"].clone();
    other["alias"] = "global".into();
    other["expressions"]["region"] = json!({"constant_value":"us-east-1"});
    v["configuration"]["provider_config"]["aws.global"] = other;
    for i in 2..6 {
        v["configuration"]["root_module"]["resources"][i]["provider_config_key"] =
            "aws.global".into();
    }
    let p = plan::evidence::tests::parse_fixture(&v.to_string()).unwrap();
    let raw = semantic::Input::new(&p);
    for path in [super::ROLE, super::STREAM, "distribution_id"] {
        assert!(raw.attributes.iter().any(|r|r.attribute==path&&super::super::service_references::verified(&raw,r)),"{path}");
    }
    let graph = semantic::transform(&p);
    for n in graph
        .nodes
        .iter()
        .filter(|n| n.resource_type == super::SUBSCRIPTION || n.resource_type == super::LOG)
    {
        assert!(n.entity.scope.as_ref().unwrap().account.is_some());
    }
    let selected = crate::view::single(
        &graph,
        &crate::view::Options {
            focus: Some("aws_cloudfront_monitoring_subscription.main".into()),
            depth: Some(0),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(selected.coverage.selected.resources, 1);
    for (kind, path) in [
        ("aws_iam_role", super::ROLE),
        ("aws_kinesis_stream", super::STREAM),
        ("aws_cloudfront_distribution", "distribution_id"),
    ] {
        let mut p = p.clone();
        p.nodes
            .iter_mut()
            .find(|n| n.resource_type == kind)
            .unwrap()
            .provider_configuration
            .as_mut()
            .unwrap()
            .account = crate::model::AwsAccountScope::new("222222222222");
        let raw = semantic::Input::new(&p);
        assert!(!raw.attributes.iter().any(|r|r.attribute==path&&super::super::service_references::verified(&raw,r)),"{kind}");
    }
    let mut v = input();
    v["configuration"]["provider_config"]["aws"]["expressions"]
        .as_object_mut()
        .unwrap()
        .remove("region");
    assert!(!verified(&v, 2, super::STREAM));
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
        assert!(
            !crate::generate(
                FIXTURE,
                &crate::Options {
                    format,
                    ..Default::default()
                }
            )
            .unwrap()
            .content
            .contains("PRIVATE_")
        );
    }
}
