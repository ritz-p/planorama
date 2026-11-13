use crate::{
    model::{EdgeKind, Placement},
    plan, semantic,
};
use serde_json::{Value, json};
const FIXTURE: &str = include_str!("../../fixtures/agentcore-services-plan.json");
fn fixture() -> Value {
    serde_json::from_str(FIXTURE).unwrap()
}
fn graph(v: &Value) -> crate::model::ArchitectureGraph {
    semantic::transform(&plan::evidence::tests::parse_fixture(&v.to_string()).unwrap())
}
fn node<'a>(g: &'a crate::model::ArchitectureGraph, v: &Value, t: usize) -> &'a crate::model::Node {
    g.nodes
        .iter()
        .find(|n| n.address == v["resource_changes"][t]["address"])
        .unwrap()
}
fn verified(v: &Value, t: usize, p: &str) -> bool {
    node(&graph(v), v, t).references.iter().any(|r| {
        r.attribute == p
            && r.complete
            && Some(r.label) == super::label(v["resource_changes"][t]["type"].as_str().unwrap(), p)
    })
}
fn set(v: &mut Value, p: &str, x: Value) {
    *v.pointer_mut(&format!("/{}", p.replace('.', "/"))).unwrap() = x;
}
const LINKS: &[(usize, &str, &str, usize, &str)] = &[
    (12, "kms_key_arn", "kms_key_arn", 2, "arn"),
    (
        12,
        "evaluator_config.code_based.lambda_config.lambda_arn",
        "evaluator_config.0.code_based.0.lambda_config.0.lambda_arn",
        3,
        "qualified_arn",
    ),
    (
        14,
        "memory_execution_role_arn",
        "memory_execution_role_arn",
        0,
        "arn",
    ),
    (14, "encryption_key_arn", "encryption_key_arn", 2, "arn"),
    (
        14,
        "stream_delivery_resources.resource.kinesis.data_stream_arn",
        "stream_delivery_resources.0.resource.0.kinesis.0.data_stream_arn",
        4,
        "arn",
    ),
    (15, "memory_id", "memory_id", 14, "id"),
    (
        15,
        "memory_execution_role_arn",
        "memory_execution_role_arn",
        1,
        "arn",
    ),
    (
        15,
        "configuration.self_managed_configuration.invocation_configuration.topic_arn",
        "configuration.0.self_managed_configuration.0.invocation_configuration.0.topic_arn",
        5,
        "arn",
    ),
    (
        15,
        "configuration.self_managed_configuration.invocation_configuration.payload_delivery_bucket_name",
        "configuration.0.self_managed_configuration.0.invocation_configuration.0.payload_delivery_bucket_name",
        6,
        "bucket",
    ),
    (
        17,
        "evaluation_execution_role_arn",
        "evaluation_execution_role_arn",
        0,
        "arn",
    ),
    (18, "encryption_key_arn", "encryption_key_arn", 2, "arn"),
    (
        19,
        "policy_engine_id",
        "policy_engine_id",
        18,
        "policy_engine_id",
    ),
    (20, "resource_arn", "resource_arn", 9, "agent_runtime_arn"),
    (21, "resource_arn", "resource_arn", 11, "gateway_arn"),
    (
        23,
        "kms_configuration.kms_key_arn",
        "kms_configuration.0.kms_key_arn",
        2,
        "arn",
    ),
];
#[test]
fn agentcore_services_typed_links_and_logical_owners_remain_independent() {
    let v = fixture();
    for &(t, p, _, _, _) in LINKS {
        assert!(verified(&v, t, p), "{t} {p}");
    }
    assert!(verified(&v, 17, "evaluator.evaluator_id"));
    assert!(verified(
        &v,
        17,
        "data_source_config.cloudwatch_logs.log_group_names"
    ));
    assert!(!verified(&v, 22, "resource_arn"));
    assert!(!verified(&v, 24, "kms_configuration.kms_key_arn"));
    let g = graph(&v);
    assert_eq!(g.nodes.len(), 26);
    assert_eq!(g.coverage.selected.resources, 26);
    for t in 12..26 {
        let n = node(&g, &v, t);
        assert_eq!(n.placement, Placement::Logical);
        assert_eq!(
            n.member_owner.is_some(),
            matches!(t, 15 | 16 | 19 | 20 | 21),
            "{t}"
        );
        assert!(
            n.references
                .iter()
                .all(|r| !super::indexed(&n.resource_type, &r.attribute))
        );
    }
    assert!(
        !g.edges
            .iter()
            .any(|e| e.kind == EdgeKind::Containment
                && super::resource(&g.nodes[e.to].resource_type))
    );
}
#[test]
fn agentcore_services_endpoints_require_explicit_exact_known_current_outputs() {
    for &(t, p, path, s, out) in LINKS {
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
            "source-unknown",
            "consumer-delete",
            "array",
        ] {
            let mut v = fixture();
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
                        .to_owned();
                    set(
                        &mut v,
                        &format!("configuration.root_module.resources.{t}.expressions.{path}"),
                        json!({"references":[format!("{a}.wrong")]}),
                    );
                }
                "mismatch" => v["resource_changes"][s]["change"]["after"][out] = json!("different"),
                "data" => v["resource_changes"][s]["mode"] = json!("data"),
                "delete" => v["resource_changes"][s]["change"]["actions"] = json!(["delete"]),
                "deposed" => v["resource_changes"][s]["deposed"] = json!("old"),
                "provider" => {
                    v["resource_changes"][s]["provider_name"] =
                        json!("registry.terraform.io/hashicorp/google")
                }
                "source-unknown" => {
                    v["resource_changes"][s]["change"]["after_unknown"] = json!({out:true})
                }
                "consumer-delete" => {
                    v["resource_changes"][t]["change"]["actions"] = json!(["delete"])
                }
                "array" => {
                    let x = v
                        .pointer(&format!(
                            "/resource_changes/{t}/change/after/{}",
                            path.replace('.', "/")
                        ))
                        .unwrap()
                        .clone();
                    set(
                        &mut v,
                        &format!("resource_changes.{t}.change.after.{path}"),
                        json!([x]),
                    );
                }
                _ => unreachable!(),
            }
            assert!(!verified(&v, t, p), "{t} {p} {case}");
        }
    }
}
#[test]
fn agentcore_services_mixed_evaluator_set_is_complete_and_order_independent() {
    let p = "evaluator.evaluator_id";
    let mut v = fixture();
    v["resource_changes"][17]["change"]["after"]["evaluator"]
        .as_array_mut()
        .unwrap()
        .reverse();
    assert!(verified(&v, 17, p));
    for case in [
        "duplicate",
        "literal-custom",
        "unknown-builtin",
        "sensitive-builtin",
        "extra-literal",
        "wrong-output",
        "wrong-type",
        "builtin-output",
        "unknown-source",
        "extra-source",
        "missing-row",
        "empty",
        "too-many",
        "array-id",
        "foreign-provider",
        "deleted",
        "unknown-block",
        "mask-length",
    ] {
        let mut v = fixture();
        match case {
            "duplicate" => {
                v["resource_changes"][17]["change"]["after"]["evaluator"][2] =
                    v["resource_changes"][17]["change"]["after"]["evaluator"][0].clone()
            }
            "literal-custom" => {
                v["configuration"]["root_module"]["resources"][17]["expressions"]["evaluator"][0]
                    ["evaluator_id"] = json!({"constant_value":"Code-0123456789"})
            }
            "unknown-builtin" | "sensitive-builtin" => {
                v["resource_changes"][17]["change"][if case == "unknown-builtin" {
                    "after_unknown"
                } else {
                    "after_sensitive"
                }] = json!({"evaluator":[{}, {"evaluator_id":true},{}]})
            }
            "extra-literal" => v["resource_changes"][17]["change"]["after"]["evaluator"]
                .as_array_mut()
                .unwrap()
                .push(json!({"evaluator_id":"Builtin.Safety"})),
            "wrong-output" => {
                v["configuration"]["root_module"]["resources"][17]["expressions"]["evaluator"][0]
                    ["evaluator_id"] =
                    json!({"references":["aws_bedrockagentcore_evaluator.code.evaluator_arn"]})
            }
            "wrong-type" => {
                v["resource_changes"][12]["type"] = json!("aws_bedrockagentcore_memory")
            }
            "builtin-output" => {
                v["resource_changes"][12]["change"]["after"]["evaluator_id"] =
                    json!("Builtin.Safety");
                v["resource_changes"][17]["change"]["after"]["evaluator"][0]["evaluator_id"] =
                    json!("Builtin.Safety");
            }
            "unknown-source" => {
                v["resource_changes"][12]["change"]["after_unknown"] = json!({"evaluator_id":true})
            }
            "extra-source" => {
                v["configuration"]["root_module"]["resources"][17]["expressions"]["evaluator"][0]
                    ["evaluator_id"] = json!({"references":["aws_bedrockagentcore_evaluator.code.evaluator_id","aws_bedrockagentcore_evaluator.judge.evaluator_id"]})
            }
            "missing-row" => {
                v["configuration"]["root_module"]["resources"][17]["expressions"]["evaluator"]
                    .as_array_mut()
                    .unwrap()
                    .pop();
            }
            "empty" => v["resource_changes"][17]["change"]["after"]["evaluator"] = json!([]),
            "too-many" => {
                v["resource_changes"][17]["change"]["after"]["evaluator"] = json!(
                    (0..11)
                        .map(|i| json!({"evaluator_id":format!("Builtin.B{i}")}))
                        .collect::<Vec<_>>()
                )
            }
            "array-id" => {
                v["resource_changes"][17]["change"]["after"]["evaluator"][0]["evaluator_id"] =
                    json!(["Code-0123456789"])
            }
            "foreign-provider" => {
                v["resource_changes"][12]["provider_name"] =
                    json!("registry.terraform.io/hashicorp/google")
            }
            "deleted" => v["resource_changes"][12]["change"]["actions"] = json!(["delete"]),
            "unknown-block" => {
                v["resource_changes"][17]["change"]["after_unknown"] = json!({"evaluator":true})
            }
            "mask-length" => {
                v["resource_changes"][17]["change"]["after_unknown"] = json!({"evaluator":[{}]})
            }
            _ => unreachable!(),
        }
        assert!(!verified(&v, 17, p), "{case}");
    }
    assert!(!verified(&fixture(), 25, p));
}
#[test]
fn agentcore_services_discriminators_shapes_and_typed_policy_targets_are_bounded() {
    for case in [
        "llm-also",
        "llm-unknown",
        "code-twice",
        "stream-twice",
        "strategy-type",
        "strategy-config-type",
        "strategy-also",
        "strategy-parent",
        "vault-unknown",
        "vault-service",
        "policy-memory",
        "policy-endpoint",
        "log-partial",
        "log-duplicate",
        "log-unknown",
    ] {
        let mut v = fixture();
        let (t, p) = match case {
            "llm-also" => {
                v["resource_changes"][12]["change"]["after"]["evaluator_config"][0]["llm_as_a_judge"] =
                    json!([{}]);
                (12, LINKS[1].1)
            }
            "llm-unknown" => {
                v["resource_changes"][12]["change"]["after_unknown"] =
                    json!({"evaluator_config":[{"llm_as_a_judge":true}]});
                (12, LINKS[1].1)
            }
            "code-twice" => {
                let x = v["resource_changes"][12]["change"]["after"]["evaluator_config"][0].clone();
                v["resource_changes"][12]["change"]["after"]["evaluator_config"]
                    .as_array_mut()
                    .unwrap()
                    .push(x);
                (12, LINKS[1].1)
            }
            "stream-twice" => {
                let x = v["resource_changes"][14]["change"]["after"]["stream_delivery_resources"]
                    [0]
                .clone();
                v["resource_changes"][14]["change"]["after"]["stream_delivery_resources"]
                    .as_array_mut()
                    .unwrap()
                    .push(x);
                (14, LINKS[4].1)
            }
            "strategy-type" => {
                v["resource_changes"][15]["change"]["after"]["type"] = json!("SEMANTIC");
                (15, LINKS[7].1)
            }
            "strategy-config-type" => {
                v["resource_changes"][15]["change"]["after"]["configuration"][0]["type"] =
                    json!("SEMANTIC_OVERRIDE");
                (15, LINKS[7].1)
            }
            "strategy-also" => {
                v["resource_changes"][15]["change"]["after"]["configuration"][0]["extraction"] =
                    json!([{}]);
                (15, LINKS[7].1)
            }
            "strategy-parent" => {
                v["resource_changes"][14]["change"]["after_unknown"] = json!({"id":true});
                (15, LINKS[7].1)
            }
            "vault-unknown" => {
                v["resource_changes"][23]["change"]["after_unknown"] =
                    json!({"kms_configuration":[{"key_type":true}]});
                (23, LINKS[14].1)
            }
            "vault-service" => {
                v["resource_changes"][23]["change"]["after"]["kms_configuration"][0]["key_type"] =
                    json!("ServiceManagedKey");
                (23, LINKS[14].1)
            }
            "policy-memory" | "policy-endpoint" => {
                let (s, out) = if case == "policy-memory" {
                    (14, "arn")
                } else {
                    (10, "agent_runtime_arn")
                };
                v["resource_changes"][20]["change"]["after"]["resource_arn"] =
                    v["resource_changes"][s]["change"]["after"][out].clone();
                v["configuration"]["root_module"]["resources"][20]["expressions"]["resource_arn"] = json!({"references":[format!("{}.{}",v["resource_changes"][s]["address"].as_str().unwrap(),out)]});
                (20, "resource_arn")
            }
            "log-partial" => {
                v["configuration"]["root_module"]["resources"][17]["expressions"]["data_source_config"]
                    [0]["cloudwatch_logs"][0]["log_group_names"] =
                    json!({"references":["aws_cloudwatch_log_group.one.name"]});
                (17, "data_source_config.cloudwatch_logs.log_group_names")
            }
            "log-duplicate" => {
                v["resource_changes"][17]["change"]["after"]["data_source_config"][0]["cloudwatch_logs"]
                    [0]["log_group_names"] = json!(["/aws/agentcore/one", "/aws/agentcore/one"]);
                (17, "data_source_config.cloudwatch_logs.log_group_names")
            }
            "log-unknown" => {
                v["resource_changes"][17]["change"]["after_unknown"] =
                    json!({"data_source_config":true});
                (17, "data_source_config.cloudwatch_logs.log_group_names")
            }
            _ => unreachable!(),
        };
        assert!(!verified(&v, t, p), "{case}");
    }
    let mut v = fixture();
    v["resource_changes"][12]["change"]["after"]["evaluator_config"][0]["code_based"][0]["opaque"] =
        json!("PRIVATE_PAYLOAD");
    v["resource_changes"][12]["change"]["after_sensitive"] =
        json!({"evaluator_config":[{"code_based":[{"opaque":true}]}]});
    assert!(verified(&v, 12, LINKS[1].1));
}
#[test]
fn agentcore_services_independent_accounts_regions_and_global_iam_identity_are_required() {
    for (s, t, p, global) in [
        (0, 14, "memory_execution_role_arn", true),
        (1, 15, "memory_execution_role_arn", true),
        (2, 18, "encryption_key_arn", false),
        (12, 17, "evaluator.evaluator_id", false),
        (14, 15, "memory_id", false),
        (9, 20, "resource_arn", false),
        (
            14,
            15,
            "configuration.self_managed_configuration.invocation_configuration.topic_arn",
            false,
        ),
    ] {
        for axis in ["account", "region"] {
            let mut v = fixture();
            let mut alias = v["configuration"]["provider_config"]["aws"].clone();
            alias["alias"] = json!("foreign");
            if axis == "account" {
                alias["expressions"]["assume_role"][0]["role_arn"]["constant_value"] =
                    json!("arn:aws:iam::999999999999:role/Test");
            } else {
                alias["expressions"]["region"]["constant_value"] = json!("us-west-2");
            }
            v["configuration"]["provider_config"]["aws.foreign"] = alias;
            v["configuration"]["root_module"]["resources"][s]["provider_config_key"] =
                json!("aws.foreign");
            assert_eq!(verified(&v, t, p), global && axis == "region", "{s} {axis}");
        }
    }
}
#[test]
fn agentcore_services_lifecycle_data_filters_named_states_and_personal_data_remain_separate() {
    for t in 12..=25 {
        let mut v = fixture();
        v["resource_changes"][t]["change"]["actions"] = json!(["delete", "create"]);
        v["resource_changes"][t]["change"]["importing"] = json!({"id":"domain,user"});
        v["resource_changes"][t]["previous_address"] =
            json!("aws_bedrockagentcore_memory_strategy.old");
        assert_eq!(graph(&v).coverage.selected.resources, 26);
    }
    let mut v = fixture();
    v["resource_changes"][14]["mode"] = json!("data");
    assert!(node(&graph(&v), &v, 15).member_owner.is_none());
    assert!(!verified(&v, 15, "memory_id"));
    let v = fixture();
    let g = graph(&v);
    let selected = crate::view::single(
        &g,
        &crate::view::Options {
            focus: Some("aws_bedrockagentcore_memory_strategy.self".into()),
            depth: Some(0),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(selected.coverage.selected.resources, 1);
    assert!(selected.edges.is_empty());
    assert!(
        !selected
            .nodes
            .iter()
            .any(|n| n.resource_type == super::MEMORY)
    );
    let inputs = plan::parse_inputs(
        ["a", "b"]
            .into_iter()
            .map(|s| crate::model::PlanInput {
                state_id: crate::model::StateId::new(s).unwrap(),
                plan_json: FIXTURE.into(),
            })
            .collect(),
    )
    .unwrap();
    assert!(
        semantic::cross_state::resolve(&inputs, &[])
            .edges
            .is_empty()
    );
    for format in [crate::OutputFormat::Svg, crate::OutputFormat::Html] {
        let o = crate::generate(
            FIXTURE,
            &crate::Options {
                format,
                ..Default::default()
            },
        )
        .unwrap();
        assert!(!o.content.contains("PRIVATE_"));
    }
}
#[test]
fn agentcore_services_evaluator_set_masks_are_checked_per_identity_leaf() {
    for mask in ["after_unknown", "after_sensitive"] {
        for (m, expected) in [
            (json!([{}, {}, {}]), true),
            (json!([{"evaluator_id":false}, {}, {}]), true),
            (json!([{"opaque":true}, {}, {}]), true),
            (json!([{"evaluator_id":true}, {}, {}]), false),
            (json!([{}, {"evaluator_id":true}, {}]), false),
            (json!([{}, {}, {"evaluator_id":true}]), false),
            (json!([{}, true, {}]), false),
            (json!([{}]), false),
        ] {
            let mut v = fixture();
            v["resource_changes"][17]["change"][mask] = json!({"evaluator":m});
            assert_eq!(
                verified(&v, 17, "evaluator.evaluator_id"),
                expected,
                "{mask} {m}"
            );
        }
    }
}
