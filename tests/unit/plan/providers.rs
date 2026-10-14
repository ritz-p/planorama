use crate::{
    icons::{self, ResourceIcon},
    model::{PlanInput, ProviderIdentity, ResourceRole, StateId},
    plan, semantic,
};
use serde_json::{Value, json};

fn fixture() -> Value {
    let resources = [
        ("aws_vpc.default", "aws"),
        ("aws_vpc.tokyo", "aws.tokyo"),
        ("aws_vpc.virginia", "aws.virginia"),
        ("aws_vpc.unresolved", "aws.missing"),
    ];
    json!({
        "format_version":"1.2",
        "resource_changes": resources.iter().map(|(address, _)| json!({"address":address,"type":"aws_vpc","provider_name":"registry.terraform.io/hashicorp/aws"})).collect::<Vec<_>>(),
        "configuration": {
            "provider_config": {
                "aws":{"full_name":"registry.terraform.io/hashicorp/aws","expressions":{"access_key":{"constant_value":"TOP_SECRET"}}},
                "aws.tokyo":{"full_name":"registry.terraform.io/hashicorp/aws","alias":"tokyo","expressions":{"region":{"constant_value":"ap-northeast-1"}}},
                "aws.virginia":{"full_name":"registry.terraform.io/hashicorp/aws","alias":"virginia"}
            },
            "root_module":{"resources":resources.iter().map(|(address, key)| json!({"address":address,"provider_config_key":key})).collect::<Vec<_>>()}
        }
    })
}

fn role(id: &str) -> Value {
    json!({"role_arn":{"constant_value":format!("arn:aws:iam::{id}:role/path/Planorama")},"external_id":{"constant_value":"PRIVATE_EXTERNAL"}})
}

#[test]
fn account_is_independent_of_global_region_and_module_overrides() {
    let mut value = fixture();
    value["configuration"]["provider_config"]["aws.tokyo"]["expressions"]["assume_role_with_web_identity"] =
        json!([role("123456789012")]);
    value["configuration"]["root_module"]["resources"][1]["expressions"]["region"] =
        json!({"references":["var.region"]});
    value["resource_changes"].as_array_mut().unwrap().push(json!({"address":"module.child[0].aws_cloudfront_distribution.main","type":"aws_cloudfront_distribution"}));
    value["configuration"]["root_module"]["module_calls"]["child"] = json!({"module":{"resources":[{"address":"aws_cloudfront_distribution.main","provider_config_key":"aws.tokyo"}]}});
    let graph = semantic::transform(
        &crate::plan::evidence::tests::parse_fixture(&value.to_string()).unwrap(),
    );
    for (address, global) in [
        ("aws_vpc.tokyo", false),
        ("module.child[0].aws_cloudfront_distribution.main", true),
    ] {
        let scope = graph
            .nodes
            .iter()
            .find(|n| n.address == address)
            .unwrap()
            .entity
            .scope
            .as_ref()
            .unwrap();
        assert_eq!(scope.account.as_ref().unwrap().as_str(), "123456789012");
        assert_eq!(scope.global, global);
        assert!(scope.region.is_none());
    }
}

