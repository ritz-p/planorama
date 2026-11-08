use crate::{plan, semantic};
use serde_json::{Value, json};
const FIXTURE: &str = include_str!("../../fixtures/cloudfront-multitenant-plan.json");
fn complete(v: &Value, path: &str) -> bool {
    let p = plan::parse(&v.to_string()).unwrap();
    let raw = semantic::Input::new(&p);
    raw.attributes
        .iter()
        .any(|r| r.attribute == path && super::super::service_references::verified(&raw, r))
}

#[test]
fn multitenant_consumer_paths_and_tenant_groups_use_exact_outputs() {
    let v: Value = serde_json::from_str(FIXTURE).unwrap();
    for path in [
        "origin.domain_name",
        "origin.origin_access_control_id",
        "origin.vpc_origin_config.vpc_origin_id",
        "origin.custom_origin_config.origin_mtls_config.client_certificate_arn",
        "viewer_certificate.acm_certificate_arn",
        "web_acl_id",
        "distribution_id",
        "connection_group_id",
        "customizations.certificate.arn",
        "customizations.web_acl.arn",
        "anycast_ip_list_id",
    ] {
        assert!(complete(&v, path), "{path}");
    }
    for block in ["default_cache_behavior", "cache_behavior"] {
        for field in [
            "cache_policy_id",
            "lambda_function_association.lambda_function_arn",
            "function_association.function_arn",
            "trusted_key_groups.items",
            "field_level_encryption_id",
            "realtime_log_config_arn",
        ] {
            let path = format!("{block}.{field}");
            assert!(complete(&v, &path), "{path}");
        }
    }
    let g = semantic::transform(&plan::parse(FIXTURE).unwrap());
    assert_eq!(g.coverage.selected.resources, 16);
    assert_eq!(
        g.nodes
            .iter()
            .find(|n| n.resource_type == "aws_cloudfront_multitenant_distribution")
            .unwrap()
            .placement,
        crate::model::Placement::Spatial
    );
    assert!(g.nodes.iter().all(|n| n.member_owner.is_none()));
}
#[test]
fn collections_templates_and_standard_distribution_aliases_stay_conservative() {
    for case in [
        "swapped",
        "incomplete",
        "unknown",
        "sensitive",
        "literal",
        "mode",
        "standard-parent",
        "wrong-lambda-output",
        "unpublished",
        "regional-waf",
        "template",
    ] {
        let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
        let path = match case {
            "swapped" => {
                v["configuration"]["root_module"]["resources"][0]["expressions"]["cache_behavior"]
                    .as_array_mut()
                    .unwrap()
                    .swap(0, 1);
                "cache_behavior.cache_policy_id"
            }
            "incomplete" => {
                v["resource_changes"][0]["change"]["after"]["cache_behavior"]
                    .as_array_mut()
                    .unwrap()
                    .push(json!({"cache_policy_id":"literal"}));
                "cache_behavior.cache_policy_id"
            }
            "unknown" => {
                v["resource_changes"][0]["change"]["after_unknown"] =
                    json!({"cache_behavior":true});
                "cache_behavior.cache_policy_id"
            }
            "sensitive" => {
                v["resource_changes"][15]["change"]["after_sensitive"] = json!({"id":true});
                "cache_behavior.cache_policy_id"
            }
            "literal" => {
                v["configuration"]["root_module"]["resources"][1]["expressions"]["distribution_id"] =
                    json!({"constant_value":"multi-1"});
                "distribution_id"
            }
            "mode" => {
                v["resource_changes"][0]["change"]["after"]["connection_mode"] = json!("direct");
                "distribution_id"
            }
            "standard-parent" => {
                v["resource_changes"][0]["type"] = json!("aws_cloudfront_distribution");
                "distribution_id"
            }
            "wrong-lambda-output" => {
                v["configuration"]["root_module"]["resources"][0]["expressions"]["cache_behavior"]
                    [0]["lambda_function_association"][0]["lambda_function_arn"] =
                    json!({"references":["aws_lambda_function.main.arn"]});
                "cache_behavior.lambda_function_association.lambda_function_arn"
            }
            "unpublished" => {
                v["resource_changes"][7]["change"]["after"]["publish"] = json!(false);
                "cache_behavior.function_association.function_arn"
            }
            "regional-waf" => {
                v["resource_changes"][12]["change"]["after"]["scope"] = json!("REGIONAL");
                "web_acl_id"
            }
            _ => {
                v["resource_changes"][0]["change"]["after"]["origin"][0]["domain_name"] =
                    json!("{{domain}}");
                v["resource_changes"][4]["change"]["after"]["bucket_regional_domain_name"] =
                    json!("{{domain}}");
                "origin.domain_name"
            }
        };
        assert!(!complete(&v, path), "{case}");
    }
}
#[test]
fn absent_defaults_do_not_fabricate_groups_or_managed_certificates() {
    let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
    v["resource_changes"][1]["change"]["after"]["connection_group_id"] = Value::Null;
    v["resource_changes"][1]["change"]["after"]["managed_certificate_request"] =
        json!([{"primary_domain_name":"tenant.invalid"}]);
    v["configuration"]["root_module"]["resources"][1]["expressions"]
        .as_object_mut()
        .unwrap()
        .remove("connection_group_id");
    let g = semantic::transform(&plan::parse(&v.to_string()).unwrap());
    assert_eq!(g.coverage.selected.resources, 16);
    assert!(!complete(&v, "connection_group_id"));
}

