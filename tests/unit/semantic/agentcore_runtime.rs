use crate::{
    model::{EdgeKind, Placement},
    plan, semantic,
};
use serde_json::{Value, json};
const FIXTURE: &str = include_str!("../../fixtures/agentcore-runtime-plan.json");
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
fn contained(v: &Value, t: usize) -> bool {
    let g = graph(v);
    g.edges.iter().any(|e| {
        e.kind == EdgeKind::Containment
            && g.nodes[e.to].address == v["resource_changes"][t]["address"]
    })
}
#[test]
fn agentcore_runtime_typed_references_networks_and_versioned_endpoint_keep_exact_coverage() {
    let v = fixture();
    let g = graph(&v);
    assert_eq!(g.coverage.selected.resources, 24);
    assert_eq!(g.nodes.len(), 24);
    for (t, p) in [
        (14, "role_arn"),
        (
            14,
            "filesystem_configuration.efs_access_point.access_point_arn",
        ),
        (
            14,
            "filesystem_configuration.s3_files_access_point.access_point_arn",
        ),
        (
            14,
            "agent_runtime_artifact.code_configuration.code.s3.bucket",
        ),
        (15, "agent_runtime_id"),
        (16, "certificate.location.secrets_manager.secret_arn"),
        (16, "recording.s3_location.bucket"),
        (16, "enterprise_policy.location.s3.bucket"),
        (17, "execution_role_arn"),
        (17, "certificate.location.secrets_manager.secret_arn"),
        (18, "execution_role_arn"),
        (18, "memory.agentcore_memory_configuration.arn"),
        (18, "tool.config.agentcore_browser.browser_arn"),
        (
            18,
            "tool.config.agentcore_code_interpreter.code_interpreter_arn",
        ),
        (18, "tool.config.agentcore_gateway.gateway_arn"),
        (18, "model.openai_model_config.api_key_arn"),
        (
            18,
            "environment.agentcore_runtime_environment.filesystem_configuration.efs_access_point.access_point_arn",
        ),
    ] {
        assert!(verified(&v, t, p), "{t} {p}");
    }
    for t in [14, 16, 17, 18, 19, 23] {
        assert_eq!(node(&g, &v, t).placement, Placement::Spatial);
        assert!(node(&g, &v, t).member_owner.is_none());
        assert_eq!(contained(&v, t), matches!(t, 14 | 16 | 17));
    }
    assert!(node(&g, &v, 15).member_owner.is_some());
    assert!(node(&g, &v, 20).member_owner.is_none());
    for t in [15, 20] {
        assert_eq!(node(&g, &v, t).placement, Placement::Logical);
    }
    for n in g.nodes.iter().filter(|n| super::resource(&n.resource_type)) {
        assert!(
            n.references
                .iter()
                .all(|r| !super::indexed(&n.resource_type, &r.attribute))
        );
        assert_eq!(
            n.entity
                .scope
                .as_ref()
                .unwrap()
                .account
                .as_ref()
                .unwrap()
                .as_str(),
            "123456789012"
        );
    }
    for t in [6, 7, 8, 9, 10, 11, 12, 13, 21, 22, 23] {
        assert!(node(&g, &v, t).member_owner.is_none());
    }
    assert!(!verified(
        &v,
        18,
        "environment.agentcore_runtime_environment.agent_runtime_arn"
    ));
}
#[test]
fn agentcore_runtime_endpoint_boundaries_require_exact_current_explicit_outputs() {
    for (t, p, path, s, out, wrong) in [
        (14, "role_arn", "role_arn", 6, "arn", "name"),
        (
            14,
            "filesystem_configuration.efs_access_point.access_point_arn",
            "filesystem_configuration.0.efs_access_point.0.access_point_arn",
            10,
            "arn",
            "id",
        ),
        (
            14,
            "filesystem_configuration.s3_files_access_point.access_point_arn",
            "filesystem_configuration.1.s3_files_access_point.0.access_point_arn",
            11,
            "arn",
            "id",
        ),
        (
            14,
            "agent_runtime_artifact.code_configuration.code.s3.bucket",
            "agent_runtime_artifact.0.code_configuration.0.code.0.s3.0.bucket",
            7,
            "bucket",
            "arn",
        ),
        (
            15,
            "agent_runtime_id",
            "agent_runtime_id",
            14,
            "agent_runtime_id",
            "agent_runtime_arn",
        ),
        (
            16,
            "certificate.location.secrets_manager.secret_arn",
            "certificate.0.location.0.secrets_manager.0.secret_arn",
            8,
            "arn",
            "id",
        ),
        (
            16,
            "recording.s3_location.bucket",
            "recording.0.s3_location.0.bucket",
            7,
            "bucket",
            "arn",
        ),
        (
            18,
            "memory.agentcore_memory_configuration.arn",
            "memory.0.agentcore_memory_configuration.0.arn",
            12,
            "arn",
            "id",
        ),
        (
            18,
            "tool.config.agentcore_browser.browser_arn",
            "tool.0.config.0.agentcore_browser.0.browser_arn",
            16,
            "browser_arn",
            "browser_id",
        ),
        (
            18,
            "tool.config.agentcore_gateway.gateway_arn",
            "tool.2.config.0.agentcore_gateway.0.gateway_arn",
            13,
            "gateway_arn",
            "gateway_id",
        ),
        (
            18,
            "model.openai_model_config.api_key_arn",
            "model.0.openai_model_config.0.api_key_arn",
            8,
            "arn",
            "id",
        ),
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
            "source-unknown",
            "consumer-delete",
            "ambiguous",
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
                        json!({"references":[format!("{a}.{wrong}")]}),
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
                "ambiguous" => {
                    let a = v["resource_changes"][s]["address"]
                        .as_str()
                        .unwrap()
                        .to_owned();
                    set(
                        &mut v,
                        &format!("configuration.root_module.resources.{t}.expressions.{path}"),
                        json!({"references":[format!("{a}.{out}"),"aws_iam_role.main.name"]}),
                    );
                }
                _ => unreachable!(),
            }
            assert!(!verified(&v, t, p), "{t} {p} {case}");
        }
    }
}
#[test]
fn agentcore_runtime_network_placement_requires_vpc_mode_and_complete_same_vpc_sets() {
    for t in [14, 16, 17] {
        for case in [
            "public",
            "sandbox",
            "unknown-mode",
            "partial-subnets",
            "partial-groups",
            "duplicate",
            "wrong-vpc",
            "unknown-parent",
            "literal-parent",
            "shape",
            "object-leaf",
            "foreign-parent",
        ] {
            let mut v = fixture();
            let key = if t == 14 {
                "network_mode_config"
            } else {
                "vpc_config"
            };
            match case {
                "public" | "sandbox" => {
                    v["resource_changes"][t]["change"]["after"]["network_configuration"][0]["network_mode"] =
                        json!(if case == "public" {
                            "PUBLIC"
                        } else {
                            "SANDBOX"
                        })
                }
                "unknown-mode" => {
                    v["resource_changes"][t]["change"]["after_unknown"] =
                        json!({"network_configuration":[{"network_mode":true}]})
                }
                "partial-subnets" | "partial-groups" | "duplicate" => {
                    let field = if case == "partial-groups" {
                        "security_groups"
                    } else {
                        "subnets"
                    };
                    let known = if field == "subnets" {
                        "subnet-main"
                    } else {
                        "sg-main"
                    };
                    v["resource_changes"][t]["change"]["after"]["network_configuration"][0][key]
                        [0][field] = json!([
                        known,
                        if case == "duplicate" {
                            known
                        } else {
                            "external"
                        }
                    ]);
                }
                "wrong-vpc" => {
                    v["resource_changes"][2]["change"]["after"]["vpc_id"] = json!("vpc-other");
                    v["configuration"]["root_module"]["resources"][2]["expressions"]["vpc_id"] =
                        json!({"references":["aws_vpc.other.id"]});
                }
                "unknown-parent" => {
                    v["resource_changes"][1]["change"]["after_unknown"] = json!({"vpc_id":true})
                }
                "literal-parent" => {
                    v["configuration"]["root_module"]["resources"][1]["expressions"] = json!({})
                }
                "shape" => {
                    v["resource_changes"][t]["change"]["after"]["network_configuration"] =
                        json!({"network_mode":"VPC"})
                }
                "object-leaf" => {
                    v["resource_changes"][t]["change"]["after"]["network_configuration"][0][key]
                        [0]["subnets"] = json!("subnet-main")
                }
                "foreign-parent" => {
                    let mut alias = v["configuration"]["provider_config"]["aws"].clone();
                    alias["expressions"]["region"]["constant_value"] = json!("us-west-2");
                    v["configuration"]["provider_config"]["aws.foreign"] = alias;
                    v["configuration"]["root_module"]["resources"][0]["provider_config_key"] =
                        json!("aws.foreign");
                }
                _ => unreachable!(),
            }
            assert!(!contained(&v, t), "{t} {case}");
            assert!(!contained(&v, 18));
        }
    }
}
#[test]
fn agentcore_runtime_versions_and_selected_unions_do_not_guess() {
    for case in [
        "unknown",
        "mismatch",
        "zero",
        "foreign-version",
        "runtime-unknown",
        "literal-version",
    ] {
        let mut v = fixture();
        match case {
            "unknown" => {
                v["resource_changes"][15]["change"]["after_unknown"] =
                    json!({"agent_runtime_version":true})
            }
            "mismatch" => {
                v["resource_changes"][15]["change"]["after"]["agent_runtime_version"] = json!("1")
            }
            "zero" => {
                for t in [14, 15] {
                    v["resource_changes"][t]["change"]["after"]["agent_runtime_version"] =
                        json!("0");
                }
            }
            "foreign-version" => {
                v["configuration"]["root_module"]["resources"][15]["expressions"]["agent_runtime_version"] = json!({"references":["aws_bedrockagentcore_agent_runtime.other.agent_runtime_version"]})
            }
            "runtime-unknown" => {
                v["resource_changes"][14]["change"]["after_unknown"] =
                    json!({"agent_runtime_version":true})
            }
            "literal-version" => {
                v["configuration"]["root_module"]["resources"][15]["expressions"]["agent_runtime_version"] =
                    json!({"constant_value":"2"})
            }
            _ => unreachable!(),
        }
        assert_eq!(
            node(&graph(&v), &v, 15).member_owner.is_some(),
            case == "literal-version",
            "{case}"
        );
    }
    for (t, p, path, x) in [
        (
            14,
            "filesystem_configuration.efs_access_point.access_point_arn",
            "filesystem_configuration.0.session_storage",
            json!([{}]),
        ),
        (
            14,
            "agent_runtime_artifact.code_configuration.code.s3.bucket",
            "agent_runtime_artifact.0.container_configuration",
            json!([{"container_uri":"PRIVATE_IMAGE"}]),
        ),
        (
            16,
            "recording.s3_location.bucket",
            "recording.0.enabled",
            json!(false),
        ),
        (
            18,
            "memory.agentcore_memory_configuration.arn",
            "memory.0.managed_memory_configuration",
            json!([{"arn":"PRIVATE_COMPUTED_MEMORY"}]),
        ),
        (
            18,
            "tool.config.agentcore_browser.browser_arn",
            "tool.0.type",
            json!("remote_mcp"),
        ),
        (
            18,
            "tool.config.agentcore_gateway.gateway_arn",
            "tool.2.config.0.inline_function",
            json!([{}]),
        ),
        (
            18,
            "model.openai_model_config.api_key_arn",
            "model.0.bedrock_model_config",
            json!([{}]),
        ),
    ] {
        let mut v = fixture();
        let parts: Vec<_> = path.rsplitn(2, '.').collect();
        let parent = format!(
            "/resource_changes/{t}/change/after/{}",
            parts[1].replace('.', "/")
        );
        v.pointer_mut(&parent).unwrap()[parts[0]] = x;
        assert!(!verified(&v, t, p), "{t} {p}");
    }
    for (t, p, root) in [
        (
            16,
            "certificate.location.secrets_manager.secret_arn",
            "certificate",
        ),
        (
            16,
            "enterprise_policy.location.s3.bucket",
            "enterprise_policy",
        ),
        (18, "tool.config.agentcore_browser.browser_arn", "tool"),
    ] {
        let mut v = fixture();
        v["resource_changes"][t]["change"]["after_unknown"] = json!({root:true});
        assert!(!verified(&v, t, p));
    }
}
#[test]
fn agentcore_runtime_provider_account_and_region_are_independent() {
    for (s, t, p, global) in [
        (6, 14, "role_arn", true),
        (
            7,
            14,
            "agent_runtime_artifact.code_configuration.code.s3.bucket",
            false,
        ),
        (
            8,
            16,
            "certificate.location.secrets_manager.secret_arn",
            false,
        ),
        (14, 15, "agent_runtime_id", false),
        (16, 18, "tool.config.agentcore_browser.browser_arn", false),
        (12, 18, "memory.agentcore_memory_configuration.arn", false),
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
fn agentcore_runtime_actions_data_filters_states_and_payload_exclusion() {
    for t in 14..=18 {
        let mut v = fixture();
        v["resource_changes"][t]["change"]["actions"] = json!(["delete", "create"]);
        v["resource_changes"][t]["change"]["importing"] = json!({"id":"opaque"});
        assert_eq!(graph(&v).coverage.selected.resources, 24);
    }
    let mut v = fixture();
    v["resource_changes"][14]["mode"] = json!("data");
    assert!(node(&graph(&v), &v, 15).member_owner.is_none());
    assert!(!contained(&v, 14));
    let v = fixture();
    let selected = crate::view::single(
        &graph(&v),
        &crate::view::Options {
            focus: Some("aws_bedrockagentcore_agent_runtime_endpoint.main".into()),
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
            .any(|n| n.resource_type == super::RUNTIME)
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