#[test]
fn account_evidence_is_bound_to_provider_and_state_without_retaining_secrets() {
    let mut value = fixture();
    for (key, id) in [
        ("aws", "123456789012"),
        ("aws.tokyo", "210987654321"),
        ("aws.virginia", "123456789012"),
    ] {
        value["configuration"]["provider_config"][key]["expressions"]["assume_role"] =
            json!([role(id)]);
    }
    let raw = crate::plan::evidence::tests::parse_fixture(&value.to_string()).unwrap();
    let graph = semantic::transform(&raw);
    for (address, id) in [
        ("aws_vpc.default", "123456789012"),
        ("aws_vpc.tokyo", "210987654321"),
        ("aws_vpc.virginia", "123456789012"),
    ] {
        let node = raw.nodes.iter().find(|n| n.address == address).unwrap();
        assert_eq!(
            node.provider_configuration
                .as_ref()
                .unwrap()
                .account
                .as_ref()
                .unwrap()
                .as_str(),
            id
        );
        let scope = graph
            .nodes
            .iter()
            .find(|n| n.address == address)
            .unwrap()
            .entity
            .scope
            .as_ref()
            .unwrap();
        assert_eq!(scope.account.as_ref().unwrap().as_str(), id);
        assert!(!scope.global);
        assert_eq!(scope.region.is_some(), address == "aws_vpc.tokyo");
    }
    let debug = format!("{raw:?}{graph:?}");
    for secret in ["PRIVATE_EXTERNAL", "TOP_SECRET", "Planorama", "arn:aws:iam"] {
        assert!(!debug.contains(secret));
    }
    let input = |id| PlanInput {
        state_id: StateId::new(id).unwrap(),
        plan_json: value.to_string(),
    };
    let plans = plan::parse_inputs(vec![input("a"), input("b")]).unwrap();
    assert_eq!(plans.states.len(), 2);
    assert_eq!(
        plans,
        plan::parse_inputs(vec![input("b"), input("a")]).unwrap()
    );
    value["resource_changes"].as_array_mut().unwrap().reverse();
    value["configuration"]["root_module"]["resources"]
        .as_array_mut()
        .unwrap()
        .reverse();
    assert_eq!(
        raw,
        crate::plan::evidence::tests::parse_fixture(&value.to_string()).unwrap()
    );
}

#[test]
fn account_resolution_rejects_dynamic_invalid_conflicting_and_unbound_evidence() {
    for blocks in [
        json!([{"role_arn":{"references":["var.role"]}}]),
        json!([{"role_arn":{"constant_value":"arn:aws:iam::123456789012:role/Test","references":["var.role"]}}]),
        json!([role("prod")]),
        json!([role("12345678901")]),
        json!([role("123456789012"), role("210987654321")]),
        json!([role("123456789012"), {}]),
        json!({"constant_value":[{"role_arn":"arn:aws:iam::123456789012:role/Test"}]}),
    ] {
        let mut value = fixture();
        value["configuration"]["provider_config"]["aws"]["expressions"]["assume_role"] = blocks;
        let raw = plan::parse(&value.to_string()).unwrap();
        let config = raw
            .nodes
            .iter()
            .find(|n| n.address == "aws_vpc.default")
            .unwrap()
            .provider_configuration
            .as_ref()
            .unwrap();
        assert!(config.account.is_none());
        assert_eq!(
            semantic::diagnostics::collect(&raw)
                .iter()
                .any(|d| d.reason == crate::model::DiagnosticReason::ConflictingAwsAccount),
            config.account_conflict
        );
    }
    for scenario in 0..5 {
        let mut value = fixture();
        value["configuration"]["provider_config"]["aws"]["expressions"] = json!({"region":{"constant_value":"us-east-1"},"profile":{"constant_value":"123456789012"},"allowed_account_ids":{"constant_value":["123456789012"]}});
        if scenario > 0 {
            value["configuration"]["provider_config"]["aws"]["expressions"]["assume_role"] =
                json!([role("123456789012")]);
        }
        match scenario {
            1 => value["resource_changes"][0]["provider_name"] = json!("acme/custom"),
            2 => {
                value["configuration"]["provider_config"]["aws"]["full_name"] = json!("acme/custom")
            }
            3 => value["resource_changes"][0]["deposed"] = json!("old"),
            4 => {
                value["configuration"]["root_module"]["resources"][0]["provider_config_key"] =
                    json!("aws.missing")
            }
            _ => {}
        }
        let raw = plan::parse(&value.to_string()).unwrap();
        assert!(raw.nodes.iter().all(|n| {
            n.provider_configuration
                .as_ref()
                .is_none_or(|c| c.account.is_none())
        }));
    }
    for arn in [
        "arn:aws-cn:iam::123456789012:role/Test",
        "arn:aws:sts::123456789012:assumed-role/Test/session",
        "arn:aws:iam:us-east-1:123456789012:role/Test",
        "arn:aws:iam::123456789012:role/",
        "arn:aws:iam::123456789012:user/Test",
    ] {
        assert!(super::role_account(arn).is_none());
    }
}

