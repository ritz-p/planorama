use crate::{model::*, plan, semantic};
use serde_json::{Value, json};

fn fixture(kind: &str) -> Value {
    let address = format!("{kind}.endpoint");
    let mut values =
        json!({"arn":"endpoint-arn","load_balancer_type":"application","subnets":["subnet-1"]});
    let mut expressions = json!({"subnets":{"references":["aws_subnet.test.id"]}});
    if kind == "aws_instance" {
        values = json!({"arn":"endpoint-arn","subnet_id":"subnet-1"});
        expressions = json!({"subnet_id":{"references":["aws_subnet.test.id"]}});
    }
    json!({
        "format_version":"1.2",
        "resource_changes":[
            {"address":"aws_cloudfront_vpc_origin.test","type":"aws_cloudfront_vpc_origin","mode":"managed","change":{"actions":["no-op"],"after":{"arn":"origin-output-arn","id":"vo-1","vpc_origin_endpoint_config":[{"arn":"endpoint-arn","origin_protocol_policy":"https-only"}]}}},
            {"address":address,"type":kind,"mode":"managed","change":{"actions":["no-op"],"after":values}},
            {"address":"aws_subnet.test","type":"aws_subnet","mode":"managed","change":{"actions":["no-op"],"after":{"id":"subnet-1","vpc_id":"vpc-1"}}}
        ],
        "configuration":{
            "provider_config":{"aws":{"name":"aws","full_name":"registry.terraform.io/hashicorp/aws","expressions":{"region":{"constant_value":"us-west-2"}}}},
            "root_module":{"resources":[
                {"address":"aws_cloudfront_vpc_origin.test","provider_config_key":"aws","expressions":{"vpc_origin_endpoint_config":[{"arn":{"references":[format!("{address}.arn")]}}]}},
                {"address":address,"provider_config_key":"aws","expressions":expressions},
                {"address":"aws_subnet.test","provider_config_key":"aws"}
            ]}
        }
    })
}

fn graph(value: &Value) -> ArchitectureGraph {
    semantic::transform(&plan::parse(&value.to_string()).unwrap())
}

#[test]
fn cloudfront_vpc_origin_keeps_global_control_and_independent_regional_endpoints() {
    for kind in ["aws_lb", "aws_alb", "aws_instance"] {
        let value = fixture(kind);
        let result = graph(&value);
        let origin = result
            .nodes
            .iter()
            .find(|n| n.resource_type == "aws_cloudfront_vpc_origin")
            .unwrap();
        assert!(origin.entity.scope.as_ref().unwrap().global);
        assert_eq!(origin.placement, Placement::Logical);
        assert!(
            origin
                .references
                .iter()
                .any(|r| r.label == "VPC origin endpoint" && r.complete && r.targets.len() == 1)
        );
        assert!(
            origin
                .references
                .iter()
                .any(|r| r.label == "Endpoint subnet context"
                    && r.complete
                    && r.targets[0].id.address == "aws_subnet.test")
        );
        let endpoint = result
            .nodes
            .iter()
            .find(|n| n.resource_type == kind)
            .unwrap();
        assert_eq!(
            endpoint
                .entity
                .scope
                .as_ref()
                .unwrap()
                .region
                .as_ref()
                .unwrap()
                .0,
            "us-west-2"
        );
        assert_eq!(result.coverage.selected.resources, 3);
    }
    let mut nlb = fixture("aws_lb");
    nlb["resource_changes"][1]["change"]["after"]["load_balancer_type"] = json!("network");
    assert!(
        graph(&nlb).nodes[0]
            .references
            .iter()
            .any(|r| r.label == "VPC origin endpoint" && r.complete)
    );
}

#[test]
fn cloudfront_vpc_origin_rejects_unproven_endpoints_without_losing_resources() {
    for case in 0..10 {
        let mut value = fixture("aws_lb");
        match case {
            0 => value["resource_changes"][1]["change"]["after"]["arn"] = json!("wrong"),
            1 => value["resource_changes"][1]["change"]["after_sensitive"] = json!({"arn":true}),
            2 => {
                value["resource_changes"][0]["change"]["after_unknown"] =
                    json!({"vpc_origin_endpoint_config":true})
            }
            3 => value["resource_changes"][1]["change"]["actions"] = json!(["delete"]),
            4 => value["resource_changes"][1]["mode"] = json!("data"),
            5 => {
                value["resource_changes"][1]["provider_name"] =
                    json!("registry.terraform.io/other/aws")
            }
            6 => {
                value["resource_changes"][1]["change"]["after"]["load_balancer_type"] =
                    json!("gateway")
            }
            7 => {
                value["resource_changes"][0]["change"]["after"]["vpc_origin_endpoint_config"][0]["arn"] =
                    json!("origin-output-arn")
            }
            8 => {
                value["configuration"]["root_module"]["resources"][0]["expressions"]["vpc_origin_endpoint_config"]
                    [0]["arn"] = json!({"references":["aws_lb.endpoint.arn","var.missing"]})
            }
            _ => value["resource_changes"][1]["deposed"] = json!("old"),
        }
        let result = graph(&value);
        assert!(
            !result.nodes[0]
                .references
                .iter()
                .any(|r| r.label == "VPC origin endpoint" && r.complete),
            "case {case}"
        );
        assert!(
            !result.nodes[0]
                .references
                .iter()
                .any(|r| r.label == "Endpoint subnet context")
        );
    }
}

