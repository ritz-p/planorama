use crate::{plan, semantic};
use serde_json::{Value, json};

fn fixture(kind: &str) -> Value {
    let block = super::block(kind).unwrap();
    let auth = json!({"user_pool_arn":"pool", "user_pool_client_id":"client", "user_pool_domain":"domain", "session_cookie_name":"PRIVATE_COOKIE"});
    let refs = json!({"user_pool_arn":{"references":["aws_cognito_user_pool.main.arn"]}, "user_pool_client_id":{"references":["aws_cognito_user_pool_client.main.id"]}, "user_pool_domain":{"references":["aws_cognito_user_pool_domain.main.domain"]}});
    let definitions = [
        (
            kind,
            json!({block:[{"type":"authenticate-cognito", "authenticate_cognito":[auth]},{"type":"forward","target_group_arn":"group"}]}),
            json!({block:[{"type":{"constant_value":"authenticate-cognito"},"authenticate_cognito":[refs]},{"type":{"constant_value":"forward"},"target_group_arn":{"references":["aws_lb_target_group.main.arn"]}}]}),
        ),
        ("aws_cognito_user_pool", json!({"arn":"pool"}), json!({})),
        (
            "aws_cognito_user_pool_client",
            json!({"id":"client", "client_secret":"PRIVATE_SECRET"}),
            json!({}),
        ),
        (
            "aws_cognito_user_pool_domain",
            json!({"domain":"domain"}),
            json!({}),
        ),
        ("aws_lb_target_group", json!({"arn":"group"}), json!({})),
    ];
    json!({"format_version":"1.2", "resource_changes":definitions.iter().map(|(kind,value,_)|json!({"address":format!("{kind}.main"),"type":kind,"mode":"managed","provider_name":"registry.terraform.io/hashicorp/aws","change":{"actions":["create"],"after":value}})).collect::<Vec<_>>(),"configuration":{"provider_config":{"aws":{"name":"aws","full_name":"registry.terraform.io/hashicorp/aws","expressions":{"region":{"constant_value":"us-east-1"}}}},"root_module":{"resources":definitions.iter().map(|(kind,_,expr)|json!({"address":format!("{kind}.main"),"type":kind,"mode":"managed","provider_config_key":"aws","expressions":expr})).collect::<Vec<_>>()}}})
}

fn verified(v: &Value, field: &str) -> bool {
    let graph = plan::parse(&v.to_string()).unwrap();
    let raw = semantic::Input::new(&graph);
    raw.attributes.iter().any(|r| {
        r.attribute
            .ends_with(&format!("authenticate_cognito.{field}"))
            && super::super::service_references::verified(&raw, r)
    })
}

#[test]
fn listener_and_rule_aliases_have_typed_cognito_references() {
    for kind in [
        "aws_lb_listener",
        "aws_alb_listener",
        "aws_lb_listener_rule",
        "aws_alb_listener_rule",
    ] {
        let v = fixture(kind);
        for field in ["user_pool_arn", "user_pool_client_id", "user_pool_domain"] {
            assert!(verified(&v, field), "{kind} {field}");
        }
        let graph = semantic::transform(&plan::parse(&v.to_string()).unwrap());
        assert_eq!(graph.coverage.selected.resources, 5);
        assert!(graph.nodes.iter().any(|n| {
            n.references
                .iter()
                .any(|r| r.label == "Authentication user pool" && r.complete)
        }));
        assert!(
            graph
                .nodes
                .iter()
                .any(|n| n.address == "aws_cognito_user_pool.main")
        );
        let output = crate::generate_named(
            &[crate::NamedInput {
                state: "identity".into(),
                json: v.to_string(),
            }],
            &Default::default(),
        )
        .unwrap()
        .content;
        assert!(!output.contains("PRIVATE_COOKIE"));
        assert!(!output.contains("PRIVATE_SECRET"));
    }
}

