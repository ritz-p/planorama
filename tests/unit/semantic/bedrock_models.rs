use serde_json::{Value, json};
const FIXTURE: &str = include_str!("../../fixtures/bedrock-models-plan.json");
fn fixture() -> Value {
    serde_json::from_str(FIXTURE).unwrap()
}
fn index(v: &Value, address: &str) -> usize {
    v["resource_changes"]
        .as_array()
        .unwrap()
        .iter()
        .position(|n| n["address"] == address)
        .unwrap()
}
fn graph(v: &Value) -> crate::model::ArchitectureGraph {
    crate::semantic::transform(&crate::plan::parse(&v.to_string()).unwrap())
}
fn proven(v: &Value, address: &str, path: &str) -> bool {
    graph(v)
        .nodes
        .iter()
        .find(|n| n.address == address)
        .is_some_and(|n| {
            n.references.iter().any(|r| {
                r.attribute == path
                    && Some(r.label) == super::label(&n.resource_type, path)
                    && r.complete
                    && !r.dependency
                    && !r.targets.is_empty()
            })
        })
}
#[test]
fn bedrock_model_job_and_version_contracts_preserve_independent_identities() {
    let v = fixture();
    let raw = crate::plan::parse(FIXTURE).unwrap();
    let g = graph(&v);
    let mut checked = 0;
    for r in &raw.attributes {
        let node = &raw.nodes[r.target];
        if super::label(&node.resource_type, &r.attribute).is_some() {
            assert!(
                proven(&v, &node.address, &r.attribute),
                "{} {}: {r:?}",
                node.address,
                r.attribute
            );
            checked += 1;
        }
    }
    assert!(checked >= 25);
    assert_eq!(g.nodes.len(), raw.nodes.len());
    assert_eq!(g.coverage.selected.resources, raw.nodes.len());
    for n in &raw.nodes {
        let nodes: Vec<_> = g
            .nodes
            .iter()
            .filter(|a| a.address == n.address && a.deposed_key == n.deposed_key)
            .collect();
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].action, n.action);
    }
    for n in &g.nodes {
        if matches!(
            n.resource_type.as_str(),
            super::CUSTOM | super::BATCH | super::PROFILE | super::THROUGHPUT
        ) {
            assert_eq!(n.placement, crate::model::Placement::Logical);
        }
    }
    for (i, n) in g.nodes.iter().enumerate() {
        if matches!(n.resource_type.as_str(), super::CUSTOM | super::BATCH) {
            assert!(
                !g.edges
                    .iter()
                    .any(|e| e.kind == crate::model::EdgeKind::Containment && e.to == i)
            );
        }
    }
}
#[test]
fn bedrock_references_reject_unknown_incomplete_foreign_and_lifecycle_mismatches() {
    let base = fixture();
    let raw = crate::plan::parse(FIXTURE).unwrap();
    for r in &raw.attributes {
        let n = &raw.nodes[r.target];
        if super::label(&n.resource_type, &r.attribute).is_none() {
            continue;
        }
        let source = r.sources[0];
        let field = r.source_attributes_by_instance[&source].first().unwrap();
        let si = index(&base, &raw.nodes[source].address);
        let ti = index(&base, &n.address);
        for case in 0..11 {
            let mut v = base.clone();
            match case {
                0 => v["resource_changes"][si]["change"]["after"][field] = json!("other"),
                1 => v["resource_changes"][si]["change"]["after_unknown"] = json!({field:true}),
                2 => v["resource_changes"][si]["change"]["after_sensitive"] = json!({field:true}),
                3 => v["resource_changes"][si]["mode"] = json!("data"),
                4 => v["resource_changes"][si]["deposed"] = json!("old"),
                5 => v["resource_changes"][si]["change"]["actions"] = json!(["delete"]),
                6 => v["resource_changes"][si]["type"] = json!("aws_sqs_queue"),
                7 => {
                    v["resource_changes"][si]["provider_name"] =
                        json!("registry.terraform.io/hashicorp/google")
                }
                8 => v["configuration"]["root_module"]["resources"][ti]["expressions"] = json!({}),
                9 => v["resource_changes"][ti]["change"]["after_unknown"] = json!(true),
                _ => {
                    v["configuration"]["provider_config"]["aws.other"] = json!({"full_name":"registry.terraform.io/hashicorp/aws","expressions":{"region":{"constant_value":"us-west-2"}}});
                    v["configuration"]["root_module"]["resources"][si]["provider_config_key"] =
                        json!("aws.other");
                }
            }
            assert!(
                !proven(&v, &n.address, &r.attribute),
                "{} {} case {case}",
                n.address,
                r.attribute
            );
        }
    }
}
#[test]
fn bedrock_guardrails_require_the_correct_published_arn_version_pair() {
    for case in 0..7 {
        let mut v = fixture();
        let t = index(&v, "aws_bedrock_enforced_guardrail_configuration.main");
        let version = index(&v, "aws_bedrock_guardrail_version.main");
        match case {
            0 => v["resource_changes"][t]["change"]["after"]["guardrail_version"] = json!("DRAFT"),
            1 => v["resource_changes"][t]["change"]["after"]["guardrail_version"] = json!("0"),
            2 => {
                v["resource_changes"][t]["change"]["after_unknown"] =
                    json!({"guardrail_version":true})
            }
            3 => {
                let other = index(&v, "aws_bedrock_guardrail.other");
                v["resource_changes"][t]["change"]["after"]["guardrail_arn"] =
                    v["resource_changes"][other]["change"]["after"]["guardrail_arn"].clone();
                v["configuration"]["root_module"]["resources"][t]["expressions"]["guardrail_arn"] =
                    json!({"references":["aws_bedrock_guardrail.other.guardrail_arn"]});
            }
            4 => {
                v["configuration"]["root_module"]["resources"][t]["expressions"]["guardrail_version"] =
                    json!({"constant_value":"1"})
            }
            5 => v["configuration"]["root_module"]["resources"][version]["expressions"] = json!({}),
            _ => v["resource_changes"][version]["change"]["after"]["version"] = json!("2"),
        }
        assert!(
            !proven(
                &v,
                "aws_bedrock_enforced_guardrail_configuration.main",
                "guardrail_arn"
            ),
            "case {case}"
        );
        assert!(
            !proven(
                &v,
                "aws_bedrock_enforced_guardrail_configuration.main",
                "guardrail_version"
            ),
            "case {case}"
        );
    }
    let g = graph(&fixture());
    for kind in [super::VERSION, super::ENFORCED] {
        assert!(
            g.nodes
                .iter()
                .filter(|n| n.resource_type == kind)
                .all(|n| n.member_owner.is_some())
        );
    }
}

