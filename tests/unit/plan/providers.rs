use crate::{
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
