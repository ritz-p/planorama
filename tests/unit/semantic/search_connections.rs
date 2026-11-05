use crate::{plan, semantic};
use serde_json::{Value, json};

const FIXTURE: &str = include_str!("../../fixtures/search-connections-plan.json");

fn input() -> Value {
    serde_json::from_str(FIXTURE).unwrap()
}
fn parse(v: &Value) -> crate::model::TerraformPlan {
    plan::evidence::tests::parse_fixture(&v.to_string()).unwrap()
}
fn verified(v: &Value, path: &str) -> bool {
    let p = parse(v);
    let raw = semantic::Input::new(&p);
    raw.attributes
        .iter()
        .any(|r| r.attribute == path && super::super::service_references::verified(&raw, r))
}

#[test]
fn endpoints_and_accepter_require_independent_account_and_region_proof() {
    let mut v = input();
    for mode in ["DIRECT", "VPC_ENDPOINT"] {
        v["resource_changes"][2]["change"]["after"]["connection_mode"] = mode.into();
        for field in [super::LOCAL, super::REMOTE, "connection_id"] {
            assert!(verified(&v, field), "{field}");
        }
    }
    let graph = semantic::transform(&parse(&v));
    assert_eq!(graph.coverage.selected.resources, 4);
    assert!(
        graph
            .nodes
            .iter()
            .any(|n| n.resource_type == super::INBOUND)
    );
    assert!(
        graph
            .nodes
            .iter()
            .any(|n| n.resource_type == super::OUTBOUND)
    );
    assert!(
        !graph
            .edges
            .iter()
            .any(|e| e.kind == crate::model::EdgeKind::Containment)
    );
    let raw_plan = plan::parse(FIXTURE).unwrap();
    let raw = semantic::Input::new(&raw_plan);
    assert!(
        raw.attributes
            .iter()
            .filter(|r| super::label(&raw.nodes[r.target].resource_type, &r.attribute).is_some())
            .all(|r| !super::super::service_references::verified(&raw, r))
    );
}

#[test]
fn unproven_selectors_values_lifecycle_and_shapes_remain_unresolved() {
    for case in 0..22 {
        let mut v = input();
        match case {
            0 => {
                v["resource_changes"][2]["change"]["after"]["remote_domain_info"][0]["owner_id"] =
                    "111111111111".into()
            }
            1 => {
                v["resource_changes"][2]["change"]["after"]["remote_domain_info"][0]["region"] =
                    "us-east-1".into()
            }
            2 => {
                v["resource_changes"][2]["change"]["after"]["remote_domain_info"][0]["domain_name"] =
                    "wrong".into()
            }
            3 => {
                v["resource_changes"][2]["change"]["after_unknown"] =
                    json!({"remote_domain_info":[{"owner_id":true}]})
            }
            4 => {
                v["resource_changes"][2]["change"]["after_sensitive"] =
                    json!({"remote_domain_info":[{"region":true}]})
            }
            5 => v["resource_changes"][1]["change"]["after_unknown"] = json!({"domain_name":true}),
            6 => {
                v["configuration"]["root_module"]["resources"][2]["expressions"]["remote_domain_info"] =
                    json!([{"domain_name":{"constant_value":"remote"}}])
            }
            7 => {
                v["configuration"]["root_module"]["resources"][2]["expressions"]["remote_domain_info"] = json!([{"domain_name":{"references":["aws_opensearch_domain.remote.domain_name","var.unresolved"]}}])
            }
            8 => {
                v["configuration"]["root_module"]["resources"][2]["expressions"]["remote_domain_info"] =
                    json!([{"domain_name":{"references":["aws_opensearch_domain.remote.arn"]}}])
            }
            9 => v["resource_changes"][1]["change"]["actions"] = json!(["delete"]),
            10 => v["resource_changes"][1]["deposed"] = "old".into(),
            11 => v["resource_changes"][1]["mode"] = "data".into(),
            12 => v["resource_changes"][2]["change"]["after"]["connection_mode"] = "UNKNOWN".into(),
            13 => {
                v["resource_changes"][2]["change"]["after_unknown"] =
                    json!({"connection_mode":true})
            }
            14 => v["resource_changes"][2]["change"]["after"]["remote_domain_info"] = json!([]),
            15 => {
                v["resource_changes"][2]["change"]["after_unknown"] =
                    json!({"remote_domain_info":[{},{}]})
            }
            16 => {
                v["resource_changes"][2]["change"]["after"]["local_domain_info"][0]["owner_id"] =
                    "222222222222".into()
            }
            17 => {
                v["configuration"]["root_module"]["resources"][2]["provider_config_key"] =
                    "remote".into()
            }
            18 => v["resource_changes"][1]["provider_name"] = "example.com/custom/aws".into(),
            19 => {
                let row =
                    v["resource_changes"][2]["change"]["after"]["remote_domain_info"][0].clone();
                v["resource_changes"][2]["change"]["after"]["remote_domain_info"]
                    .as_array_mut()
                    .unwrap()
                    .push(row);
            }
            21 => {
                v["configuration"]["root_module"]["resources"][2]["expressions"]["remote_domain_info"] = json!([{"domain_name":{"references":["aws_opensearch_domain.remote.domain_name","aws_opensearch_domain.local.domain_name"]}}])
            }
            _ => {
                let text = v.to_string().replace(
                    "aws_opensearch_domain.remote",
                    "aws_elasticsearch_domain.remote",
                );
                v = serde_json::from_str(&text).unwrap();
                v["resource_changes"][1]["type"] = "aws_elasticsearch_domain".into();
                v["configuration"]["root_module"]["resources"][1]["type"] =
                    "aws_elasticsearch_domain".into();
            }
        }
        assert!(!verified(&v, super::REMOTE), "remote case {case}");
        assert!(!verified(&v, "connection_id"), "accepter case {case}");
    }
}

