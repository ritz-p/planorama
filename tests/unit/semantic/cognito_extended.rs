use crate::{model::Placement, plan, semantic};
use serde_json::{Value, json};
const FIXTURE: &str = include_str!("../../fixtures/cognito-extended-plan.json");
fn graph(v: &Value) -> crate::model::ArchitectureGraph {
    semantic::transform(&plan::evidence::tests::parse_fixture(&v.to_string()).unwrap())
}
fn verified(v: &Value, t: usize, p: &str) -> bool {
    graph(v)
        .nodes
        .iter()
        .find(|n| n.address == v["resource_changes"][t]["address"])
        .unwrap()
        .references
        .iter()
        .any(|r| {
            r.attribute == p
                && r.complete
                && Some(r.label)
                    == super::label(v["resource_changes"][t]["type"].as_str().unwrap(), p)
        })
}
fn set(v: &mut Value, p: &str, x: Value) {
    *v.pointer_mut(&format!("/{}", p.replace('.', "/"))).unwrap() = x;
}
fn owner(v: &Value, t: usize) -> Option<String> {
    graph(v)
        .nodes
        .iter()
        .find(|n| n.address == v["resource_changes"][t]["address"])
        .unwrap()
        .member_owner
        .as_ref()
        .map(|o| o.target.address.clone())
}
#[test]
fn cognito_extended_typed_references_keep_existing_cards_and_independent_helpers() {
    let v: Value = serde_json::from_str(FIXTURE).unwrap();
    for (t, p) in [
        (11, "lambda_config.pre_sign_up"),
        (11, "lambda_config.custom_email_sender.lambda_arn"),
        (11, "lambda_config.pre_token_generation_config.lambda_arn"),
        (11, "lambda_config.kms_key_id"),
        (11, "email_configuration.source_arn"),
        (11, "sms_configuration.sns_caller_arn"),
        (12, "supported_identity_providers"),
        (12, "analytics_configuration.application_id"),
        (12, "analytics_configuration.role_arn"),
        (13, "openid_connect_provider_arns"),
        (13, "saml_provider_arns"),
        (14, "identity_pool_id"),
        (14, "identity_provider_name"),
        (15, "identity_pool_id"),
        (15, "roles"),
        (15, "role_mapping.identity_provider"),
        (15, "role_mapping.mapping_rule.role_arn"),
        (16, "user_pool_id"),
        (17, "user_pool_id"),
        (
            17,
            "log_configurations.cloud_watch_logs_configuration.log_group_arn",
        ),
        (17, "log_configurations.firehose_configuration.stream_arn"),
        (18, "client_id"),
        (19, "user_pool_id"),
        (20, "user_pool_id"),
        (21, "client_id"),
        (
            21,
            "account_takeover_risk_configuration.notify_configuration.source_arn",
        ),
        (22, "user_pool_id"),
        (23, "role_arn"),
        (24, "group_name"),
        (24, "username"),
        (25, "certificate_arn"),
        (26, "user_pool_id"),
        (27, "identity_provider_name"),
        (28, "identity_provider_name"),
    ] {
        assert!(verified(&v, t, p), "{t} {p}");
    }
    let g = graph(&v);
    assert_eq!(g.coverage.selected.resources, 30);
    assert_eq!(g.nodes.len(), 30);
    for t in [14, 15, 16, 17, 18, 20, 21, 22, 23, 25, 26, 27, 28] {
        assert!(owner(&v, t).is_some(), "{t}");
    }
    assert_eq!(
        owner(&v, 18).as_deref(),
        Some("aws_cognito_user_pool_client.main")
    );
    assert_eq!(
        owner(&v, 21).as_deref(),
        Some("aws_cognito_user_pool_client.main")
    );
    assert_eq!(owner(&v, 26).as_deref(), Some("aws_cognito_user_pool.main"));
    for t in [11, 12, 13, 19, 24] {
        assert!(owner(&v, t).is_none(), "{t}");
    }
    for n in g.nodes.iter().filter(|n| super::resource(&n.resource_type)) {
        assert!(
            n.references
                .iter()
                .all(|r| !super::indexed(&n.resource_type, &r.attribute))
        );
        if !matches!(
            n.resource_type.as_str(),
            super::POOL | super::CLIENT | super::IDENTITY
        ) {
            assert_eq!(n.placement, Placement::Logical);
        }
    }
    let old = semantic::transform(
        &plan::parse(include_str!("../../fixtures/cognito-plan.json")).unwrap(),
    );
    assert_eq!(old.coverage.selected.resources, 3);
    assert!(old.nodes.iter().any(|n| {
        n.references
            .iter()
            .any(|r| r.complete && r.label == "User pool reference")
    }));
}
#[test]
fn cognito_extended_endpoint_contracts_reject_unknown_literal_foreign_and_historical_evidence() {
    for (t, path, p, s, out, wrong) in [
        (
            11,
            "lambda_config.0.pre_sign_up",
            "lambda_config.pre_sign_up",
            2,
            "arn",
            "id",
        ),
        (
            12,
            "analytics_configuration.0.application_id",
            "analytics_configuration.application_id",
            4,
            "application_id",
            "id",
        ),
        (
            13,
            "openid_connect_provider_arns",
            "openid_connect_provider_arns",
            9,
            "arn",
            "url",
        ),
        (
            14,
            "identity_provider_name",
            "identity_provider_name",
            9,
            "arn",
            "url",
        ),
        (16, "user_pool_id", "user_pool_id", 11, "id", "arn"),
        (18, "client_id", "client_id", 12, "id", "name"),
        (24, "group_name", "group_name", 23, "name", "id"),
        (24, "username", "username", 22, "username", "sub"),
        (25, "certificate_arn", "certificate_arn", 5, "arn", "id"),
    ] {
        for case in [
            "literal",
            "unknown",
            "sensitive",
            "wrong-output",
            "mismatch",
            "data",
            "delete",
            "deposed",
            "provider",
            "source-sensitive",
            "consumer-delete",
        ] {
            let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
            match case {
                "literal" => set(
                    &mut v,
                    &format!("configuration.root_module.resources.{t}.expressions.{path}"),
                    json!({"constant_value":"literal"}),
                ),
                "unknown" | "sensitive" => {
                    v["resource_changes"][t]["change"][if case == "unknown" {
                        "after_unknown"
                    } else {
                        "after_sensitive"
                    }] = json!({path.split('.').next().unwrap():true})
                }
                "wrong-output" => {
                    let a = v["resource_changes"][s]["address"]
                        .as_str()
                        .unwrap()
                        .to_string();
                    set(
                        &mut v,
                        &format!("configuration.root_module.resources.{t}.expressions.{path}"),
                        json!({"references":[format!("{a}.{wrong}")]}),
                    );
                }
                "mismatch" => v["resource_changes"][s]["change"]["after"][out] = json!("other"),
                "data" => v["resource_changes"][s]["mode"] = json!("data"),
                "delete" => v["resource_changes"][s]["change"]["actions"] = json!(["delete"]),
                "deposed" => v["resource_changes"][s]["deposed"] = json!("old"),
                "provider" => {
                    v["resource_changes"][s]["provider_name"] =
                        json!("registry.terraform.io/hashicorp/google")
                }
                "source-sensitive" => {
                    v["resource_changes"][s]["change"]["after_sensitive"] = json!({out:true})
                }
                "consumer-delete" => {
                    v["resource_changes"][t]["change"]["actions"] = json!(["delete"])
                }
                _ => unreachable!(),
            }
            assert!(!verified(&v, t, p), "{t} {p} {case}");
        }
    }
}
#[test]
fn cognito_extended_maps_sets_and_mapping_discriminators_have_distinct_contracts() {
    for case in [
        "role-key",
        "role-unknown",
        "role-literal",
        "role-partial",
        "builtin-only",
        "provider-type",
        "provider-partial",
        "provider-duplicate",
        "provider-unknown",
        "mapping-type",
        "mapping-provider",
        "mapping-parent",
        "mapping-version",
        "mapping-foreign",
    ] {
        let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
        let (t, p) = match case {
            "role-key" => {
                v["resource_changes"][15]["change"]["after"]["roles"]["external"] =
                    v["resource_changes"][0]["change"]["after"]["arn"].clone();
                (15, "roles")
            }
            "role-unknown" => {
                v["resource_changes"][15]["change"]["after_unknown"] =
                    json!({"roles":{"authenticated":true}});
                (15, "roles")
            }
            "role-literal" => {
                v["configuration"]["root_module"]["resources"][15]["expressions"]["roles"] =
                    json!({"constant_value":{}});
                (15, "roles")
            }
            "role-partial" => {
                v["resource_changes"][15]["change"]["after"]["roles"]["authenticated"] =
                    json!("external");
                (15, "roles")
            }
            "builtin-only" => {
                v["resource_changes"][12]["change"]["after"]["supported_identity_providers"] =
                    json!(["COGNITO"]);
                (12, "supported_identity_providers")
            }
            "provider-type" => {
                v["resource_changes"][16]["change"]["after"]["provider_type"] = json!("Other");
                (12, "supported_identity_providers")
            }
            "provider-partial" => {
                v["resource_changes"][12]["change"]["after"]["supported_identity_providers"]
                    .as_array_mut()
                    .unwrap()
                    .push(json!("external"));
                (12, "supported_identity_providers")
            }
            "provider-duplicate" => {
                v["resource_changes"][12]["change"]["after"]["supported_identity_providers"]
                    .as_array_mut()
                    .unwrap()
                    .push(json!("CorpSAML"));
                (12, "supported_identity_providers")
            }
            "provider-unknown" => {
                v["resource_changes"][12]["change"]["after_unknown"] =
                    json!({"supported_identity_providers":true});
                (12, "supported_identity_providers")
            }
            "mapping-type" => {
                v["resource_changes"][15]["change"]["after"]["role_mapping"][0]["type"] =
                    json!("Token");
                (15, "role_mapping.mapping_rule.role_arn")
            }
            "mapping-provider" => {
                v["configuration"]["root_module"]["resources"][15]["expressions"]["role_mapping"]
                    [0]["identity_provider"] = json!({"constant_value":"literal"});
                (15, "role_mapping.mapping_rule.role_arn")
            }
            "mapping-parent" => {
                v["resource_changes"][12]["change"]["after"]["user_pool_id"] =
                    json!("us-west-2_other");
                v["configuration"]["root_module"]["resources"][12]["expressions"]["user_pool_id"] =
                    json!({"references":["aws_cognito_user_pool.other.id"]});
                (15, "role_mapping.identity_provider")
            }
            "mapping-version" => {
                v["resource_changes"][15]["change"]["after_unknown"] =
                    json!({"role_mapping":[{}, {"type":true}]});
                (15, "role_mapping.identity_provider")
            }
            "mapping-foreign" => {
                let mut a = v["configuration"]["provider_config"]["aws"].clone();
                a["alias"] = json!("foreign");
                a["expressions"]["assume_role"][0]["role_arn"]["constant_value"] =
                    json!("arn:aws:iam::999999999999:role/Test");
                v["configuration"]["provider_config"]["aws.foreign"] = a;
                v["configuration"]["root_module"]["resources"][9]["provider_config_key"] =
                    json!("aws.foreign");
                (15, "role_mapping.mapping_rule.role_arn")
            }
            _ => unreachable!(),
        };
        assert!(!verified(&v, t, p), "{case}");
        assert!(verified(&v, 15, "identity_pool_id"));
    }
    let v: Value = serde_json::from_str(FIXTURE).unwrap();
    assert!(verified(&v, 15, "roles"));
    assert!(verified(&v, 12, "supported_identity_providers"));
}
#[test]
fn cognito_extended_parent_pairs_scope_and_optional_clients_do_not_use_name_equality() {
    for (t, p, s) in [
        (18, "client_id", 12),
        (21, "client_id", 12),
        (24, "group_name", 23),
        (24, "username", 22),
        (12, "supported_identity_providers", 16),
    ] {
        let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
        v["resource_changes"][s]["change"]["after"]["user_pool_id"] = json!("us-west-2_other");
        v["configuration"]["root_module"]["resources"][s]["expressions"]["user_pool_id"] =
            json!({"references":["aws_cognito_user_pool.other.id"]});
        assert!(!verified(&v, t, p), "{t} {p}");
    }
    for case in ["wrong-region", "foreign-account", "unknown-account"] {
        let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
        match case {
            "wrong-region" => {
                v["configuration"]["provider_config"]["aws.east"]["expressions"]["region"]["constant_value"] =
                    json!("us-west-2")
            }
            "foreign-account" => {
                v["configuration"]["provider_config"]["aws.east"]["expressions"]["assume_role"][0]
                    ["role_arn"]["constant_value"] = json!("arn:aws:iam::999999999999:role/Test")
            }
            _ => {
                v["configuration"]["provider_config"]["aws.east"]["expressions"]["assume_role"] =
                    json!([])
            }
        }
        assert!(!verified(&v, 25, "certificate_arn"), "{case}");
        assert!(verified(&v, 25, "user_pool_id"));
    }
    let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
    v["resource_changes"][21]["change"]["after"]["client_id"] = Value::Null;
    v["configuration"]["root_module"]["resources"][21]["expressions"]
        .as_object_mut()
        .unwrap()
        .remove("client_id");
    assert_eq!(owner(&v, 21).as_deref(), Some("aws_cognito_user_pool.main"));
    assert!(!verified(&v, 26, "client_id"));
    v["resource_changes"][18]["change"]["after"]["client_id"] = json!("managed-client");
    v["configuration"]["root_module"]["resources"][18]["expressions"]["client_id"] =
        json!({"references":["aws_cognito_managed_user_pool_client.lookup.id"]});
    assert_eq!(
        owner(&v, 18).as_deref(),
        Some("aws_cognito_managed_user_pool_client.lookup")
    );
    assert!(owner(&v, 19).is_none());
}
#[test]
fn cognito_extended_nested_modes_require_matching_log_analytics_and_sender_contracts() {
    for case in [
        "log-level",
        "log-source",
        "log-unknown",
        "sender-version",
        "token-version",
        "kms-output",
        "analytics-conflict",
    ] {
        let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
        let (t, p) = match case {
            "log-level" => {
                v["resource_changes"][17]["change"]["after"]["log_configurations"][1]["log_level"] =
                    json!("ERROR");
                (17, "log_configurations.firehose_configuration.stream_arn")
            }
            "log-source" => {
                v["resource_changes"][17]["change"]["after"]["log_configurations"][1]["event_source"] =
                    json!("userNotification");
                (17, "log_configurations.firehose_configuration.stream_arn")
            }
            "log-unknown" => {
                v["resource_changes"][17]["change"]["after_unknown"] =
                    json!({"log_configurations":true});
                (17, "log_configurations.firehose_configuration.stream_arn")
            }
            "sender-version" => {
                v["resource_changes"][11]["change"]["after"]["lambda_config"][0]["custom_email_sender"]
                    [0]["lambda_version"] = json!("V2_0");
                (11, "lambda_config.custom_email_sender.lambda_arn")
            }
            "token-version" => {
                v["resource_changes"][11]["change"]["after"]["lambda_config"][0]["pre_token_generation_config"]
                    [0]["lambda_version"] = json!("V9_0");
                (11, "lambda_config.pre_token_generation_config.lambda_arn")
            }
            "kms-output" => {
                v["resource_changes"][1]["change"]["after"]["key_id"] =
                    v["resource_changes"][1]["change"]["after"]["arn"].clone();
                v["configuration"]["root_module"]["resources"][11]["expressions"]["lambda_config"]
                    [0]["kms_key_id"] = json!({"references":["aws_kms_key.main.key_id"]});
                (11, "lambda_config.kms_key_id")
            }
            "analytics-conflict" => {
                v["resource_changes"][12]["change"]["after"]["analytics_configuration"][0]["application_arn"] =
                    v["resource_changes"][4]["change"]["after"]["arn"].clone();
                (12, "analytics_configuration.application_id")
            }
            _ => unreachable!(),
        };
        assert!(!verified(&v, t, p), "{case}");
    }
    let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
    let app = v["resource_changes"][4]["change"]["after"]["arn"].clone();
    v["resource_changes"][12]["change"]["after"]["analytics_configuration"] =
        json!([{"application_arn":app,"user_data_shared":false}]);
    v["configuration"]["root_module"]["resources"][12]["expressions"]["analytics_configuration"] =
        json!([{"application_arn":{"references":["aws_pinpoint_app.main.arn"]}}]);
    assert!(verified(&v, 12, "analytics_configuration.application_arn"));
    let bucket = v["resource_changes"][7]["change"]["after"]["arn"].clone();
    v["resource_changes"][17]["change"]["after"]["log_configurations"][1] = json!({"event_source":"userAuthEvents","log_level":"INFO","s3_configuration":[{"bucket_arn":bucket}]});
    v["configuration"]["root_module"]["resources"][17]["expressions"]["log_configurations"][1] =
        json!({"s3_configuration":[{"bucket_arn":{"references":["aws_s3_bucket.logs.arn"]}}]});
    assert!(verified(
        &v,
        17,
        "log_configurations.s3_configuration.bucket_arn"
    ));
}
#[test]
fn cognito_extended_lifecycle_named_states_filtering_and_private_payloads_stay_independent() {
    for t in 14..=28 {
        let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
        v["resource_changes"][t]["change"]["actions"] = json!(["delete", "create"]);
        v["resource_changes"][t]["change"]["importing"] = json!({"id":"pool,helper"});
        v["resource_changes"][t]["previous_address"] = json!("aws_cognito_user.old");
        assert_eq!(graph(&v).coverage.selected.resources, 30);
    }
    let v: Value = serde_json::from_str(FIXTURE).unwrap();
    let g = crate::view::single(
        &graph(&v),
        &crate::view::Options {
            focus: Some("aws_cognito_user_in_group.main".into()),
            depth: Some(0),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(g.coverage.selected.resources, 1);
    assert!(g.edges.is_empty());
    assert!(g.nodes[0].member_owner.is_none());
    let plans = plan::parse_inputs(
        ["a", "b"]
            .into_iter()
            .map(|s| crate::model::PlanInput {
                state_id: crate::model::StateId::new(s).unwrap(),
                plan_json: FIXTURE.into(),
            })
            .collect(),
    )
    .unwrap();
    assert!(semantic::cross_state::resolve(&plans, &[]).edges.is_empty());
    for format in [crate::OutputFormat::Svg, crate::OutputFormat::Html] {
        let output = crate::generate(
            FIXTURE,
            &crate::Options {
                format,
                ..Default::default()
            },
        )
        .unwrap();
        assert!(!output.content.contains("PRIVATE_"));
    }
}
#[test]
fn cognito_extended_trigger_allowlist_and_conflicting_token_versions_are_bounded() {
    for trigger in super::TRIGGERS.iter().filter(|p| !p.contains('.')) {
        let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
        let arn = v["resource_changes"][2]["change"]["after"]["arn"].clone();
        v["resource_changes"][11]["change"]["after"]["lambda_config"][0][*trigger] = arn;
        v["configuration"]["root_module"]["resources"][11]["expressions"]["lambda_config"][0]
            [*trigger] = json!({"references":["aws_lambda_function.trigger.arn"]});
        assert!(
            verified(&v, 11, &format!("lambda_config.{trigger}")),
            "{trigger}"
        );
    }
    let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
    v["resource_changes"][11]["change"]["after"]["lambda_config"][0]["pre_token_generation"] =
        json!("other");
    v["configuration"]["root_module"]["resources"][11]["expressions"]["lambda_config"][0]["pre_token_generation"] =
        json!({"references":["aws_lambda_function.trigger.arn"]});
    assert!(!verified(
        &v,
        11,
        "lambda_config.pre_token_generation_config.lambda_arn"
    ));
    assert!(!verified(&v, 11, "lambda_config.pre_token_generation"));
    let sender =
        v["resource_changes"][11]["change"]["after"]["lambda_config"][0]["custom_email_sender"]
            .clone();
    v["resource_changes"][11]["change"]["after"]["lambda_config"][0]["custom_sms_sender"] = sender;
    v["configuration"]["root_module"]["resources"][11]["expressions"]["lambda_config"][0]["custom_sms_sender"] =
        json!([{"lambda_arn":{"references":["aws_lambda_function.trigger.arn"]}}]);
    assert!(verified(
        &v,
        11,
        "lambda_config.custom_sms_sender.lambda_arn"
    ));
}
#[test]
fn cognito_extended_complete_block_shapes_and_provider_membership_are_required() {
    for case in [
        "provider-unconfigured",
        "provider-unknown",
        "mapping-pair",
        "mapping-extra",
        "roles-extra-mask",
        "log-shape",
        "client-parent-literal",
    ] {
        let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
        let (t, p) = match case {
            "provider-unconfigured" => {
                v["resource_changes"][13]["change"]["after"]["openid_connect_provider_arns"] =
                    json!([]);
                (14, "identity_provider_name")
            }
            "provider-unknown" => {
                v["resource_changes"][13]["change"]["after_unknown"] =
                    json!({"cognito_identity_providers":true});
                (27, "identity_provider_name")
            }
            "mapping-pair" => {
                v["resource_changes"][13]["change"]["after"]["cognito_identity_providers"][0]["client_id"] =
                    json!("other-client");
                (15, "role_mapping.identity_provider")
            }
            "mapping-extra" => {
                v["resource_changes"][15]["change"]["after"]["role_mapping"]
                    .as_array_mut()
                    .unwrap()
                    .push(json!({"identity_provider":"external","type":"Rules"}));
                (15, "role_mapping.identity_provider")
            }
            "roles-extra-mask" => {
                v["resource_changes"][15]["change"]["after_unknown"] =
                    json!({"roles":{"extra":true}});
                (15, "roles")
            }
            "log-shape" => {
                let a=v["resource_changes"][17]["change"]["after"]["log_configurations"][0]["cloud_watch_logs_configuration"][0].clone();
                v["resource_changes"][17]["change"]["after"]["log_configurations"][0]["cloud_watch_logs_configuration"] =
                    a;
                (
                    17,
                    "log_configurations.cloud_watch_logs_configuration.log_group_arn",
                )
            }
            "client-parent-literal" => {
                v["configuration"]["root_module"]["resources"][12]["expressions"]["user_pool_id"] =
                    json!({"constant_value":"us-west-2_main"});
                (18, "client_id")
            }
            _ => unreachable!(),
        };
        assert!(!verified(&v, t, p), "{case}");
    }
}