#[test]
fn resource_region_overrides_do_not_inherit_provider_scope_or_change_binding() {
    for expression in [
        json!({"constant_value":"eu-west-3"}),
        json!({"references":["var.region"]}),
        Value::Null,
    ] {
        let mut value = fixture();
        value["configuration"]["root_module"]["resources"][1]["expressions"]["region"] = expression;
        let raw = plan::parse(&value.to_string()).unwrap();
        let graph = semantic::transform(&raw);
        let node = raw
            .nodes
            .iter()
            .find(|n| n.address == "aws_vpc.tokyo")
            .unwrap();
        assert!(node.has_region_override);
        let binding = node.provider_configuration.as_ref().unwrap();
        assert_eq!(binding.key, "aws.tokyo");
        assert_eq!(binding.region.as_ref().unwrap().0, "ap-northeast-1");
        assert!(
            graph
                .nodes
                .iter()
                .find(|n| n.address == node.address)
                .unwrap()
                .entity
                .scope
                .is_none()
        );
        assert!(!format!("{raw:?}{graph:?}").contains("eu-west-3"));
    }
}

#[test]
fn static_regions_follow_exact_bindings_and_survive_projection() {
    let mut value = fixture();
    for (key, region) in [
        ("aws", "eu-west-1"),
        ("aws.tokyo", "ap-northeast-1"),
        ("aws.virginia", "us-east-1"),
    ] {
        value["configuration"]["provider_config"][key]["expressions"]["region"] =
            json!({"constant_value":region});
    }
    value["resource_changes"]
        .as_array_mut()
        .unwrap()
        .push(json!({"address":"module.child[0].aws_vpc.main","type":"aws_vpc"}));
    value["configuration"]["root_module"]["module_calls"]["child"] = json!({"module":{"resources":[{"address":"aws_vpc.main","provider_config_key":"aws.tokyo"}]}});
    let raw = plan::parse(&value.to_string()).unwrap();
    let graph = semantic::transform(&raw);
    for (address, region) in [
        ("aws_vpc.default", "eu-west-1"),
        ("aws_vpc.tokyo", "ap-northeast-1"),
        ("aws_vpc.virginia", "us-east-1"),
        ("module.child[0].aws_vpc.main", "ap-northeast-1"),
    ] {
        let node = raw.nodes.iter().find(|n| n.address == address).unwrap();
        assert_eq!(
            node.provider_configuration
                .as_ref()
                .unwrap()
                .region
                .as_ref()
                .map(|r| r.0.as_str()),
            Some(region)
        );
        assert_eq!(
            graph
                .nodes
                .iter()
                .find(|n| n.address == address)
                .unwrap()
                .provider_configuration,
            node.provider_configuration
        );
    }
    assert!(!format!("{raw:?}{graph:?}").contains("TOP_SECRET"));
    value["resource_changes"].as_array_mut().unwrap().reverse();
    assert_eq!(raw, plan::parse(&value.to_string()).unwrap());
}

#[test]
fn unresolved_regions_and_foreign_providers_never_borrow_a_region() {
    for expression in [
        json!({"references":["var.region"]}),
        json!({"constant_value":"us-east-1","references":["var.region"]}),
        json!({"constant_value":42}),
        json!({"constant_value":""}),
        json!({"constant_value":"unsafe<region>"}),
        Value::Null,
    ] {
        let mut value = fixture();
        value["configuration"]["provider_config"]["aws.tokyo"]["expressions"]["region"] =
            expression;
        let raw = plan::parse(&value.to_string()).unwrap();
        assert!(raw.nodes.iter().all(|n| {
            n.provider_configuration
                .as_ref()
                .is_none_or(|c| c.region.is_none())
        }));
    }
    for foreign_resource in [false, true] {
        let mut value = fixture();
        value["configuration"]["provider_config"]["aws"]["expressions"]["region"] =
            json!({"constant_value":"us-east-1"});
        if foreign_resource {
            value["resource_changes"][0]["provider_name"] = json!("acme/custom");
        } else {
            value["configuration"]["provider_config"]["aws"]["full_name"] = json!("acme/custom");
        }
        let raw = plan::parse(&value.to_string()).unwrap();
        assert!(
            raw.nodes
                .iter()
                .find(|n| n.address == "aws_vpc.default")
                .unwrap()
                .provider_configuration
                .as_ref()
                .unwrap()
                .region
                .is_none()
        );
    }
}

