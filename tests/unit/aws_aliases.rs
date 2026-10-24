use crate::{model, plan, provider, semantic};
use serde_json::{Value, json};

const TYPES: &[(&str, &str)] = &[
    ("aws_alb", "aws_lb"),
    ("aws_alb_listener", "aws_lb_listener"),
    (
        "aws_alb_listener_certificate",
        "aws_lb_listener_certificate",
    ),
    ("aws_alb_listener_rule", "aws_lb_listener_rule"),
    ("aws_alb_target_group", "aws_lb_target_group"),
    (
        "aws_alb_target_group_attachment",
        "aws_lb_target_group_attachment",
    ),
];

fn substitute(input: &str, mask: usize) -> String {
    let mut input = input.to_owned();
    for (index, (alias, canonical)) in TYPES.iter().enumerate().rev() {
        if mask & (1 << index) != 0 {
            input = input.replace(&format!("\"{canonical}\""), &format!("\"{alias}\""));
        }
    }
    input
}

fn normalized(value: impl std::fmt::Debug) -> String {
    format!("{value:?}").replace("aws_alb", "aws_lb")
}

fn equivalent(input: &str, mask: usize) {
    let canonical = plan::parse(input).unwrap();
    let alias = plan::parse(&substitute(input, mask)).unwrap();
    assert_eq!(
        normalized(&canonical),
        normalized(&alias),
        "parsed mask {mask}"
    );
    assert_eq!(
        normalized(semantic::transform(&canonical)),
        normalized(semantic::transform(&alias)),
        "graph mask {mask}"
    );
    assert_eq!(
        normalized(provider::diagnostics(&canonical)),
        normalized(provider::diagnostics(&alias)),
        "diagnostics mask {mask}"
    );
}

#[test]
fn aliases_and_mixed_endpoints_preserve_semantics_and_provenance() {
    for fixture in [
        include_str!("../fixtures/components-plan.json"),
        include_str!("../fixtures/aws-relationships-plan.json"),
        include_str!("../fixtures/security-groups-plan.json"),
        include_str!("../fixtures/network-scope-plan.json"),
        include_str!("../fixtures/route53-plan.json"),
        include_str!("../fixtures/cloudfront-plan.json"),
        include_str!("../fixtures/waf-plan.json"),
        include_str!("../fixtures/acm-plan.json"),
        include_str!("../fixtures/data-icons-plan.json"),
    ] {
        for mask in 1..64 {
            equivalent(fixture, mask);
        }
    }
}

#[test]
fn unknown_ambiguous_foreign_and_historical_inputs_keep_canonical_safety() {
    let fixture: Value =
        serde_json::from_str(include_str!("../fixtures/components-plan.json")).unwrap();
    for case in 0..9 {
        let mut input = fixture.clone();
        match case {
            0 => {
                input["configuration"]["root_module"]["resources"][0]["expressions"]["load_balancer_arn"] =
                    json!({"references":["aws_lb.app.arn", "aws_lb.other.arn"]})
            }
            1 => {
                input["configuration"]["root_module"]["resources"][0]["expressions"]["load_balancer_arn"] = json!({"constant_value":"arn:aws:elasticloadbalancing:us-east-1:123456789012:loadbalancer/app/literal"})
            }
            2 => {
                input["configuration"]["root_module"]["resources"][0]["provider_config_key"] =
                    "aws.unresolved".into()
            }
            3 => input["resource_changes"][0]["provider_name"] = "example.com/foreign/aws".into(),
            4 => input["resource_changes"][1]["provider_name"] = "example.com/foreign/aws".into(),
            5 => input["resource_changes"][0]["change"]["actions"] = json!(["delete"]),
            6 => input["resource_changes"][1]["deposed"] = "deadbeef".into(),
            7 => {
                input["resource_changes"][1]["change"]["after_unknown"] =
                    json!({"load_balancer_arn":true})
            }
            _ => {
                input["resource_changes"][1]["change"]["after"] =
                    json!({"load_balancer_arn":"unmatched"})
            }
        }
        equivalent(&input.to_string(), 63);
    }
}