#[test]
fn cloudfront_vpc_origin_network_context_requires_complete_non_sensitive_subnets() {
    for case in 0..4 {
        let mut value = fixture("aws_lb");
        match case {
            0 => {
                value["resource_changes"][1]["change"]["after_sensitive"] = json!({"subnets":true})
            }
            1 => value["resource_changes"][2]["change"]["after"]["id"] = json!("different"),
            2 => value["resource_changes"][2]["change"]["actions"] = json!(["delete"]),
            _ => {
                value["resource_changes"][1]["change"]["after"]["subnets"] =
                    json!(["subnet-1", "literal"])
            }
        }
        let result = graph(&value);
        assert!(
            result.nodes[0]
                .references
                .iter()
                .any(|r| r.label == "VPC origin endpoint" && r.complete)
        );
        assert!(
            !result.nodes[0]
                .references
                .iter()
                .any(|r| r.label == "Endpoint subnet context")
        );
    }
}

#[test]
fn cloudfront_vpc_origin_preserves_named_states_selection_and_privacy() {
    let value = fixture("aws_instance");
    let mut partial = value.clone();
    partial["resource_changes"]
        .as_array_mut()
        .unwrap()
        .remove(1);
    let states = plan::parse_inputs(
        [("one", value.clone()), ("two", partial)]
            .into_iter()
            .map(|(state, value)| PlanInput {
                state_id: StateId::new(state).unwrap(),
                plan_json: value.to_string(),
            })
            .collect(),
    )
    .unwrap();
    for (state, parsed) in states.states {
        let result = semantic::transform(&parsed);
        assert_eq!(
            result.nodes[0]
                .references
                .iter()
                .any(|r| r.label == "VPC origin endpoint" && r.complete),
            state.as_str() == "one"
        );
    }
    let selected = crate::view::single(
        &graph(&value),
        &crate::view::Options {
            focus: Some("aws_cloudfront_vpc_origin.test".into()),
            depth: Some(0),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(selected.coverage.selected.resources, 1);
    for format in [crate::OutputFormat::Svg, crate::OutputFormat::Html] {
        let output = crate::generate(
            &value.to_string(),
            &crate::Options {
                format,
                ..Default::default()
            },
        )
        .unwrap()
        .content;
        assert!(!output.contains("endpoint-arn"));
        assert!(!output.contains("origin-output-arn"));
        assert!(output.contains("VPC origin endpoint"));
    }
}

#[test]
fn cloudfront_vpc_origin_uses_verified_nlb_subnet_mappings() {
    let mut value = fixture("aws_lb");
    value["resource_changes"][1]["change"]["after"] = json!({
        "arn":"endpoint-arn","load_balancer_type":"network",
        "subnet_mapping":[{"subnet_id":"subnet-1"}]
    });
    value["configuration"]["root_module"]["resources"][1]["expressions"] = json!({
        "subnet_mapping":[{"subnet_id":{"references":["aws_subnet.test.id"]}}]
    });
    assert!(
        graph(&value).nodes[0]
            .references
            .iter()
            .any(|r| r.label == "Endpoint subnet context")
    );
    value["resource_changes"][1]["change"]["after_unknown"] = json!({"subnet_mapping":true});
    assert!(
        !graph(&value).nodes[0]
            .references
            .iter()
            .any(|r| r.label == "Endpoint subnet context")
    );
}

#[test]
fn cloudfront_vpc_origin_context_uses_known_effective_subnet_regions() {
    for kind in ["aws_lb", "aws_instance"] {
        for (default, override_region, unknown, expected) in [
            ("us-east-1", "us-west-2", false, true),
            ("us-west-2", "us-east-1", false, false),
            ("us-west-2", "us-west-2", true, false),
            ("us-west-2", "us-west-2", false, true),
        ] {
            let mut value = fixture(kind);
            value["terraform_version"] = json!("1.9.8");
            let provider = value["configuration"]["provider_config"]["aws"].clone();
            value["configuration"]["provider_config"]["aws.subnet"] = provider;
            value["configuration"]["provider_config"]["aws.subnet"]["version_constraint"] =
                json!("6.0.0");
            value["configuration"]["provider_config"]["aws.subnet"]["expressions"]["region"] =
                json!({"constant_value":default});
            let resource = &mut value["configuration"]["root_module"]["resources"][2];
            resource["provider_config_key"] = json!("aws.subnet");
            resource["mode"] = json!("managed");
            resource["type"] = json!("aws_subnet");
            resource["expressions"] = json!({"region":{"constant_value":override_region}});
            value["resource_changes"][2]["change"]["after"]["region"] = json!(override_region);
            value["resource_changes"][2]["change"]["after_unknown"] = json!({"region":unknown});
            let raw = plan::parse(&value.to_string()).unwrap();
            let mut nodes = raw.nodes.clone();
            for node in &mut nodes {
                node.provider_configuration.as_mut().unwrap().account =
                    AwsAccountScope::new("123456789012");
            }
            assert!(nodes[2].has_region_override);
            assert_eq!(nodes[2].region_override.is_some(), !unknown);
            let evidence: Vec<_> = value["resource_changes"]
                .as_array()
                .unwrap()
                .iter()
                .map(|r| (&r["change"]["after"], &r["change"]["after_unknown"]))
                .collect();
            assert_eq!(
                super::context(&nodes, &raw.attributes, &evidence, 1).is_some(),
                expected,
                "{kind}: {default}/{override_region}, unknown={unknown}"
            );
        }
    }
}

#[test]
fn cloudfront_vpc_origin_rejects_unknown_effective_endpoint_region() {
    let mut value = fixture("aws_instance");
    value["configuration"]["root_module"]["resources"][1]["expressions"]["region"] =
        json!({"references":["var.region"]});
    let result = graph(&value);
    assert!(!result.nodes[0].references.iter().any(
        |r| r.complete && matches!(r.label, "VPC origin endpoint" | "Endpoint subnet context")
    ));
}