#[test]
fn default_and_aliases_preserve_source_and_configuration_separately() {
    let value = fixture();
    let raw = plan::parse(&value.to_string()).unwrap();
    let graph = semantic::transform(&raw);
    for (address, key, alias) in [
        ("aws_vpc.default", "aws", None),
        ("aws_vpc.tokyo", "aws.tokyo", Some("tokyo")),
        ("aws_vpc.virginia", "aws.virginia", Some("virginia")),
        ("aws_vpc.unresolved", "aws.missing", None),
    ] {
        let node = raw.nodes.iter().find(|n| n.address == address).unwrap();
        assert_eq!(
            node.provider.source(),
            Some("registry.terraform.io/hashicorp/aws")
        );
        let config = node.provider_configuration.as_ref().unwrap();
        assert_eq!(config.key, key);
        assert_eq!(config.alias.as_deref(), alias);
        let projected = graph.nodes.iter().find(|n| n.address == address).unwrap();
        assert_eq!(
            projected.provider_configuration,
            node.provider_configuration
        );
        assert_eq!(projected.role, ResourceRole::Container);
    }
    assert!(!format!("{raw:?}{graph:?}").contains("TOP_SECRET"));

    let mut reordered = value.clone();
    reordered["resource_changes"]
        .as_array_mut()
        .unwrap()
        .reverse();
    reordered["configuration"]["root_module"]["resources"]
        .as_array_mut()
        .unwrap()
        .reverse();
    reordered["configuration"]["provider_config"]["aws"]["expressions"] =
        json!({"secret_key":{"constant_value":"OTHER_SECRET"}});
    assert_eq!(raw, plan::parse(&reordered.to_string()).unwrap());
}

#[test]
fn module_instances_retain_exact_provider_bindings() {
    let mut value = fixture();
    for (name, key) in [
        ("inherited", "aws.tokyo"),
        ("local", "module.local:aws.secondary"),
    ] {
        value["resource_changes"]
            .as_array_mut()
            .unwrap()
            .push(json!({"address":format!("module.{name}[0].aws_vpc.main[0]"),"type":"aws_vpc"}));
        value["configuration"]["root_module"]["module_calls"][name] =
            json!({"module":{"resources":[{"address":"aws_vpc.main","provider_config_key":key}]}});
    }
    value["configuration"]["provider_config"]["module.local:aws.secondary"] = json!({"full_name":"hashicorp/aws","alias":"secondary","module_address":"module.local","expressions":{"secret_key":{"constant_value":"TOP_SECRET"},"region":{"constant_value":"ap-northeast-1"}}});
    let raw = plan::parse(&value.to_string()).unwrap();
    for (name, key, alias) in [
        ("inherited", "aws.tokyo", "tokyo"),
        ("local", "module.local:aws.secondary", "secondary"),
    ] {
        let node = raw
            .nodes
            .iter()
            .find(|n| n.address == format!("module.{name}[0].aws_vpc.main[0]"))
            .unwrap();
        assert!(node.provider.is_aws());
        let config = node.provider_configuration.as_ref().unwrap();
        assert_eq!(config.key, key);
        assert_eq!(config.alias.as_deref(), Some(alias));
        assert_eq!(config.region.as_ref().unwrap().0, "ap-northeast-1");
    }
    assert!(!format!("{raw:?}").contains("TOP_SECRET"));
}