#[test]
fn empty_non_authentication_blocks_accept_only_empty_or_known_masks() {
    for kind in [
        "aws_lb_listener",
        "aws_alb_listener",
        "aws_lb_listener_rule",
        "aws_alb_listener_rule",
    ] {
        let block = super::block(kind).unwrap();
        let mut v = fixture(kind);
        v["resource_changes"][0]["change"]["after"][block][1]["authenticate_cognito"] = json!([]);
        for masks in [
            (json!([]), json!([])),
            (json!([]), json!(false)),
            (json!(false), json!([])),
        ] {
            v["resource_changes"][0]["change"]["after_unknown"] =
                json!({block:[{}, {"authenticate_cognito":masks.0}]});
            v["resource_changes"][0]["change"]["after_sensitive"] =
                json!({block:[{}, {"authenticate_cognito":masks.1}]});
            for field in ["user_pool_arn", "user_pool_client_id", "user_pool_domain"] {
                assert!(verified(&v, field), "{kind} {field} {masks:?}");
                for mask in [json!(true), json!([false]), json!([{}]), json!({})] {
                    for mask_field in ["after_unknown", "after_sensitive"] {
                        let mut bad = v.clone();
                        bad["resource_changes"][0]["change"][mask_field][block][1]["authenticate_cognito"] =
                            mask.clone();
                        assert!(!verified(&bad, field), "{kind} {field} {mask_field} {mask}");
                    }
                }
            }
        }
        v["resource_changes"][0]["change"]["after"][block][1]["authenticate_cognito"] = Value::Null;
        assert!(!verified(&v, "user_pool_arn"));
        v["resource_changes"][0]["change"]["after"][block][1]["authenticate_cognito"] = json!([{}]);
        assert!(!verified(&v, "user_pool_arn"));
    }
}

#[test]
fn action_lists_require_element_correlated_complete_references() {
    for kind in ["aws_lb_listener", "aws_lb_listener_rule"] {
        let block = super::block(kind).unwrap();
        let mut v = fixture(kind);
        let extra = v["resource_changes"][0]["change"]["after"][block][0].clone();
        v["resource_changes"][0]["change"]["after"][block]
            .as_array_mut()
            .unwrap()
            .push(extra);
        let extra =
            v["configuration"]["root_module"]["resources"][0]["expressions"][block][0].clone();
        v["configuration"]["root_module"]["resources"][0]["expressions"][block]
            .as_array_mut()
            .unwrap()
            .push(extra);
        let mut second = v["resource_changes"][1].clone();
        second["address"] = "aws_cognito_user_pool.other".into();
        second["change"]["after"]["arn"] = "other-pool".into();
        v["resource_changes"].as_array_mut().unwrap().push(second);
        let mut second = v["configuration"]["root_module"]["resources"][1].clone();
        second["address"] = "aws_cognito_user_pool.other".into();
        v["configuration"]["root_module"]["resources"]
            .as_array_mut()
            .unwrap()
            .push(second);
        v["resource_changes"][0]["change"]["after"][block][2]["authenticate_cognito"][0]["user_pool_arn"] =
            "other-pool".into();
        v["configuration"]["root_module"]["resources"][0]["expressions"][block][2]["authenticate_cognito"]
            [0]["user_pool_arn"] = json!({"references":["aws_cognito_user_pool.other.arn"]});
        assert!(verified(&v, "user_pool_arn"));
        for case in 0..8 {
            let mut bad = v.clone();
            match case {
                0 => bad["resource_changes"][0]["change"]["after"][block]
                    .as_array_mut()
                    .unwrap()
                    .swap(0, 1),
                1 => {
                    bad["resource_changes"][0]["change"]["after"][block]
                        .as_array_mut()
                        .unwrap()
                        .pop();
                }
                2 => {
                    bad["resource_changes"][0]["change"]["after_unknown"] =
                        json!({block:[{},{},{"type":true}]})
                }
                3 => {
                    bad["resource_changes"][0]["change"]["after_sensitive"] =
                        json!({block:[{},{},{"authenticate_cognito":[{"user_pool_arn":true}]}]})
                }
                4 => {
                    bad["configuration"]["root_module"]["resources"][0]["expressions"][block][2]["authenticate_cognito"]
                        [0]["user_pool_arn"] = json!({"constant_value":"pool"})
                }
                5 => {
                    bad["resource_changes"][0]["change"]["after"][block][2]["authenticate_cognito"]
                        .as_array_mut()
                        .unwrap()
                        .push(json!({"user_pool_arn":"pool"}))
                }
                6 => bad["resource_changes"][0]["change"]["after_unknown"] = json!({block:true}),
                _ => bad["resource_changes"][0]["change"]["after"][block]
                    .as_array_mut()
                    .unwrap()
                    .swap(0, 2),
            }
            assert!(!verified(&bad, "user_pool_arn"), "{kind} case {case}");
        }
    }
}

