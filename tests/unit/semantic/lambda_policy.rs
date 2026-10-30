use crate::{plan, semantic};
use serde_json::{Value, json};

#[test]
fn policy_helpers_keep_named_state_coverage_and_filtered_links_without_payloads() {
    let plans = plan::parse_inputs(
        ["first", "second"]
            .into_iter()
            .map(|state| crate::model::PlanInput {
                state_id: crate::model::StateId::new(state).unwrap(),
                plan_json: fixture().to_string(),
            })
            .collect(),
    )
    .unwrap();
    assert!(semantic::cross_state::resolve(&plans, &[]).edges.is_empty());
    for raw in plans.states.values() {
        assert_eq!(semantic::transform(raw).coverage.selected.resources, 7);
        for focus in [None, Some("aws_lambda_permission.config".into())] {
            let graph = crate::generation::project(
                raw,
                &crate::view::Options {
                    depth: focus.as_ref().map(|_| 0),
                    focus,
                    ..Default::default()
                },
            )
            .unwrap();
            let svg = crate::svg::render(&graph, &crate::layout::Layout::new(&graph));
            assert!(!svg.contains("PRIVATE_"));
            let doc = roxmltree::Document::parse(&svg).unwrap();
            for href in doc
                .descendants()
                .filter_map(|n| n.attribute("href"))
                .filter_map(|s| s.strip_prefix('#'))
            {
                assert!(doc.descendants().any(|n| n.attribute("id") == Some(href)));
            }
        }
    }
}

#[test]
fn permission_sources_are_typed_and_service_principal_specific() {
    for (kind, principal, arn) in [
        (
            "aws_s3_bucket",
            "s3.amazonaws.com",
            "arn:aws:s3:::synthetic",
        ),
        (
            "aws_sns_topic",
            "sns.amazonaws.com",
            "arn:aws:sns:us-east-1:123456789012:synthetic",
        ),
        (
            "aws_cloudwatch_event_rule",
            "events.amazonaws.com",
            "arn:aws:events:us-east-1:123456789012:rule/synthetic",
        ),
        (
            "aws_cloudwatch_log_group",
            "logs.us-east-1.amazonaws.com",
            "arn:aws:logs:us-east-1:123456789012:log-group:synthetic",
        ),
        (
            "aws_cognito_user_pool",
            "cognito-idp.amazonaws.com",
            "arn:aws:cognito-idp:us-east-1:123456789012:userpool/synthetic",
        ),
        (
            "aws_lb_target_group",
            "elasticloadbalancing.amazonaws.com",
            "arn:aws:elasticloadbalancing:us-east-1:123456789012:targetgroup/synthetic/123",
        ),
    ] {
        let mut v = fixture();
        let address = format!("{kind}.source");
        v["resource_changes"][2]["type"] = json!(kind);
        v["resource_changes"][2]["address"] = json!(address);
        v["resource_changes"][2]["change"]["after"] = json!({"arn":arn,"target_type":"lambda"});
        v["configuration"]["root_module"]["resources"][2]["type"] = json!(kind);
        v["configuration"]["root_module"]["resources"][2]["address"] = json!(address);
        v["resource_changes"][3]["change"]["after"]["principal"] = json!(principal);
        v["resource_changes"][3]["change"]["after"]["source_account"] = Value::Null;
        v["resource_changes"][3]["change"]["after"]["source_arn"] =
            json!(if kind == "aws_cloudwatch_log_group" {
                format!("{arn}:*")
            } else {
                arn.into()
            });
        v["configuration"]["root_module"]["resources"][3]["expressions"]["source_arn"] =
            json!({"references":[format!("{address}.arn")]});
        assert!(
            complete(&v, "aws_lambda_permission.config", "source_arn"),
            "{kind}"
        );
        v["resource_changes"][3]["change"]["after_unknown"] = json!({"principal":true});
        assert!(
            !complete(&v, "aws_lambda_permission.config", "source_arn"),
            "unknown {kind}"
        );
    }
}
fn fixture() -> Value {
    serde_json::from_str(include_str!("../../fixtures/lambda-policy-plan.json")).unwrap()
}
fn complete(v: &Value, address: &str, field: &str) -> bool {
    semantic::transform(&plan::parse(&v.to_string()).unwrap())
        .nodes
        .iter()
        .find(|n| n.address == address)
        .unwrap()
        .references
        .iter()
        .any(|r| {
            r.attribute == field
                && r.complete
                && matches!(
                    r.label,
                    "Lambda policy resource"
                        | "Permission source"
                        | "Configured Lambda function"
                        | "Function qualifier"
                )
        })
}

