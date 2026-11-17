use serde_json::{Value, json};
const FIXTURE: &str = include_str!("../../fixtures/bedrock-agents-plan.json");
fn fixture() -> Value {
    serde_json::from_str(FIXTURE).unwrap()
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
fn bedrock_agents_preserve_configured_helpers_and_shared_endpoints() {
    let v = fixture();
    let raw = crate::plan::parse(FIXTURE).unwrap();
    let g = graph(&v);
    let mut checked = 0;
    for r in &raw.attributes {
        let n = &raw.nodes[r.target];
        if super::label(&n.resource_type, &r.attribute).is_some() {
            assert!(
                proven(&v, &n.address, &r.attribute),
                "{} {} {r:?}",
                n.address,
                r.attribute
            );
            checked += 1;
        }
    }
    assert!(checked >= 25, "{checked}");
    assert_eq!(g.nodes.len(), raw.nodes.len());
    assert_eq!(g.coverage.selected.resources, raw.nodes.len());
    for n in &raw.nodes {
        let nodes: Vec<_> = g.nodes.iter().filter(|a| a.address == n.address).collect();
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].action, n.action);
    }
    for (i, n) in g.nodes.iter().enumerate() {
        if super::resource(&n.resource_type) {
            assert_eq!(n.placement, crate::model::Placement::Logical);
            assert!(
                !g.edges
                    .iter()
                    .any(|e| e.kind == crate::model::EdgeKind::Containment && e.to == i)
            );
        }
    }
}
#[test]
fn bedrock_agent_links_reject_incompatible_missing_and_lifecycle_evidence() {
    let base = fixture();
    let raw = crate::plan::parse(FIXTURE).unwrap();
    for r in &raw.attributes {
        let n = &raw.nodes[r.target];
        if super::label(&n.resource_type, &r.attribute).is_none() {
            continue;
        }
        let si = base["resource_changes"]
            .as_array()
            .unwrap()
            .iter()
            .position(|v| v["address"] == raw.nodes[r.sources[0]].address)
            .unwrap();
        let ti = base["resource_changes"]
            .as_array()
            .unwrap()
            .iter()
            .position(|v| v["address"] == n.address)
            .unwrap();
        let field = r.source_attributes_by_instance[&r.sources[0]]
            .first()
            .unwrap();
        for case in 0..10 {
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
                _ => v["resource_changes"][ti]["change"]["after_unknown"] = json!(true),
            }
            assert!(
                !proven(&v, &n.address, &r.attribute),
                "{} {} case{case}",
                n.address,
                r.attribute
            );
        }
    }
}
fn index(v: &Value, address: &str) -> usize {
    v["resource_changes"]
        .as_array()
        .unwrap()
        .iter()
        .position(|n| n["address"] == address)
        .unwrap()
}
#[test]
fn agent_helper_discriminators_and_object_attributes_require_exact_pairs() {
    let base = fixture();
    let agent = "aws_bedrockagent_agent.main";
    let ai = index(&base, agent);
    for case in 0..6 {
        let mut v = base.clone();
        let version = index(&v, "aws_bedrock_guardrail_version.main");
        match case {
            0 => {
                v["resource_changes"][ai]["change"]["after"]["guardrail_configuration"][0]["guardrail_version"] =
                    json!("DRAFT")
            }
            1 => {
                v["resource_changes"][version]["change"]["after"]["guardrail_arn"] =
                    json!("wrong-parent")
            }
            2 => v["configuration"]["root_module"]["resources"][version]["expressions"] = json!({}),
            3 => {
                v["resource_changes"][ai]["change"]["after_unknown"] =
                    json!({"guardrail_configuration":true})
            }
            4 => {
                v["configuration"]["root_module"]["resources"][ai]["expressions"]["guardrail_configuration"] =
                    json!({"constant_value":[]})
            }
            _ => {
                v["resource_changes"][ai]["change"]["after"]["guardrail_configuration"][0]["guardrail_version"] =
                    json!("0")
            }
        }
        assert!(
            !proven(&v, agent, "guardrail_configuration"),
            "guardrail case{case}"
        );
    }
    for case in 0..4 {
        let mut v = base.clone();
        match case {
            0 => {
                v["resource_changes"][ai]["change"]["after"]["prompt_override_configuration"][0]["prompt_configurations"]
                    [0]["parser_mode"] = json!("DEFAULT")
            }
            1 => {
                v["resource_changes"][ai]["change"]["after_unknown"] =
                    json!({"prompt_override_configuration":[{"prompt_configurations":true}]})
            }
            2 => {
                v["resource_changes"][ai]["change"]["after"]["prompt_override_configuration"][0]["prompt_configurations"]
                    [0]["prompt_state"] = json!("UNKNOWN")
            }
            _ => {
                v["configuration"]["root_module"]["resources"][ai]["expressions"]["prompt_override_configuration"]
                    ["references"] = json!([
                    "aws_lambda_function.executor.arn",
                    "aws_iam_role.bedrock.arn"
                ])
            }
        }
        assert!(
            !proven(&v, agent, "prompt_override_configuration"),
            "parser case{case}"
        );
    }
    for (address, path, field, value) in [
        (
            "aws_bedrockagent_agent_action_group.main",
            "agent_id",
            "agent_version",
            json!("1"),
        ),
        (
            "aws_bedrockagent_agent_action_group.main",
            "agent_id",
            "action_group_id",
            Value::Null,
        ),
        (
            "aws_bedrockagent_agent_knowledge_base_association.main",
            "agent_id",
            "knowledge_base_state",
            json!("UNKNOWN"),
        ),
    ] {
        let mut v = base.clone();
        let i = index(&v, address);
        v["resource_changes"][i]["change"]["after"][field] = value;
        assert!(!proven(&v, address, path));
    }
    let address = "aws_bedrockagent_agent_action_group.main";
    let mut v = base.clone();
    let i = index(&v, address);
    v["resource_changes"][i]["change"]["after"]["action_group_executor"][0]["custom_control"] =
        json!("RETURN_CONTROL");
    assert!(!proven(&v, address, "action_group_executor.lambda"));
    let mut v = base.clone();
    let i = index(&v, "aws_bedrockagent_agent_knowledge_base_association.main");
    v["configuration"]["root_module"]["resources"][i]["expressions"]["knowledge_base_id"] =
        json!({});
    assert!(!proven(
        &v,
        "aws_bedrockagent_agent_knowledge_base_association.main",
        "agent_id"
    ));
}
#[test]
fn typed_branches_collection_completeness_and_independent_scope_are_required() {
    let base = fixture();
    for (address, path, pointer, value) in [
        (
            "aws_bedrockagent_knowledge_base.main",
            "storage_configuration.opensearch_serverless_configuration.collection_arn",
            "/storage_configuration/0/type",
            json!("RDS"),
        ),
        (
            "aws_bedrockagent_knowledge_base.main",
            "storage_configuration.opensearch_serverless_configuration.collection_arn",
            "/knowledge_base_configuration/0/type",
            json!("SQL"),
        ),
        (
            "aws_bedrockagent_data_source.main",
            "data_source_configuration.s3_configuration.bucket_arn",
            "/data_source_configuration/0/type",
            json!("WEB"),
        ),
        (
            "aws_bedrockagent_data_source.main",
            "vector_ingestion_configuration.context_enrichment_configuration.bedrock_foundation_model_configuration.model_arn",
            "/vector_ingestion_configuration/0/context_enrichment_configuration/0/type",
            json!("OTHER"),
        ),
        (
            "aws_bedrockagent_data_source.main",
            "vector_ingestion_configuration.custom_transformation_configuration.transformation.transformation_function.transformation_lambda_configuration.lambda_arn",
            "/vector_ingestion_configuration/0/custom_transformation_configuration/0/transformation/0/step_to_apply",
            json!("OTHER"),
        ),
        (
            "aws_bedrockagent_knowledge_base.provisioned",
            "knowledge_base_configuration.sql_knowledge_base_configuration.redshift_configuration.query_engine_configuration.provisioned_configuration.cluster_identifier",
            "/knowledge_base_configuration/0/sql_knowledge_base_configuration/0/redshift_configuration/0/query_engine_configuration/0/type",
            json!("SERVERLESS"),
        ),
        (
            "aws_bedrockagent_agent_alias.main",
            "routing_configuration",
            "/routing_configuration/0/agent_version",
            json!("DRAFT"),
        ),
        (
            "aws_bedrockagent_flow.main",
            "definition.node.configuration.agent.agent_alias_arn",
            "/definition/0/node/0/type",
            json!("LambdaFunction"),
        ),
    ] {
        let mut v = base.clone();
        let i = index(&v, address);
        *v["resource_changes"][i]["change"]["after"]
            .pointer_mut(pointer)
            .unwrap() = value;
        assert!(!proven(&v, address, path), "{address} {path}");
    }
    let flow = "aws_bedrockagent_flow.main";
    let path = "definition.node.configuration.agent.agent_alias_arn";
    for case in 0..3 {
        let mut v = base.clone();
        let i = index(&v, flow);
        match case {
            0 => {
                v["resource_changes"][i]["change"]["after_unknown"] =
                    json!({"definition":[{"node":true}]})
            }
            1 => {
                let node =
                    v["resource_changes"][i]["change"]["after"]["definition"][0]["node"][0].clone();
                v["resource_changes"][i]["change"]["after"]["definition"][0]["node"]
                    .as_array_mut()
                    .unwrap()
                    .push(node);
            }
            _ => {
                v["configuration"]["root_module"]["resources"][i]["expressions"]["definition"][0]
                    ["node"][0] = json!({})
            }
        }
        assert!(!proven(&v, flow, path), "collection case{case}");
    }
    let mut v = base.clone();
    let source = index(&v, "aws_bedrockagent_agent_alias.main");
    v["configuration"]["provider_config"]["aws.other"] = json!({"full_name":"registry.terraform.io/hashicorp/aws","expressions":{"region":{"constant_value":"us-west-2"}}});
    v["configuration"]["root_module"]["resources"][source]["provider_config_key"] =
        json!("aws.other");
    assert!(!proven(&v, flow, path));
    let mut raw = crate::plan::parse(FIXTURE).unwrap();
    let ai = raw
        .nodes
        .iter()
        .position(|n| n.address == "aws_bedrockagent_agent.main")
        .unwrap();
    let ri = raw
        .nodes
        .iter()
        .position(|n| n.address == "aws_iam_role.bedrock")
        .unwrap();
    for (i, n) in raw.nodes.iter_mut().enumerate() {
        if let Some(c) = &mut n.provider_configuration {
            c.account = crate::model::AwsAccountScope::new(if i == ri {
                "222222222222"
            } else {
                "111111111111"
            });
        }
    }
    assert_ne!(ai, ri);
    let g = crate::semantic::transform(&raw);
    assert!(
        !g.nodes[ai]
            .references
            .iter()
            .any(|r| r.attribute == "agent_resource_role_arn"
                && r.label == "Bedrock execution role"
                && r.complete
                && !r.dependency)
    );
}
#[test]
fn agent_resources_filter_and_named_states_do_not_supply_missing_endpoints() {
    let g = graph(&fixture());
    let selected = crate::view::single(
        &g,
        &crate::view::Options {
            focus: Some("aws_bedrockagent_agent.main".into()),
            depth: Some(0),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(selected.coverage.selected.resources, 1);
    let mut missing = fixture();
    let i = index(&missing, "aws_bedrockagent_agent.main");
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