#[test]
fn unproven_endpoints_and_non_cognito_actions_stay_conservative() {
    for (field, source, output) in [
        ("user_pool_arn", 1, "arn"),
        ("user_pool_client_id", 2, "id"),
        ("user_pool_domain", 3, "domain"),
    ] {
        let v = fixture("aws_lb_listener_rule");
        for case in 0..15 {
            let mut bad = v.clone();
            match case {
                0 => {
                    bad["resource_changes"][source]["change"]["after"][output] = "different".into()
                }
                1 => {
                    bad["resource_changes"][source]["change"]["after_sensitive"] =
                        json!({output:true})
                }
                2 => {
                    bad["resource_changes"][source]["change"]["after_unknown"] =
                        json!({output:true})
                }
                3 => bad["resource_changes"][source]["change"]["actions"] = json!(["delete"]),
                4 => bad["resource_changes"][source]["deposed"] = "old".into(),
                5 => bad["resource_changes"][source]["mode"] = "data".into(),
                6 => {
                    bad["resource_changes"][source]["provider_name"] =
                        "example.com/custom/aws".into()
                }
                7 => {
                    bad["resource_changes"][0]["change"]["after"]["action"][0]["type"] =
                        "authenticate-oidc".into()
                }
                8 => {
                    bad["resource_changes"][0]["change"]["after"]["action"][0]["type"] =
                        "jwt-validation".into()
                }
                9 => {
                    bad["resource_changes"][0]["change"]["after"]["action"][0]["type"] =
                        "redirect".into()
                }
                10 => {
                    bad["configuration"]["root_module"]["resources"][0]["expressions"]["action"]
                        [0]["authenticate_cognito"][0][field] =
                        json!({"references":["aws_lb_target_group.main.arn"]})
                }
                11 => {
                    bad["configuration"]["root_module"]["resources"][0]["expressions"]["action"]
                        [0]["authenticate_cognito"][0][field] = json!({"constant_value":v["resource_changes"][source]["change"]["after"][output]})
                }
                12 => {
                    bad["resource_changes"]
                        .as_array_mut()
                        .unwrap()
                        .remove(source);
                }
                13 => {
                    bad["configuration"]["provider_config"]["other"] =
                        bad["configuration"]["provider_config"]["aws"].clone();
                    bad["configuration"]["provider_config"]["other"]["expressions"]["region"] =
                        json!({"constant_value":"us-west-2"});
                    bad["configuration"]["root_module"]["resources"][source]["provider_config_key"] =
                        "other".into();
                }
                _ => {
                    bad["resource_changes"][0]["change"]["after_unknown"] =
                        json!({"action":[{"authenticate_cognito":true},{}]})
                }
            }
            assert!(!verified(&bad, field), "{field} case {case}");
        }
    }
}

#[test]
fn named_states_do_not_resolve_missing_cognito_endpoints_across_inputs() {
    let complete = fixture("aws_lb_listener");
    let mut missing = complete.clone();
    missing["resource_changes"]
        .as_array_mut()
        .unwrap()
        .remove(1);
    assert!(!verified(&missing, "user_pool_arn"));
    for format in [crate::OutputFormat::Svg, crate::OutputFormat::Html] {
        let output = crate::generate_named(
            &[
                crate::NamedInput {
                    state: "complete".into(),
                    json: complete.to_string(),
                },
                crate::NamedInput {
                    state: "missing".into(),
                    json: missing.to_string(),
                },
            ],
            &crate::Options {
                format,
                ..Default::default()
            },
        )
        .unwrap()
        .content;
        assert!(output.contains("data-state-id=\"complete\""));
        assert!(output.contains("data-state-id=\"missing\""));
        assert!(!output.contains("<metadata id=\"cross-state-relationships\">"));
        assert!(!output.contains("PRIVATE_SECRET"));
    }
}