#[test]
fn missing_bindings_and_unresolved_metadata_degrade_without_alias_guessing() {
    for scenario in 0..5 {
        let mut value = fixture();
        value["resource_changes"][0]
            .as_object_mut()
            .unwrap()
            .remove("provider_name");
        let config = &mut value["configuration"]["root_module"]["resources"][0];
        match scenario {
            0 => config["provider_config_key"] = json!("aws.absent"),
            1 => config["provider_config_key"] = Value::Null,
            2 => config["provider_config_key"] = json!(42),
            3 => {
                config["provider_config_key"] = json!("aws.tokyo");
                value["configuration"]["provider_config"]["aws.tokyo"]
                    .as_object_mut()
                    .unwrap()
                    .remove("alias");
            }
            _ => value["resource_changes"][0]["provider_name"] = json!("acme/custom"),
        }
        let raw = plan::parse(&value.to_string()).unwrap();
        let node = raw
            .nodes
            .iter()
            .find(|n| n.address == "aws_vpc.default")
            .unwrap();
        assert_eq!(
            node.provider_configuration.is_some(),
            !matches!(scenario, 1 | 2)
        );
        if let Some(config) = &node.provider_configuration {
            assert!(config.alias.is_none());
        }
        match scenario {
            0 => assert!(!node.provider.is_aws()),
            4 => assert_eq!(node.provider, ProviderIdentity::from_source("acme/custom")),
            _ => assert!(node.provider.is_aws()),
        }
    }
}

#[test]
fn equal_provider_aliases_remain_scoped_to_their_named_states() {
    let input = |id| PlanInput {
        state_id: StateId::new(id).unwrap(),
        plan_json: fixture().to_string(),
    };
    let plans = plan::parse_inputs(vec![input("first"), input("second")]).unwrap();
    let identities: std::collections::BTreeSet<_> = plans
        .states
        .iter()
        .flat_map(|(state, plan)| {
            plan.nodes.iter().filter_map(move |n| {
                n.provider_configuration
                    .as_ref()
                    .map(|config| (state, &config.key))
            })
        })
        .collect();
    assert_eq!(identities.len(), 8);
    assert_eq!(
        plans,
        plan::parse_inputs(vec![input("second"), input("first")]).unwrap()
    );
}

#[test]
fn current_provider_binding_is_not_applied_to_deposed_predecessors() {
    for explicit_source in [false, true] {
        for current_present in [false, true] {
            let mut value = fixture();
            let address = "aws_vpc.default[0]";
            let mut predecessor = json!({
                "address":address, "type":"aws_vpc", "deposed":"old",
                "change":{"actions":["delete"]}
            });
            let expected_source = if explicit_source {
                predecessor["provider_name"] = json!("acme/historical");
                ProviderIdentity::from_source("acme/historical")
            } else {
                ProviderIdentity::inferred("aws_vpc")
            };
            let changes = value["resource_changes"].as_array_mut().unwrap();
            if current_present {
                changes[0] =
                    json!({"address":address,"type":"aws_vpc","change":{"actions":["create"]}});
            } else {
                changes.remove(0);
            }
            changes.push(predecessor);
            value["configuration"]["root_module"]["resources"][0]["provider_config_key"] =
                json!("aws.tokyo");
            let raw = plan::parse(&value.to_string()).unwrap();
            let graph = semantic::transform(&raw);
            let old = raw
                .nodes
                .iter()
                .find(|n| n.address == address && n.deposed_key.is_some())
                .unwrap();
            assert!(old.provider_configuration.is_none());
            assert_eq!(old.provider, expected_source);
            let projected = graph
                .nodes
                .iter()
                .find(|n| n.address == address && n.deposed_key.is_some())
                .unwrap();
            assert!(projected.provider_configuration.is_none());
            assert_eq!(projected.provider, expected_source);
            if current_present {
                let current = raw
                    .nodes
                    .iter()
                    .find(|n| n.address == address && n.deposed_key.is_none())
                    .unwrap();
                let configuration = current.provider_configuration.as_ref().unwrap();
                assert_eq!(configuration.key, "aws.tokyo");
                assert_eq!(configuration.alias.as_deref(), Some("tokyo"));
                assert_eq!(
                    current.provider,
                    ProviderIdentity::from_source("hashicorp/aws")
                );
            }
        }
    }
}