#[test]
fn verified_modes_and_inventory_are_explicit() {
    let inventory: Value =
        serde_json::from_str(include_str!("../fixtures/aws-inventory/6.70.0.json")).unwrap();
    let aliases: Vec<_> = inventory["resources"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["aliasOf"].is_string())
        .collect();
    assert_eq!(aliases.len(), TYPES.len());
    for (alias, canonical) in TYPES {
        assert!(
            aliases
                .iter()
                .any(|r| r["type"] == *alias && r["aliasOf"] == *canonical)
        );
        assert_eq!(super::canonical(alias), *canonical);
        assert_eq!(
            provider::aws::classify(alias),
            provider::aws::classify(canonical)
        );
        assert_eq!(
            provider::aws::placement(alias),
            provider::aws::placement(canonical)
        );
        assert_eq!(
            provider::aws::icon_for(alias),
            provider::aws::icon_for(canonical)
        );
        assert_eq!(
            provider::aws::service_name(alias),
            provider::aws::service_name(canonical)
        );
        if ["aws_alb", "aws_alb_listener", "aws_alb_target_group"].contains(alias) {
            assert_eq!(
                provider::aws::data_role(alias),
                provider::aws::data_role(canonical)
            );
            let input = json!({"format_version":"1.2", "resource_changes":[{
                "address":format!("data.{canonical}.existing"), "type":canonical,
                "mode":"data", "provider_name":"registry.terraform.io/hashicorp/aws",
                "change":{"actions":["read"]}
            }]});
            equivalent(&input.to_string(), 63);
        } else {
            assert_eq!(super::data_type(alias), *alias);
            assert_eq!(
                provider::aws::data_role(alias),
                model::ResourceRole::Unknown
            );
        }
    }
    assert_eq!(super::canonical("aws_alb_future"), "aws_alb_future");
}

#[test]
fn mixed_instances_keep_original_identity_through_selection() {
    let input = include_str!("../fixtures/aws-aliases-plan.json");
    let plan = plan::parse(input).unwrap();
    let graph = semantic::transform(&plan);
    for address in [
        "aws_alb.web",
        "aws_lb.separate",
        "aws_alb_target_group.separate",
        "aws_lb_target_group.app",
    ] {
        let node = graph.nodes.iter().find(|n| n.address == address).unwrap();
        assert_eq!(node.resource_type, address.split('.').next().unwrap());
        assert_eq!(node.entity.provenance[0].address, address);
        let selected = crate::view::single(
            &graph,
            &crate::view::Options {
                focus: Some(address.into()),
                depth: Some(0),
                ..Default::default()
            },
        )
        .unwrap();
        assert!(selected.nodes.iter().any(|n| n.address == address));
    }
    assert!(graph.edges.iter().any(|e| {
        e.change
            .as_ref()
            .is_some_and(|c| c.address == "aws_alb_target_group_attachment.app")
    }));
    for (name, _) in TYPES {
        assert!(plan.nodes.iter().any(|n| n.resource_type == *name));
    }
    let canonical = input.replace("aws_alb", "aws_lb");
    let mut indexed: Value = serde_json::from_str(&canonical).unwrap();
    for resource in indexed["resource_changes"].as_array_mut().unwrap() {
        if resource["address"] == "aws_lb.web" {
            resource["address"] = "aws_lb.web[0]".into();
            resource["index"] = 0.into();
        }
    }
    let indexed = indexed
        .to_string()
        .replace("aws_lb.web.arn", "aws_lb.web[0].arn");
    equivalent(&indexed, 63);
}

#[test]
fn state_module_and_changed_filters_do_not_merge_alias_instances() {
    let input = json!({"format_version":"1.2", "resource_changes":[
        {"address":"module.app.aws_alb.web[0]", "type":"aws_alb", "change":{"actions":["create"]}},
        {"address":"module.app.aws_lb.web[0]", "type":"aws_lb", "change":{"actions":["no-op"]}},
        {"address":"module.other.aws_alb.web[0]", "type":"aws_alb", "change":{"actions":["create"]}}
    ]});
    let graph = semantic::transform(&plan::parse(&input.to_string()).unwrap());
    let architecture = model::architecture::Architecture {
        states: [
            (model::StateId::new("first").unwrap(), graph.clone()),
            (model::StateId::new("second").unwrap(), graph),
        ]
        .into_iter()
        .collect(),
        relationships: Vec::new(),
    };
    let selected = crate::view::apply(
        &architecture,
        &crate::view::Options {
            focus: Some("first:module.app.aws_alb.web[0]".into()),
            depth: Some(0),
            changed_only: true,
            modules: vec!["first:module.app".into()],
            ..Default::default()
        },
    )
    .unwrap();
    let retained: Vec<_> = selected
        .states
        .iter()
        .flat_map(|(state, graph)| {
            graph
                .nodes
                .iter()
                .map(move |node| (state.as_str(), node.address.as_str()))
        })
        .collect();
    assert_eq!(retained, [("first", "module.app.aws_alb.web[0]")]);
}
