use crate::{plan, semantic};
use serde_json::{Value, json};

const FIXTURE: &str = include_str!("../../fixtures/cloudfront-functions-plan.json");
fn input() -> Value {
    serde_json::from_str(FIXTURE).unwrap()
}
fn verified(v: &Value, address: &str, path: &str) -> bool {
    let p = plan::parse(&v.to_string()).unwrap();
    let raw = semantic::Input::new(&p);
    raw.attributes.iter().any(|r| {
        raw.nodes[r.target].address == address
            && r.attribute == path
            && super::super::service_references::verified(&raw, r)
    })
}

#[test]
fn store_sets_nested_associations_and_helpers_preserve_shared_identity() {
    let v = input();
    for (address, path) in [
        (
            "aws_cloudfront_function.main",
            "key_value_store_associations",
        ),
        ("aws_cloudfront_connection_function.main", super::NESTED),
        (
            "aws_cloudfrontkeyvaluestore_key.main",
            "key_value_store_arn",
        ),
        (
            "aws_cloudfrontkeyvaluestore_keys_exclusive.main",
            "key_value_store_arn",
        ),
        (
            "aws_cloudfront_distribution.main",
            "default_cache_behavior.function_association.function_arn",
        ),
        (
            "aws_cloudfront_distribution.main",
            "connection_function_association.id",
        ),
    ] {
        assert!(verified(&v, address, path), "{address}/{path}");
    }
    let g = semantic::transform(&plan::parse(FIXTURE).unwrap());
    assert_eq!(g.coverage.selected.resources, 7);
    assert_eq!(
        g.nodes
            .iter()
            .filter(|n| n.resource_type == super::STORE)
            .count(),
        2
    );
    assert!(
        !g.edges
            .iter()
            .any(|e| e.kind == crate::model::EdgeKind::Containment)
    );
    for n in g.nodes.iter().filter(|n| super::helper(&n.resource_type)) {
        assert!(n.member_owner.is_some());
        assert_eq!(n.placement, crate::model::Placement::Logical);
    }
}

#[test]
fn complete_known_store_sets_and_runtime_are_required() {
    for case in 0..15 {
        let mut v = input();
        match case {
            0 => {
                v["resource_changes"][2]["change"]["after"]["runtime"] = "cloudfront-js-1.0".into()
            }
            1 => v["resource_changes"][2]["change"]["after_unknown"] = json!({"runtime":true}),
            2 => {
                v["resource_changes"][2]["change"]["after_sensitive"] =
                    json!({"key_value_store_associations":true})
            }
            3 => v["resource_changes"][0]["change"]["after_unknown"] = json!({"arn":true}),
            4 => {
                v["resource_changes"][2]["change"]["after"]["key_value_store_associations"][0] =
                    "wrong".into()
            }
            5 => {
                v["configuration"]["root_module"]["resources"][2]["expressions"]["key_value_store_associations"] =
                    json!({"constant_value":["first","second"]})
            }
            6 => {
                v["configuration"]["root_module"]["resources"][2]["expressions"]["key_value_store_associations"] =
                    json!({"references":["aws_cloudfront_key_value_store.first.arn","var.missing"]})
            }
            7 => {
                v["configuration"]["root_module"]["resources"][2]["expressions"]["key_value_store_associations"] = json!({"references":["aws_cloudfront_key_value_store.first.id","aws_cloudfront_key_value_store.second.id"]})
            }
            8 => v["resource_changes"][0]["change"]["actions"] = json!(["delete"]),
            9 => v["resource_changes"][0]["deposed"] = "old".into(),
            10 => v["resource_changes"][0]["mode"] = "data".into(),
            11 => v["resource_changes"][0]["provider_name"] = "example.com/custom/aws".into(),
            12 => {
                v["resource_changes"][2]["change"]["after_unknown"] =
                    json!({"key_value_store_associations":[false]})
            }
            13 => {
                v["resource_changes"][2]["change"]["after"]["key_value_store_associations"] =
                    json!([])
            }
            _ => {
                v["resource_changes"][1]["change"]["after"]["arn"] =
                    v["resource_changes"][0]["change"]["after"]["arn"].clone()
            }
        }
        assert!(
            !verified(
                &v,
                "aws_cloudfront_function.main",
                "key_value_store_associations"
            ),
            "case {case}"
        );
    }
    let mut reordered = input();
    reordered["resource_changes"][2]["change"]["after"]["key_value_store_associations"]
        .as_array_mut()
        .unwrap()
        .reverse();
    assert!(verified(
        &reordered,
        "aws_cloudfront_function.main",
        "key_value_store_associations"
    ));
}