#[test]
fn terraform_provider_metadata_precedence_controls_icon_selection() {
    const BUILTIN: &str = "terraform.io/builtin/terraform";
    const FOREIGN: &str = "example.com/acme/terraform";

    for (change_source, planned_source, config_source, expected_source) in [
        (Some(BUILTIN), Some(FOREIGN), Some(FOREIGN), Some(BUILTIN)),
        (Some(FOREIGN), Some(BUILTIN), Some(BUILTIN), Some(FOREIGN)),
        (None, Some(BUILTIN), Some(FOREIGN), Some(BUILTIN)),
        (None, Some(FOREIGN), Some(BUILTIN), Some(FOREIGN)),
        (None, None, Some(BUILTIN), Some(BUILTIN)),
        (None, None, Some(FOREIGN), Some(FOREIGN)),
        (None, None, None, None),
    ] {
        for (mode, resource_type, address) in [
            ("managed", "terraform_data", "terraform_data.main"),
            (
                "data",
                "terraform_remote_state",
                "data.terraform_remote_state.main",
            ),
        ] {
            let mut resource = json!({"address":address,"type":resource_type,"mode":mode});
            if let Some(source) = change_source {
                resource["provider_name"] = json!(source);
            }
            let mut input = json!({"format_version":"1.2","resource_changes":[resource]});
            if let Some(source) = planned_source {
                input["planned_values"] = json!({"root_module":{"resources":[{
                    "address":address,"type":resource_type,"mode":mode,"provider_name":source
                }]}});
            }
            if let Some(source) = config_source {
                input["configuration"] = json!({
                    "provider_config":{"selected":{"full_name":source}},
                    "root_module":{"resources":[{"address":address,"provider_config_key":"selected"}]}
                });
            }
            let raw = plan::parse(&input.to_string()).unwrap();
            let expected = expected_source
                .map(ProviderIdentity::from_source)
                .unwrap_or(ProviderIdentity::InferredTerraform);
            assert_eq!(raw.nodes[0].provider, expected);
            assert_eq!(
                raw.nodes[0].provider.is_explicit(),
                expected_source.is_some()
            );
            let graph = semantic::transform(&raw);
            assert_eq!(graph.nodes.len(), 1);
            assert_eq!(graph.nodes[0].provider, expected);
            assert_eq!(graph.nodes[0].role, ResourceRole::Unknown);
            assert_eq!(
                icons::for_node(&graph.nodes[0]),
                (expected.source() == Some(BUILTIN)).then_some(ResourceIcon::Terraform)
            );
        }
    }
}

#[test]
fn unresolved_terraform_bindings_do_not_use_type_fallback() {
    for providers in [json!({}), json!({"selected":{}})] {
        let input = json!({
            "format_version":"1.2",
            "resource_changes":[
                {"address":"terraform_data.main","type":"terraform_data"},
                {"address":"data.terraform_remote_state.main","type":"terraform_remote_state","mode":"data"}
            ],
            "configuration":{
                "provider_config":providers,
                "root_module":{"resources":[
                    {"address":"terraform_data.main","provider_config_key":"selected"},
                    {"address":"data.terraform_remote_state.main","provider_config_key":"selected"}
                ]}
            }
        });
        let raw = plan::parse(&input.to_string()).unwrap();
        assert_eq!(raw.nodes.len(), 2);
        for node in &raw.nodes {
            assert_eq!(node.provider, ProviderIdentity::from_source("unknown"));
        }
        for node in &semantic::transform(&raw).nodes {
            assert_eq!(icons::for_node(node), None);
        }
    }
}

#[test]
fn captured_terraform_entities_keep_builtin_identity_after_projection() {
    let raw = plan::parse(include_str!("../../fixtures/terraform-plan.json")).unwrap();
    let graph = semantic::transform(&raw);
    assert_eq!(raw.nodes.len(), 16);
    assert_eq!(graph.nodes.len(), raw.nodes.len());
    for node in &raw.nodes {
        assert_eq!(
            node.provider,
            ProviderIdentity::from_source("terraform.io/builtin/terraform")
        );
        let projected = graph
            .nodes
            .iter()
            .find(|n| n.address == node.address)
            .unwrap();
        assert_eq!(projected.provider, node.provider);
        assert_eq!(icons::for_node(projected), Some(ResourceIcon::Terraform));
    }
}
