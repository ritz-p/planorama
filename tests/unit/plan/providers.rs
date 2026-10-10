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
                "aws.tokyo":{"full_name":"registry.terraform.io/hashicorp/aws","alias":"tokyo","expressions":{"region":{"constant_value":"PRIVATE_REGION"}}},
                "aws.virginia":{"full_name":"registry.terraform.io/hashicorp/aws","alias":"virginia"}
            },
            "root_module":{"resources":resources.iter().map(|(address, key)| json!({"address":address,"provider_config_key":key})).collect::<Vec<_>>()}
        }
    })
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
    assert!(!format!("{raw:?}{graph:?}").contains("PRIVATE_REGION"));
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
    value["configuration"]["provider_config"]["module.local:aws.secondary"] = json!({"full_name":"hashicorp/aws","alias":"secondary","module_address":"module.local","expressions":{"secret_key":{"constant_value":"TOP_SECRET"}}});
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