#[test]
fn exact_tenant_endpoints_preserve_lifecycle_selection_and_named_state_boundaries() {
    let v: Value = serde_json::from_str(FIXTURE).unwrap();
    for (source, target, path, label) in [
        (0, 1, "distribution_id", "Multitenant distribution"),
        (2, 1, "connection_group_id", "Tenant connection group"),
        (3, 2, "anycast_ip_list_id", "Anycast IP list reference"),
    ] {
        let mut pair = v.clone();
        pair["resource_changes"] =
            json!([v["resource_changes"][source], v["resource_changes"][target]]);
        pair["configuration"]["root_module"]["resources"] = json!([
            v["configuration"]["root_module"]["resources"][source],
            v["configuration"]["root_module"]["resources"][target]
        ]);
        super::super::service_references::tests::check_service_fixture(
            &pair.to_string(),
            label,
            path,
        );
    }
    let plans = plan::parse_inputs(
        ["one", "two"]
            .into_iter()
            .map(|s| crate::model::PlanInput {
                state_id: crate::model::StateId::new(s).unwrap(),
                plan_json: FIXTURE.into(),
            })
            .collect(),
    )
    .unwrap();
    assert!(semantic::cross_state::resolve(&plans, &[]).edges.is_empty());
    let mut p = plan::parse(FIXTURE).unwrap();
    for n in &mut p.nodes {
        n.provider_configuration.as_mut().unwrap().account =
            crate::model::AwsAccountScope::new("123456789012");
    }
    p.nodes
        .iter_mut()
        .find(|n| n.resource_type == "aws_cloudfront_connection_group")
        .unwrap()
        .provider_configuration
        .as_mut()
        .unwrap()
        .account = crate::model::AwsAccountScope::new("210987654321");
    let g = semantic::transform(&p);
    assert!(
        !g.nodes
            .iter()
            .flat_map(|n| &n.references)
            .any(|r| r.label == "Tenant connection group" && r.complete)
    );
    for format in [crate::OutputFormat::Svg, crate::OutputFormat::Html] {
        assert!(
            crate::generate(
                FIXTURE,
                &crate::Options {
                    format,
                    ..Default::default()
                }
            )
            .unwrap()
            .content
            .contains("Multitenant distribution")
        );
    }
}