#[test]
fn bedrock_job_network_context_requires_both_complete_sets_in_one_explicit_vpc() {
    for address in [
        "aws_bedrock_custom_model.derived",
        "aws_bedrock_model_invocation_job.main",
    ] {
        for case in 0..8 {
            let mut v = fixture();
            let t = index(&v, address);
            let subnet = index(&v, "aws_subnet.first");
            let group = index(&v, "aws_security_group.data");
            let vpc = index(&v, "aws_vpc.data");
            match case {
                0 => {
                    v["resource_changes"][t]["change"]["after"]["vpc_config"][0]["security_group_ids"] =
                        json!([])
                }
                1 => {
                    v["resource_changes"][t]["change"]["after_unknown"] =
                        json!({"vpc_config":[{"subnet_ids":[false,true]}]})
                }
                2 => v["resource_changes"][group]["change"]["after"]["vpc_id"] = json!("different"),
                3 => {
                    v["configuration"]["root_module"]["resources"][subnet]["expressions"] =
                        json!({})
                }
                4 => v["resource_changes"][vpc]["change"]["after_unknown"] = json!({"id":true}),
                5 => {
                    v["resource_changes"][t]["change"]["after"]["vpc_config"][0]["subnet_ids"] =
                        json!(["subnet-0", "subnet-0"])
                }
                6 => {
                    v["configuration"]["root_module"]["resources"][t]["expressions"]["vpc_config"]
                        [0]["subnet_ids"] = json!({"references":["aws_subnet.first.id"]})
                }
                _ => {
                    v["resource_changes"][t]["change"]["after_sensitive"] =
                        json!({"vpc_config":true})
                }
            }
            assert!(
                !proven(&v, address, super::SUBNETS),
                "{address} case {case}"
            );
            assert!(!proven(&v, address, super::GROUPS), "{address} case {case}");
            assert!(proven(&v, address, "role_arn"));
        }
    }
}
#[test]
fn evaluation_choices_are_exclusive_known_and_payload_independent() {
    let human = "aws_bedrock_evaluation_job.human";
    for case in 0..4 {
        let mut v = fixture();
        let t = index(&v, human);
        match case {
            0 => {
                v["resource_changes"][t]["change"]["after"]["evaluation_config"][0]["automated"] =
                    json!([{}])
            }
            1 => {
                v["resource_changes"][t]["change"]["after_unknown"] =
                    json!({"evaluation_config":[{"automated":true}]})
            }
            2 => {
                v["resource_changes"][t]["change"]["after"]["evaluation_config"][0]["human"] =
                    json!([])
            }
            _ => {
                v["resource_changes"][t]["change"]["after_unknown"] =
                    json!({"inference_config":true})
            }
        }
        assert!(!proven(&v, human, super::FLOW), "case {case}");
        assert!(!proven(&v, human, "role_arn"));
    }
    let rag = "aws_bedrock_evaluation_job.rag";
    for case in 0..5 {
        let mut v = fixture();
        let t = index(&v, rag);
        match case {
            0 => {
                v["resource_changes"][t]["change"]["after"]["inference_config"][0]["model"] =
                    json!([{}])
            }
            1 => {
                v["resource_changes"][t]["change"]["after"]["inference_config"][0]["rag_config"]
                    [0]["precomputed_rag_source_config"] = json!([{}])
            }
            2 => {
                v["resource_changes"][t]["change"]["after"]["inference_config"][0]["rag_config"]
                    [0]["knowledge_base_config"][0]["retrieve_and_generate_config"] = json!([{}])
            }
            3 => {
                v["resource_changes"][t]["change"]["after_unknown"] =
                    json!({"inference_config":[{"rag_config":[{"knowledge_base_config":true}]}]})
            }
            _ => {
                v["configuration"]["root_module"]["resources"][t]["expressions"]["inference_config"] =
                    json!([])
            }
        }
        assert!(!proven(&v, rag, super::RETRIEVE), "case {case}");
    }
    let mut v = fixture();
    let t = index(&v, human);
    v["resource_changes"][t]["change"]["after_sensitive"] = json!({"evaluation_config":[{"human":[{"human_workflow_config":[{"instructions":true}]}]}]});
    assert!(proven(&v, human, super::FLOW));
    assert!(proven(&v, human, "role_arn"));
}
#[test]
fn custom_model_outputs_are_distinct_and_service_generated_identities_are_not_nodes() {
    let mut v = fixture();
    let t = index(&v, "aws_bedrock_provisioned_model_throughput.main");
    let model = index(&v, "aws_bedrock_custom_model.derived");
    v["resource_changes"][t]["change"]["after"]["model_arn"] =
        v["resource_changes"][model]["change"]["after"]["job_arn"].clone();
    v["configuration"]["root_module"]["resources"][t]["expressions"]["model_arn"] =
        json!({"references":["aws_bedrock_custom_model.derived.job_arn"]});
    assert!(!proven(
        &v,
        "aws_bedrock_provisioned_model_throughput.main",
        "model_arn"
    ));
    let mut v = fixture();
    let t = index(&v, "aws_bedrock_custom_model.derived");
    let base = index(&v, "aws_bedrock_custom_model.base");
    v["resource_changes"][t]["change"]["after"]["base_model_identifier"] =
        v["resource_changes"][base]["change"]["after"]["job_arn"].clone();
    v["configuration"]["root_module"]["resources"][t]["expressions"]["base_model_identifier"] =
        json!({"references":["aws_bedrock_custom_model.base.job_arn"]});
    assert!(!proven(
        &v,
        "aws_bedrock_custom_model.derived",
        "base_model_identifier"
    ));
    let v = fixture();
    let g = graph(&v);
    for n in &g.nodes {
        assert!(!n.address.contains("foundation-model"));
        assert!(!n.address.contains("PRIVATE_"));
    }
    let selected = crate::view::single(
        &g,
        &crate::view::Options {
            focus: Some("aws_bedrock_custom_model.derived".into()),
            depth: Some(0),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(selected.coverage.selected.resources, 1);
    let mut missing = fixture();
    let i = index(&missing, "aws_bedrock_custom_model.base");
    missing["resource_changes"]
        .as_array_mut()
        .unwrap()
        .remove(i);
    for format in [crate::OutputFormat::Svg, crate::OutputFormat::Html] {
        let output = crate::generate_named(
            &[
                crate::NamedInput {
                    state: "first".into(),
                    json: FIXTURE.into(),
                },
                crate::NamedInput {
                    state: "second".into(),
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
        assert!(!output.contains("PRIVATE_"));
        assert!(!output.contains("<metadata id=\"cross-state-relationships\">"));
    }
}