#[test]
fn accepter_orientation_id_and_lifecycle_do_not_alias_the_outbound() {
    for case in 0..9 {
        let mut v = input();
        match case {
            0 => {
                v["configuration"]["root_module"]["resources"][3]["provider_config_key"] =
                    "local".into()
            }
            1 => v["resource_changes"][3]["change"]["after"]["connection_id"] = "wrong".into(),
            2 => v["resource_changes"][2]["change"]["after_unknown"] = json!({"id":true}),
            3 => {
                v["resource_changes"][3]["change"]["after_unknown"] = json!({"connection_id":true})
            }
            4 => v["resource_changes"][3]["change"]["actions"] = json!(["delete"]),
            5 => v["resource_changes"][3]["deposed"] = "old".into(),
            6 => v["resource_changes"][2]["change"]["actions"] = json!(["delete"]),
            7 => {
                v["configuration"]["root_module"]["resources"][3]["expressions"]["connection_id"] =
                    json!({"constant_value":"connection-example"})
            }
            _ => {
                v["configuration"]["root_module"]["resources"][3]["expressions"]["connection_id"] = json!({"references":["aws_opensearch_outbound_connection.main.id","var.unresolved"]})
            }
        }
        assert!(!verified(&v, "connection_id"), "case {case}");
    }
    for index in 0..4 {
        let mut p = parse(&input());
        p.nodes[index]
            .provider_configuration
            .as_mut()
            .unwrap()
            .account_conflict = true;
        let raw = semantic::Input::new(&p);
        assert!(
            !raw.attributes.iter().any(|r| r.attribute == "connection_id"
                && super::super::service_references::verified(&raw, r))
        );
    }
}

#[test]
fn filtering_and_named_states_keep_coverage_and_hide_runtime_payloads() {
    let parsed = parse(&input());
    let graph = semantic::transform(&parsed);
    let graph = crate::view::single(
        &graph,
        &crate::view::Options {
            focus: Some("aws_opensearch_inbound_connection_accepter.main".into()),
            depth: Some(0),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(graph.coverage.selected.resources, 1);
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
