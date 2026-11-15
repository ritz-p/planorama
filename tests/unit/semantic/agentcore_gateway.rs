use crate::{
    model::{EdgeKind, Placement},
    plan, semantic,
};
use serde_json::{Value, json};
const FIXTURE: &str = include_str!("../../fixtures/agentcore-gateway-plan.json");
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
const JWT: &str = "authorizer_configuration.custom_jwt_authorizer";
const DISCOVERY: &str = "discovery_configuration.authorizer_configuration.custom_jwt_authorizer";
const CUSTOM: &str = "oauth2_provider_config.custom_oauth2_provider_config";
#[test]
fn agentcore_gateway_keeps_distinct_registries_shared_endpoints_and_owned_helpers() {
    let v = fixture();
    let g = graph(&v);
    assert_eq!(g.coverage.selected.resources, 36);
    assert_eq!(g.nodes.len(), 36);
    for (t, p) in [
        (18, "role_arn"),
        (18, "kms_key_arn"),
        (18, "interceptor_configuration.interceptor.lambda.arn"),
        (18, "policy_engine_configuration.arn"),
        (21, "encryption_configuration.kms_key_arn"),
        (23, "api_key_secret_config.secret_id"),
        (
            24,
            "oauth2_provider_config.custom_oauth2_provider_config.client_secret_config.secret_id",
        ),
        (
            25,
            "oauth2_provider_config.google_oauth2_provider_config.client_secret_config.secret_id",
        ),
        (
            26,
            "oauth2_provider_config.included_oauth2_provider_config.client_secret_config.secret_id",
        ),
        (
            24,
            "oauth2_provider_config.custom_oauth2_provider_config.private_key_jwt_config.private_key_source.kms_key_source.kms_key_arn",
        ),
        (27, "target_configuration.http.agentcore_runtime.arn"),
        (27, "credential_provider_configuration.api_key.provider_arn"),
        (28, "target_configuration.mcp.lambda.lambda_arn"),
        (28, "credential_provider_configuration.oauth.provider_arn"),
        (29, "target_configuration.mcp.api_gateway.rest_api_id"),
        (29, "target_configuration.mcp.api_gateway.stage"),
        (31, "action.route_to_target.static_route.target_name"),
        (
            32,
            "action.route_to_target.weighted_route.traffic_split.target_name",
        ),
    ] {
        assert!(verified(&v, t, p), "{t} {p}");
    }
    for (t, base) in [(19, JWT), (22, JWT)] {
        for tail in [
            "private_endpoint.self_managed_lattice_resource.resource_configuration_identifier",
            "private_endpoint_overrides.private_endpoint.managed_vpc_resource.vpc_identifier",
            "private_endpoint_overrides.private_endpoint.managed_vpc_resource.subnet_ids",
            "private_endpoint_overrides.private_endpoint.managed_vpc_resource.security_group_ids",
            "allowed_workload_configuration.hosting_environment.arn",
        ] {
            assert!(verified(&v, t, &format!("{base}.{tail}")), "{t} {tail}");
        }
    }
    for tail in [
        "private_endpoint.managed_vpc_resource.vpc_identifier",
        "private_endpoint.managed_vpc_resource.subnet_ids",
        "private_endpoint_override.private_endpoint.managed_vpc_resource.vpc_identifier",
        "private_endpoint_override.private_endpoint.managed_vpc_resource.subnet_ids",
        "private_endpoint_override.private_endpoint.managed_vpc_resource.security_group_ids",
        "private_endpoint_override.private_endpoint.self_managed_lattice_resource.resource_configuration_identifier",
    ] {
        assert!(verified(&v, 21, &format!("{DISCOVERY}.{tail}")), "{tail}");
    }
    assert!(verified(
        &v,
        24,
        &format!(
            "{CUSTOM}.private_endpoint_override.private_endpoint.managed_vpc_resource.subnet_ids"
        )
    ));
    assert!(verified(
        &v,
        27,
        "private_endpoint.managed_vpc_resource.subnet_ids"
    ));
    assert!(!verified(
        &v,
        33,
        "action.route_to_target.static_route.target_name"
    ));
    assert!(!verified(&v, 34, "api_key_secret_arn.secret_arn"));
    for n in g.nodes.iter().filter(|n| super::resource(&n.resource_type)) {
        assert_eq!(
            n.placement,
            if n.resource_type == super::GATEWAY {
                Placement::Spatial
            } else {
                Placement::Logical
            }
        );
        assert_eq!(
            n.member_owner.is_some(),
            matches!(n.resource_type.as_str(), super::RULE | super::TARGET)
        );
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
    for t in 18..36 {
        assert!(!contained(&v, t));
    }
    for t in [6, 7, 8, 9, 10, 11, 15, 17, 35] {
        assert!(node(&g, &v, t).member_owner.is_none());
    }
    assert_eq!(node(&g, &v, 21).resource_type, super::REGISTRY);
    assert_eq!(node(&g, &v, 22).resource_type, super::LEGACY);
    assert!(!node(&g, &v, 21).entity.scope.as_ref().unwrap().global);
}
#[test]
fn agentcore_gateway_requires_exact_current_explicit_typed_endpoints() {
    for (t, p, path, s, out, wrong) in [
        (18, "role_arn", "role_arn", 6, "arn", "name"),
        (18, "kms_key_arn", "kms_key_arn", 7, "arn", "key_id"),
        (
            18,
            "interceptor_configuration.interceptor.lambda.arn",
            "interceptor_configuration.0.interceptor.0.lambda.0.arn",
            9,
            "arn",
            "function_name",
        ),
        (
            18,
            "policy_engine_configuration.arn",
            "policy_engine_configuration.0.arn",
            17,
            "policy_engine_arn",
            "policy_engine_id",
        ),
        (
            21,
            "encryption_configuration.kms_key_arn",
            "encryption_configuration.0.kms_key_arn",
            7,
            "arn",
            "key_id",
        ),
        (
            23,
            "api_key_secret_config.secret_id",
            "api_key_secret_config.0.secret_id",
            8,
            "id",
            "name",
        ),
        (
            24,
            "oauth2_provider_config.custom_oauth2_provider_config.client_secret_config.secret_id",
            "oauth2_provider_config.0.custom_oauth2_provider_config.0.client_secret_config.0.secret_id",
            8,
            "arn",
            "name",
        ),
        (
            24,
            "oauth2_provider_config.custom_oauth2_provider_config.private_key_jwt_config.private_key_source.kms_key_source.kms_key_arn",
            "oauth2_provider_config.0.custom_oauth2_provider_config.0.private_key_jwt_config.0.private_key_source.0.kms_key_source.0.kms_key_arn",
            7,
            "arn",
            "key_id",
        ),
        (
            27,
            "gateway_identifier",
            "gateway_identifier",
            18,
            "gateway_id",
            "gateway_arn",
        ),
        (
            27,
            "target_configuration.http.agentcore_runtime.arn",
            "target_configuration.0.http.0.agentcore_runtime.0.arn",
            15,
            "agent_runtime_arn",
            "agent_runtime_id",
        ),
        (
            28,
            "target_configuration.mcp.lambda.lambda_arn",
            "target_configuration.0.mcp.0.lambda.0.lambda_arn",
            9,
            "arn",
            "function_name",
        ),
        (
            29,
            "target_configuration.mcp.api_gateway.stage",
            "target_configuration.0.mcp.0.api_gateway.0.stage",
            12,
            "stage_name",
            "id",
        ),
        (
            29,
            "target_configuration.mcp.api_gateway.rest_api_id",
            "target_configuration.0.mcp.0.api_gateway.0.rest_api_id",
            11,
            "id",
            "arn",
        ),
        (
            27,
            "credential_provider_configuration.api_key.provider_arn",
            "credential_provider_configuration.0.api_key.0.provider_arn",
            23,
            "credential_provider_arn",
            "name",
        ),
        (
            28,
            "credential_provider_configuration.oauth.provider_arn",
            "credential_provider_configuration.0.oauth.0.provider_arn",
            24,
            "credential_provider_arn",
            "name",
        ),
        (
            31,
            "action.route_to_target.static_route.target_name",
            "action.0.route_to_target.0.static_route.0.target_name",
            29,
            "name",
            "target_id",
        ),
        (
            19,
            "authorizer_configuration.custom_jwt_authorizer.allowed_workload_configuration.hosting_environment.arn",
            "authorizer_configuration.0.custom_jwt_authorizer.0.allowed_workload_configuration.0.hosting_environment.0.arn",
            18,
            "gateway_arn",
            "gateway_id",
        ),
        (
            27,
            "private_endpoint.managed_vpc_resource.vpc_identifier",
            "private_endpoint.0.managed_vpc_resource.0.vpc_identifier",
            0,
            "id",
            "arn",
        ),
        (
            27,
            "private_endpoint.managed_vpc_resource.subnet_ids",
            "private_endpoint.0.managed_vpc_resource.0.subnet_ids",
            1,
            "id",
            "arn",
        ),
        (
            27,
            "private_endpoint.managed_vpc_resource.security_group_ids",
            "private_endpoint.0.managed_vpc_resource.0.security_group_ids",
            2,
            "id",
            "arn",
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
                _ => unreachable!(),
            }
            assert!(!verified(&v, t, p), "{t} {p} {case}");
        }
    }
}
#[test]
fn agentcore_gateway_auth_choices_and_endpoint_types_do_not_alias() {
    for (t, p, path, x) in [
        (
            23,
            "api_key_secret_config.secret_id",
            "api_key_secret_source",
            json!("MANAGED"),
        ),
        (
            24,
            "oauth2_provider_config.custom_oauth2_provider_config.client_secret_config.secret_id",
            "credential_provider_vendor",
            json!("GoogleOauth2"),
        ),
        (
            24,
            "oauth2_provider_config.custom_oauth2_provider_config.client_secret_config.secret_id",
            "oauth2_provider_config.0.custom_oauth2_provider_config.0.client_secret_source",
            json!("MANAGED"),
        ),
        (
            26,
            "oauth2_provider_config.included_oauth2_provider_config.client_secret_config.secret_id",
            "credential_provider_vendor",
            json!("UnknownOauth2"),
        ),
        (
            19,
            "authorizer_configuration.custom_jwt_authorizer.allowed_workload_configuration.hosting_environment.arn",
            "authorizer_type",
            json!("AWS_IAM"),
        ),
        (
            21,
            "discovery_configuration.authorizer_configuration.custom_jwt_authorizer.private_endpoint.managed_vpc_resource.subnet_ids",
            "discovery_configuration.0.authorizer_type",
            json!("AWS_IAM"),
        ),
        (
            27,
            "target_configuration.http.agentcore_runtime.arn",
            "target_configuration.0.mcp",
            json!([{}]),
        ),
        (
            28,
            "target_configuration.mcp.lambda.lambda_arn",
            "target_configuration.0.mcp.0.connector",
            json!([{}]),
        ),
        (
            27,
            "credential_provider_configuration.api_key.provider_arn",
            "credential_provider_configuration.0.oauth",
            json!([{}]),
        ),
        (
            24,
            "oauth2_provider_config.custom_oauth2_provider_config.client_secret_config.secret_id",
            "oauth2_provider_config.0.google_oauth2_provider_config",
            json!([{}]),
        ),
    ] {
        let mut v = fixture();
        let mut parts = path.rsplitn(2, '.');
        let leaf = parts.next().unwrap();
        let parent = parts.next();
        let base = format!(
            "/resource_changes/{t}/change/after{}",
            parent.map_or(String::new(), |p| format!("/{}", p.replace('.', "/")))
        );
        v.pointer_mut(&base).unwrap()[leaf] = x;
        assert!(!verified(&v, t, p), "{t} {p}");
    }
    let mut v = fixture();
    v["configuration"]["root_module"]["resources"][27]["expressions"]["target_configuration"][0]
        ["http"][0]["agentcore_runtime"][0]["arn"] = json!({"references":["aws_bedrockagentcore_agent_runtime_endpoint.main.agent_runtime_arn"]});
    assert!(!verified(
        &v,
        27,
        "target_configuration.http.agentcore_runtime.arn"
    ));
    for t in [19, 22] {
        let mut v = fixture();
        let runtime = v["resource_changes"][15]["change"]["after"]["agent_runtime_arn"].clone();
        v["resource_changes"][t]["change"]["after"]["authorizer_configuration"][0]["custom_jwt_authorizer"]
            [0]["allowed_workload_configuration"][0]["hosting_environment"][0]["arn"] = runtime;
        v["configuration"]["root_module"]["resources"][t]["expressions"]["authorizer_configuration"]
            [0]["custom_jwt_authorizer"][0]["allowed_workload_configuration"][0]["hosting_environment"]
            [0]["arn"] =
            json!({"references":["aws_bedrockagentcore_agent_runtime.main.agent_runtime_arn"]});
        assert!(!verified(
            &v,
            t,
            &format!("{JWT}.allowed_workload_configuration.hosting_environment.arn")
        ));
    }
    for (t, p, field) in [
        (
            23,
            "api_key_secret_config.secret_id",
            "api_key_secret_source",
        ),
        (
            24,
            "oauth2_provider_config.custom_oauth2_provider_config.client_secret_config.secret_id",
            "credential_provider_vendor",
        ),
        (
            19,
            "authorizer_configuration.custom_jwt_authorizer.private_endpoint.self_managed_lattice_resource.resource_configuration_identifier",
            "authorizer_type",
        ),
    ] {
        let mut v = fixture();
        v["resource_changes"][t]["change"]["after_unknown"] = json!({field:true});
        assert!(!verified(&v, t, p));
    }
}
#[test]
fn agentcore_gateway_network_rows_require_complete_collections_and_their_own_vpc() {
    let p = format!(
        "{DISCOVERY}.private_endpoint_override.private_endpoint.managed_vpc_resource.subnet_ids"
    );
    for case in [
        "partial",
        "duplicate",
        "wrong-vpc",
        "array-vpc",
        "wrong-parent",
        "unknown-groups",
        "unknown-vpc",
        "both-variants",
        "object-subnets",
        "object-block",
        "oversized",
        "unknown-mode",
        "literal-parent",
        "foreign-parent",
    ] {
        let mut v = fixture();
        let row = &mut v["resource_changes"][21]["change"]["after"]["discovery_configuration"][0]["authorizer_configuration"]
            [0]["custom_jwt_authorizer"][0]["private_endpoint_override"][1]["private_endpoint"][0];
        match case {
            "partial" => {
                row["managed_vpc_resource"][0]["subnet_ids"] = json!(["subnet-other", "external"])
            }
            "duplicate" => {
                row["managed_vpc_resource"][0]["subnet_ids"] =
                    json!(["subnet-other", "subnet-other"])
            }
            "wrong-vpc" => row["managed_vpc_resource"][0]["vpc_identifier"] = json!("vpc-main"),
            "array-vpc" => row["managed_vpc_resource"][0]["vpc_identifier"] = json!(["vpc-other"]),
            "both-variants" => row["self_managed_lattice_resource"] = json!([{}]),
            "object-subnets" => {
                row["managed_vpc_resource"][0]["subnet_ids"] = json!("subnet-other")
            }
            "object-block" => row["managed_vpc_resource"] = json!({}),
            "unknown-mode" => {
                row["managed_vpc_resource"][0]["endpoint_ip_address_type"] = json!("UNKNOWN")
            }
            "wrong-parent" => {
                v["resource_changes"][4]["change"]["after"]["vpc_id"] = json!("vpc-main");
                v["configuration"]["root_module"]["resources"][4]["expressions"]["vpc_id"] =
                    json!({"references":["aws_vpc.main.id"]});
            }
            "unknown-groups" | "unknown-vpc" => {
                v["resource_changes"][21]["change"]["after_unknown"] = json!({"discovery_configuration":[{"authorizer_configuration":[{"custom_jwt_authorizer":[{"private_endpoint_override":[{}, {"private_endpoint":[{"managed_vpc_resource":[{if case=="unknown-groups"{"security_group_ids"}else{"vpc_identifier"}:true}]}]},{}]}]}]}]})
            }
            "oversized" => {
                let a = &mut v["resource_changes"][21]["change"]["after"]["discovery_configuration"]
                    [0]["authorizer_configuration"][0]["custom_jwt_authorizer"][0]["private_endpoint_override"];
                let row = a[1].clone();
                while a.as_array().unwrap().len() < 6 {
                    a.as_array_mut().unwrap().push(row.clone());
                }
            }
            "literal-parent" => {
                v["configuration"]["root_module"]["resources"][4]["expressions"] = json!({})
            }
            "foreign-parent" => {
                let mut a = v["configuration"]["provider_config"]["aws"].clone();
                a["expressions"]["region"]["constant_value"] = json!("us-west-2");
                v["configuration"]["provider_config"]["aws.foreign"] = a;
                v["configuration"]["root_module"]["resources"][3]["provider_config_key"] =
                    json!("aws.foreign");
            }
            _ => unreachable!(),
        }
        assert!(!verified(&v, 21, &p), "{case}");
        assert!(!contained(&v, 21));
    }
    let mut v = fixture();
    v["resource_changes"][24]["change"]["after"]["oauth2_provider_config"][0]["custom_oauth2_provider_config"]
        [0]["private_endpoint"] = Value::Null;
    assert!(!verified(
        &v,
        24,
        &format!(
            "{CUSTOM}.private_endpoint_override.private_endpoint.managed_vpc_resource.subnet_ids"
        )
    ));
    let mut v = fixture();
    v["resource_changes"][21]["change"]["after"]["discovery_configuration"][0]["authorizer_configuration"]
        [0]["custom_jwt_authorizer"][0]["private_endpoint"] = Value::Null;
    assert!(verified(&v, 21, &p));
}
#[test]
fn agentcore_gateway_names_are_qualified_by_gateway_rest_api_and_complete_weights() {
    for case in [
        "foreign-target",
        "unknown-gateway",
        "foreign-stage",
        "array-stage-parent",
        "array-target-api",
        "array-gateway",
        "wrong-protocol",
        "unknown-protocol",
        "bad-weight",
        "unknown-weight",
        "partial-split",
        "duplicate-target",
        "duplicate-name",
        "ambiguous-route",
        "ambiguous-action",
    ] {
        let mut v = fixture();
        let (t, p) = match case {
            "foreign-target" => {
                v["configuration"]["root_module"]["resources"][31]["expressions"]["action"][0]["route_to_target"]
                    [0]["static_route"][0]["target_name"] =
                    json!({"references":["aws_bedrockagentcore_gateway_target.foreign.name"]});
                (31, "action.route_to_target.static_route.target_name")
            }
            "unknown-gateway" => {
                v["resource_changes"][29]["change"]["after_unknown"] =
                    json!({"gateway_identifier":true});
                (31, "action.route_to_target.static_route.target_name")
            }
            "array-stage-parent" => {
                v["resource_changes"][12]["change"]["after"]["rest_api_id"] = json!(["rest-main"]);
                (29, "target_configuration.mcp.api_gateway.stage")
            }
            "array-target-api" => {
                v["resource_changes"][29]["change"]["after"]["target_configuration"][0]["mcp"][0]
                    ["api_gateway"][0]["rest_api_id"] = json!(["rest-main"]);
                (29, "target_configuration.mcp.api_gateway.stage")
            }
            "array-gateway" => {
                v["resource_changes"][29]["change"]["after"]["gateway_identifier"] =
                    json!(["gw-mcp"]);
                (31, "action.route_to_target.static_route.target_name")
            }
            "foreign-stage" => {
                v["configuration"]["root_module"]["resources"][29]["expressions"]["target_configuration"]
                    [0]["mcp"][0]["api_gateway"][0]["stage"] =
                    json!({"references":["aws_api_gateway_stage.other.stage_name"]});
                (29, "target_configuration.mcp.api_gateway.stage")
            }
            "wrong-protocol" => {
                v["resource_changes"][18]["change"]["after"]["protocol_type"] = json!("MCP");
                (27, "target_configuration.http.agentcore_runtime.arn")
            }
            "unknown-protocol" => {
                v["resource_changes"][19]["change"]["after_unknown"] =
                    json!({"protocol_type":true});
                (28, "target_configuration.mcp.lambda.lambda_arn")
            }
            _ => {
                let a = &mut v["resource_changes"][32]["change"]["after"]["action"][0];
                match case {
                    "bad-weight" => {
                        a["route_to_target"][0]["weighted_route"][0]["traffic_split"][0]["weight"] =
                            json!(89)
                    }
                    "unknown-weight" => {
                        v["resource_changes"][32]["change"]["after_unknown"] = json!({"action":[{"route_to_target":[{"weighted_route":[{"traffic_split":[{"weight":true},{}]}]}]}]})
                    }
                    "partial-split" => {
                        a["route_to_target"][0]["weighted_route"][0]["traffic_split"]
                            .as_array_mut()
                            .unwrap()
                            .pop();
                    }
                    "duplicate-target" => {
                        a["route_to_target"][0]["weighted_route"][0]["traffic_split"][1]["target_name"] =
                            json!("lambda")
                    }
                    "duplicate-name" => {
                        a["route_to_target"][0]["weighted_route"][0]["traffic_split"][1]["name"] =
                            json!("lambda")
                    }
                    "ambiguous-route" => a["route_to_target"][0]["static_route"] = json!([{}]),
                    "ambiguous-action" => a["configuration_bundle"] = json!([{}]),
                    _ => unreachable!(),
                };
                (
                    32,
                    "action.route_to_target.weighted_route.traffic_split.target_name",
                )
            }
        };
        assert!(!verified(&v, t, p), "{case}");
    }
}
#[test]
fn agentcore_gateway_parent_and_endpoint_bindings_are_independent() {
    for (s, t, p, global) in [
        (6, 18, "role_arn", true),
        (7, 21, "encryption_configuration.kms_key_arn", false),
        (8, 23, "api_key_secret_config.secret_id", false),
        (18, 27, "gateway_identifier", false),
        (
            19,
            31,
            "action.route_to_target.static_route.target_name",
            false,
        ),
        (11, 29, "target_configuration.mcp.api_gateway.stage", false),
        (
            3,
            21,
            "discovery_configuration.authorizer_configuration.custom_jwt_authorizer.private_endpoint_override.private_endpoint.managed_vpc_resource.subnet_ids",
            false,
        ),
    ] {
        for axis in ["account", "region"] {
            let mut v = fixture();
            let mut a = v["configuration"]["provider_config"]["aws"].clone();
            a["alias"] = json!("foreign");
            if axis == "account" {
                a["expressions"]["assume_role"][0]["role_arn"]["constant_value"] =
                    json!("arn:aws:iam::999999999999:role/Test");
            } else {
                a["expressions"]["region"]["constant_value"] = json!("us-west-2");
            }
            v["configuration"]["provider_config"]["aws.foreign"] = a;
            v["configuration"]["root_module"]["resources"][s]["provider_config_key"] =
                json!("aws.foreign");
            assert_eq!(verified(&v, t, p), global && axis == "region", "{s} {axis}");
        }
    }
}
#[test]
fn agentcore_gateway_lifecycle_states_filters_and_private_data_stay_separate() {
    for t in [18, 21, 22, 23, 24, 27, 31] {
        let mut v = fixture();
        v["resource_changes"][t]["change"]["actions"] = json!(["delete", "create"]);
        v["resource_changes"][t]["change"]["importing"] = json!({"id":"opaque"});
        assert_eq!(graph(&v).coverage.selected.resources, 36);
    }
    let mut v = fixture();
    v["resource_changes"][19]["mode"] = json!("data");
    assert!(node(&graph(&v), &v, 31).member_owner.is_none());
    assert!(!verified(
        &v,
        28,
        "target_configuration.mcp.lambda.lambda_arn"
    ));
    let v = fixture();
    let selected = crate::view::single(
        &graph(&v),
        &crate::view::Options {
            focus: Some("aws_bedrockagentcore_gateway_rule.static".into()),
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
            .any(|n| n.resource_type == super::GATEWAY)
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
fn agentcore_gateway_selected_block_shape_is_separate_from_opaque_child_values() {
    for (t, config) in [
        (24, "custom_oauth2_provider_config"),
        (25, "google_oauth2_provider_config"),
        (26, "included_oauth2_provider_config"),
    ] {
        let p = format!("oauth2_provider_config.{config}.client_secret_config.secret_id");
        let mut v = fixture();
        v["resource_changes"][t]["change"]["after_unknown"] =
            json!({"oauth2_provider_config":[{config:[{"client_id":true}]}]});
        assert!(verified(&v, t, &p), "unrelated opaque client ID {config}");
        v["resource_changes"][t]["change"]["after_unknown"] =
            json!({"oauth2_provider_config":[{config:true}]});
        assert!(!verified(&v, t, &p), "unknown selected block {config}");
        v["resource_changes"][t]["change"]["after_unknown"] = json!({"oauth2_provider_config":[{config:[{"client_secret_config":[{"secret_id":true}]}]}]});
        assert!(!verified(&v, t, &p), "unknown endpoint {config}");
        v["resource_changes"][t]["change"]["after_unknown"] =
            json!({"oauth2_provider_config":[{config:[{},{}]}]});
        assert!(!verified(&v, t, &p), "malformed mask length {config}");
    }
}