#[test]
fn publish_gates_incoming_consumers_without_erasing_development_store_configuration() {
    for (index, field, path, address) in [
        (
            2,
            "default_cache_behavior.function_association.function_arn",
            "key_value_store_associations",
            "aws_cloudfront_function.main",
        ),
        (
            3,
            "connection_function_association.id",
            super::NESTED,
            "aws_cloudfront_connection_function.main",
        ),
    ] {
        for case in 0..4 {
            let mut v = input();
            match case {
                0 => v["resource_changes"][index]["change"]["after"]["publish"] = false.into(),
                1 => {
                    v["resource_changes"][index]["change"]["after_unknown"] =
                        json!({"publish":true})
                }
                2 => {
                    v["resource_changes"][index]["change"]["after_sensitive"] =
                        json!({"publish":true})
                }
                _ => {
                    v["resource_changes"][index]["change"]["after"]
                        .as_object_mut()
                        .unwrap()
                        .remove("publish");
                }
            }
            assert!(
                !verified(&v, "aws_cloudfront_distribution.main", field),
                "{field} case {case}"
            );
            assert!(verified(&v, address, path));
        }
    }
    let mut v = input();
    v["configuration"]["root_module"]["resources"][6]["expressions"]["connection_function_association"]
        [0]["id"] =
        json!({"references":["aws_cloudfront_connection_function.main.connection_function_arn"]});
    assert!(!verified(
        &v,
        "aws_cloudfront_distribution.main",
        "connection_function_association.id"
    ));
    let mut v = input();
    v["resource_changes"][3]["change"]["after"]["connection_function_config"][0]["key_value_store_association"] =
        json!([]);
    assert!(!verified(
        &v,
        "aws_cloudfront_connection_function.main",
        super::NESTED
    ));
    let mut v = input();
    v["configuration"]["root_module"]["resources"][4]["expressions"]["key_value_store_arn"] =
        json!({"references":["aws_cloudfrontkeyvaluestore_key.main.id"]});
    assert!(!verified(
        &v,
        "aws_cloudfrontkeyvaluestore_key.main",
        "key_value_store_arn"
    ));
}

#[test]
fn helper_accounts_lifecycle_filtering_and_payloads_remain_independent() {
    let mut v = input();
    v["configuration"]["provider_config"]["aws"]["expressions"]["assume_role"] =
        json!([{"role_arn":{"constant_value":"arn:aws:iam::111111111111:role/Fixture"}}]);
    let parsed = plan::evidence::tests::parse_fixture(&v.to_string()).unwrap();
    let g = semantic::transform(&parsed);
    for n in g.nodes.iter().filter(|n| super::helper(&n.resource_type)) {
        assert!(n.member_owner.is_some());
        assert!(n.entity.scope.as_ref().unwrap().global);
        assert!(n.entity.scope.as_ref().unwrap().account.is_some());
        let selected = crate::view::single(
            &g,
            &crate::view::Options {
                focus: Some(n.address.clone()),
                depth: Some(0),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(selected.coverage.selected.resources, 1);
    }
    let mut incoming = plan::parse(FIXTURE).unwrap();
    incoming
        .nodes
        .iter_mut()
        .find(|n| n.resource_type == super::FUNCTION)
        .unwrap()
        .provider_configuration
        .as_mut()
        .unwrap()
        .account = crate::model::AwsAccountScope::new("222222222222");
    let raw = semantic::Input::new(&incoming);
    assert!(!raw.attributes.iter().any(|r| r.attribute
        == "default_cache_behavior.function_association.function_arn"
        && super::super::service_references::verified(&raw, r)));
    let mut parsed = parsed;
    parsed
        .nodes
        .iter_mut()
        .find(|n| n.address == "aws_cloudfront_key_value_store.first")
        .unwrap()
        .provider_configuration
        .as_mut()
        .unwrap()
        .account = crate::model::AwsAccountScope::new("222222222222");
    let raw = semantic::Input::new(&parsed);
    assert!(
        !raw.attributes
            .iter()
            .any(|r| r.attribute == "key_value_store_arn"
                && super::super::service_references::verified(&raw, r))
    );
    v["resource_changes"][5]["change"]["actions"] = json!(["delete"]);
    assert!(!verified(
        &v,
        "aws_cloudfrontkeyvaluestore_keys_exclusive.main",
        "key_value_store_arn"
    ));
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