#[test]
fn policy_qualified_owners_require_exact_arn_parent_and_current_provenance() {
    for case in [
        "valid",
        "wrong_version",
        "wrong_alias",
        "literal_parent",
        "unknown",
        "sensitive",
        "data",
        "deleted",
        "deposed",
        "wrong_provider",
        "literal",
        "ambiguous",
    ] {
        let mut v = fixture();
        match case {
            "wrong_version" => v["resource_changes"][0]["change"]["after"]["version"] = json!("8"),
            "wrong_alias" => {
                v["resource_changes"][1]["change"]["after"]["arn"] =
                    json!("arn:aws:lambda:us-east-1:123456789012:function:other:live")
            }
            "literal_parent" => {
                v["configuration"]["root_module"]["resources"][1]["expressions"]["function_name"] =
                    json!({"constant_value":"synthetic-function"})
            }
            "unknown" => {
                v["resource_changes"][0]["change"]["after_unknown"] = json!({"qualified_arn":true})
            }
            "sensitive" => {
                v["resource_changes"][0]["change"]["after_sensitive"] =
                    json!({"qualified_arn":true})
            }
            "data" => v["resource_changes"][0]["mode"] = json!("data"),
            "deposed" => v["resource_changes"][0]["deposed"] = json!("old"),
            "deleted" => {
                v["resource_changes"][0]["change"]["actions"] = json!(["delete"]);
                v["resource_changes"][0]["change"]["before"] =
                    v["resource_changes"][0]["change"]["after"].take();
            }
            "wrong_provider" => {
                v["resource_changes"][0]["provider_name"] = json!("example.invalid/custom/aws")
            }
            "literal" => {
                v["configuration"]["root_module"]["resources"][5]["expressions"]["resource_arn"] =
                    json!({"constant_value":"literal"})
            }
            "ambiguous" => {
                v["configuration"]["root_module"]["resources"][5]["expressions"]["resource_arn"] = json!({"references":["aws_lambda_function.app.qualified_arn","aws_lambda_alias.live.arn"]})
            }
            _ => {}
        }
        assert_eq!(
            complete(&v, "aws_lambda_resource_policy.version", "resource_arn"),
            matches!(case, "valid" | "wrong_alias" | "literal_parent"),
            "{case}"
        );
        if matches!(case, "valid" | "wrong_alias" | "literal_parent") {
            assert_eq!(
                complete(&v, "aws_lambda_resource_policy.alias", "resource_arn"),
                case == "valid",
                "{case}"
            );
        }
    }
}

#[test]
fn permission_source_arn_requires_exact_api_boundary_and_discriminators() {
    let base = "arn:aws:execute-api:us-east-1:123456789012:synthetic-api";
    for (arn, expected) in [
        (format!("{base}/*"), true),
        (format!("{base}/*/*/*"), true),
        (format!("{base}/prod/GET/path"), true),
        (format!("{base}-other/*"), false),
        (format!("prefix{base}/*"), false),
        (format!("{base}:wrong/*"), false),
        (format!("{base}/"), false),
    ] {
        let mut v = fixture();
        v["resource_changes"][3]["change"]["after"]["source_arn"] = json!(arn);
        assert_eq!(
            complete(&v, "aws_lambda_permission.config", "source_arn"),
            expected,
            "{arn}"
        );
    }
    for case in [
        "principal",
        "account",
        "unknown",
        "sensitive",
        "type",
        "literal",
        "data",
        "region",
    ] {
        let mut v = fixture();
        match case {
            "principal" => {
                v["resource_changes"][3]["change"]["after"]["principal"] =
                    json!("sns.amazonaws.com")
            }
            "account" => {
                v["resource_changes"][3]["change"]["after"]["source_account"] =
                    json!("222222222222")
            }
            "unknown" => {
                v["resource_changes"][3]["change"]["after_unknown"] = json!({"source_arn":true})
            }
            "sensitive" => {
                v["resource_changes"][2]["change"]["after_sensitive"] =
                    json!({"execution_arn":true})
            }
            "type" => v["resource_changes"][2]["type"] = json!("aws_iam_role"),
            "literal" => {
                v["configuration"]["root_module"]["resources"][3]["expressions"]["source_arn"] =
                    json!({"constant_value":base})
            }
            "data" => v["resource_changes"][2]["mode"] = json!("data"),
            _ => {
                v["configuration"]["root_module"]["resources"][2]["provider_config_key"] =
                    json!("aws.other")
            }
        }
        assert!(
            !complete(&v, "aws_lambda_permission.config", "source_arn"),
            "{case}"
        );
        assert!(
            complete(&v, "aws_lambda_permission.config", "function_name"),
            "independent owner {case}"
        );
    }
}

#[test]
fn permission_owner_qualifiers_and_qualified_arn_are_not_interchangeable() {
    for (value, expected) in [
        (json!(null), true),
        (json!("live"), true),
        (json!("$LATEST"), false),
        (json!("wrong"), false),
    ] {
        let mut v = fixture();
        v["resource_changes"][3]["change"]["after"]["qualifier"] = value.clone();
        if value.is_null() {
            v["configuration"]["root_module"]["resources"][3]["expressions"]
                .as_object_mut()
                .unwrap()
                .remove("qualifier");
        }
        assert_eq!(
            complete(&v, "aws_lambda_permission.config", "function_name"),
            expected
        );
    }
    for (index, field, address) in [
        (0, "qualified_arn", "aws_lambda_function.app"),
        (1, "arn", "aws_lambda_alias.live"),
    ] {
        let mut v = fixture();
        v["resource_changes"][3]["change"]["after"]["function_name"] =
            v["resource_changes"][index]["change"]["after"][field].clone();
        v["resource_changes"][3]["change"]["after"]["qualifier"] = Value::Null;
        let expressions = &mut v["configuration"]["root_module"]["resources"][3]["expressions"];
        expressions.as_object_mut().unwrap().remove("qualifier");
        expressions["function_name"] = json!({"references":[format!("{address}.{field}")]});
        assert!(complete(
            &v,
            "aws_lambda_permission.config",
            "function_name"
        ));
    }
}
