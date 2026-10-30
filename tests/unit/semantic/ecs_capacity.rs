use crate::{plan, semantic};
use serde_json::{Value, json};

fn fixture() -> Value {
    json!({"format_version":"1.2","resource_changes":[
        {"address":"aws_autoscaling_group.pool","mode":"managed","type":"aws_autoscaling_group","provider_name":"registry.terraform.io/hashicorp/aws","change":{"actions":["create"],"after":{"arn":"arn:aws:autoscaling:us-east-1:123456789012:autoScalingGroup:pool"}}},
        {"address":"aws_ecs_capacity_provider.pool","mode":"managed","type":"aws_ecs_capacity_provider","provider_name":"registry.terraform.io/hashicorp/aws","change":{"actions":["create"],"after":{"name":"pool","auto_scaling_group_provider":[{"auto_scaling_group_arn":"arn:aws:autoscaling:us-east-1:123456789012:autoScalingGroup:pool"}]}}}
    ],"configuration":{"provider_config":{"aws":{"full_name":"hashicorp/aws","expressions":{"region":{"constant_value":"us-east-1"}}}},"root_module":{"resources":[
        {"address":"aws_autoscaling_group.pool","mode":"managed","type":"aws_autoscaling_group","name":"pool","provider_config_key":"aws","expressions":{}},
        {"address":"aws_ecs_capacity_provider.pool","mode":"managed","type":"aws_ecs_capacity_provider","name":"pool","provider_config_key":"aws","expressions":{"auto_scaling_group_provider":[{"auto_scaling_group_arn":{"references":["aws_autoscaling_group.pool.arn"]}}]}}
    ]}}})
}

fn verified(value: &Value) -> bool {
    let graph = semantic::transform(&plan::parse(&value.to_string()).unwrap());
    graph
        .nodes
        .iter()
        .find(|n| n.address == "aws_ecs_capacity_provider.pool")
        .unwrap()
        .references
        .iter()
        .any(|r| r.label == "Capacity Auto Scaling group" && r.complete)
}

#[test]
fn capacity_provider_requires_exact_current_reference_and_exclusive_variant() {
    assert!(verified(&fixture()));
    for case in [
        "literal",
        "wrong-output",
        "mismatch",
        "unknown",
        "deleted",
        "deposed",
        "both",
        "cluster",
    ] {
        let mut v = fixture();
        match case {
            "literal" => {
                v["configuration"]["root_module"]["resources"][1]["expressions"] = json!({})
            }
            "wrong-output" => {
                v["configuration"]["root_module"]["resources"][1]["expressions"]["auto_scaling_group_provider"]
                    [0]["auto_scaling_group_arn"] =
                    json!({"references":["aws_autoscaling_group.pool.id"]})
            }
            "mismatch" => v["resource_changes"][0]["change"]["after"]["arn"] = json!("different"),
            "unknown" => {
                v["resource_changes"][1]["change"]["after_unknown"] =
                    json!({"auto_scaling_group_provider":true})
            }
            "deleted" => v["resource_changes"][0]["change"]["actions"] = json!(["delete"]),
            "deposed" => v["resource_changes"][0]["deposed"] = json!("old"),
            "both" => {
                v["resource_changes"][1]["change"]["after"]["managed_instances_provider"] =
                    json!([{}])
            }
            "cluster" => v["resource_changes"][1]["change"]["after"]["cluster"] = json!("cluster"),
            _ => unreachable!(),
        }
        assert!(!verified(&v), "{case}");
    }
}

#[test]
fn capacity_strategies_require_complete_unique_managed_names() {
    let mut v = fixture();
    let association = json!({"address":"aws_ecs_cluster_capacity_providers.pool","mode":"managed","type":"aws_ecs_cluster_capacity_providers","provider_name":"registry.terraform.io/hashicorp/aws","change":{"actions":["create"],"after":{"capacity_providers":["pool"],"default_capacity_provider_strategy":[{"capacity_provider":"pool"}]}}});
    v["resource_changes"]
        .as_array_mut()
        .unwrap()
        .push(association);
    v["configuration"]["root_module"]["resources"].as_array_mut().unwrap().push(json!({"address":"aws_ecs_cluster_capacity_providers.pool","mode":"managed","type":"aws_ecs_cluster_capacity_providers","name":"pool","provider_config_key":"aws","expressions":{"capacity_providers":{"references":["aws_ecs_capacity_provider.pool.name"]},"default_capacity_provider_strategy":[{"capacity_provider":{"references":["aws_ecs_capacity_provider.pool.name"]}}]}}));
    let check = |v: &Value, field: &str| {
        let graph = semantic::transform(&plan::parse(&v.to_string()).unwrap());
        graph
            .nodes
            .iter()
            .find(|n| n.address == "aws_ecs_cluster_capacity_providers.pool")
            .unwrap()
            .references
            .iter()
            .any(|r| {
                r.attribute == field && r.label == "Configured capacity providers" && r.complete
            })
    };
    assert!(check(&v, "capacity_providers"));
    assert!(check(
        &v,
        "default_capacity_provider_strategy.capacity_provider"
    ));
    for names in [
        json!(["pool", "unproven"]),
        json!(["pool", "pool"]),
        json!(["FARGATE"]),
        json!([]),
    ] {
        let mut changed = v.clone();
        changed["resource_changes"][2]["change"]["after"]["capacity_providers"] = names;
        assert!(!check(&changed, "capacity_providers"));
    }
    v["resource_changes"][2]["change"]["after_unknown"] =
        json!({"default_capacity_provider_strategy":[{"capacity_provider":true}]});
    assert!(!check(
        &v,
        "default_capacity_provider_strategy.capacity_provider"
    ));
}

#[test]
fn managed_capacity_cluster_uses_name_only_and_requires_known_variant() {
    let mut v = fixture();
    v["resource_changes"][0] = json!({"address":"aws_ecs_cluster.main","mode":"managed","type":"aws_ecs_cluster","provider_name":"registry.terraform.io/hashicorp/aws","change":{"actions":["create"],"after":{"name":"main"}}});
    v["configuration"]["root_module"]["resources"][0] = json!({"address":"aws_ecs_cluster.main","mode":"managed","type":"aws_ecs_cluster","name":"main","provider_config_key":"aws","expressions":{}});
    v["resource_changes"][1]["change"]["after"] =
        json!({"cluster":"main","managed_instances_provider":[{}]});
    v["configuration"]["root_module"]["resources"][1]["expressions"] =
        json!({"cluster":{"references":["aws_ecs_cluster.main.name"]}});
    let check = |v: &Value| {
        let graph = semantic::transform(&plan::parse(&v.to_string()).unwrap());
        let n = graph
            .nodes
            .iter()
            .find(|n| n.address == "aws_ecs_capacity_provider.pool")
            .unwrap();
        assert_eq!(n.placement, crate::model::Placement::Logical);
        assert!(
            !graph
                .edges
                .iter()
                .any(|e| e.kind == crate::model::EdgeKind::Containment
                    && graph.nodes[e.to].address == n.address)
        );
        n.references
            .iter()
            .any(|r| r.attribute == "cluster" && r.label == "Configured cluster" && r.complete)
    };
    assert!(check(&v));
    for case in ["unknown-variant", "absent-variant", "both", "wrong-output"] {
        let mut changed = v.clone();
        match case {
            "unknown-variant" => {
                changed["resource_changes"][1]["change"]["after_unknown"] =
                    json!({"managed_instances_provider":true})
            }
            "absent-variant" => {
                changed["resource_changes"][1]["change"]["after"]["managed_instances_provider"] =
                    json!([])
            }
            "both" => {
                changed["resource_changes"][1]["change"]["after"]["auto_scaling_group_provider"] =
                    json!([{}])
            }
            "wrong-output" => {
                changed["configuration"]["root_module"]["resources"][1]["expressions"] =
                    json!({"cluster":{"references":["aws_ecs_cluster.main.arn"]}})
            }
            _ => unreachable!(),
        }
        assert!(!check(&changed), "{case}");
    }
}

#[test]
fn capacity_fixture_covers_all_rules_without_network_inheritance() {
    let input = include_str!("../../fixtures/ecs-capacity-plan.json");
    let graph = semantic::transform(&plan::parse(input).unwrap());
    assert_eq!(graph.coverage.selected.resources, 12);
    for (address, fields) in [
        (
            "aws_ecs_capacity_provider.asg",
            vec!["auto_scaling_group_provider.auto_scaling_group_arn"],
        ),
        (
            "aws_ecs_capacity_provider.managed",
            vec![
                "cluster",
                "managed_instances_provider.infrastructure_role_arn",
                "managed_instances_provider.instance_launch_template.ec2_instance_profile_arn",
                "managed_instances_provider.instance_launch_template.network_configuration.subnets",
                "managed_instances_provider.instance_launch_template.network_configuration.security_groups",
            ],
        ),
        (
            "aws_ecs_cluster_capacity_providers.main",
            vec![
                "cluster_name",
                "capacity_providers",
                "default_capacity_provider_strategy.capacity_provider",
            ],
        ),
        (
            "aws_ecs_service.app",
            vec!["capacity_provider_strategy.capacity_provider"],
        ),
    ] {
        let node = graph.nodes.iter().find(|n| n.address == address).unwrap();
        for field in fields {
            assert!(
                node.references.iter().any(|r| r.attribute == field
                    && r.complete
                    && Some(r.label) == super::label(&node.resource_type, field)),
                "{address} {field}: {:?}",
                node.references
            );
        }
        if node.resource_type != "aws_ecs_service" {
            assert_eq!(node.placement, crate::model::Placement::Logical);
            assert!(
                !graph
                    .edges
                    .iter()
                    .any(|e| e.kind == crate::model::EdgeKind::Containment
                        && graph.nodes[e.to].address == address)
            );
        }
    }
    for format in [crate::OutputFormat::Svg, crate::OutputFormat::Html] {
        let content = crate::generate(
            input,
            &crate::Options {
                format,
                ..Default::default()
            },
        )
        .unwrap()
        .content;
        assert!(!content.contains("PRIVATE_"));
    }
}

#[test]
fn managed_capacity_references_fail_closed_on_incomplete_or_incompatible_evidence() {
    let base: Value =
        serde_json::from_str(include_str!("../../fixtures/ecs-capacity-plan.json")).unwrap();
    for (source, path, output) in [
        (
            2,
            "managed_instances_provider.infrastructure_role_arn",
            "arn",
        ),
        (
            3,
            "managed_instances_provider.instance_launch_template.ec2_instance_profile_arn",
            "arn",
        ),
        (
            5,
            "managed_instances_provider.instance_launch_template.network_configuration.subnets",
            "id",
        ),
        (
            7,
            "managed_instances_provider.instance_launch_template.network_configuration.security_groups",
            "id",
        ),
    ] {
        for case in [
            "unknown-output",
            "mismatch",
            "data",
            "delete",
            "deposed",
            "wrong-type",
            "binding",
            "unknown-block",
            "incomplete-set",
        ] {
            let mut v = base.clone();
            match case {
                "unknown-output" => {
                    v["resource_changes"][source]["change"]["after_unknown"] = json!({output:true})
                }
                "mismatch" => {
                    v["resource_changes"][source]["change"]["after"][output] = json!("different")
                }
                "data" => v["resource_changes"][source]["mode"] = json!("data"),
                "delete" => v["resource_changes"][source]["change"]["actions"] = json!(["delete"]),
                "deposed" => v["resource_changes"][source]["deposed"] = json!("old"),
                "wrong-type" => v["resource_changes"][source]["type"] = json!("aws_s3_bucket"),
                "binding" => {
                    v["configuration"]["provider_config"]["aws.other"] = json!({"full_name":"hashicorp/aws","expressions":{"region":{"constant_value":"eu-west-1"}}});
                    v["configuration"]["root_module"]["resources"][source]["provider_config_key"] =
                        json!("aws.other");
                }
                "unknown-block" => {
                    v["resource_changes"][9]["change"]["after_unknown"] =
                        json!({"managed_instances_provider":true})
                }
                "incomplete-set" if source == 5 || source == 7 => {
                    let field = if source == 5 {
                        "subnets"
                    } else {
                        "security_groups"
                    };
                    v["resource_changes"][9]["change"]["after"]["managed_instances_provider"][0]["instance_launch_template"][0]["network_configuration"][0][field].as_array_mut().unwrap().push(json!("unproven"));
                }
                "incomplete-set" => continue,
                _ => unreachable!(),
            }
            let graph = semantic::transform(&plan::parse(&v.to_string()).unwrap());
            let node = graph
                .nodes
                .iter()
                .find(|n| n.address == "aws_ecs_capacity_provider.managed")
                .unwrap();
            assert!(
                !node.references.iter().any(|r| r.attribute == path
                    && r.complete
                    && Some(r.label) == super::label(&node.resource_type, path)),
                "{path}: {case}"
            );
        }
    }
}

#[test]
fn capacity_selection_keeps_state_local_links_and_resource_coverage() {
    let input = include_str!("../../fixtures/ecs-capacity-plan.json");
    let graph = semantic::transform(&plan::parse(input).unwrap());
    let selected = crate::view::single(
        &graph,
        &crate::view::Options {
            focus: Some("aws_ecs_capacity_provider.managed".into()),
            depth: Some(0),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(selected.coverage.selected.resources, 1);
    assert_eq!(selected.nodes.len(), 1);
    let svg = crate::svg::render(&selected, &crate::layout::Layout::new(&selected));
    assert!(svg.contains("Unavailable"));
    let architecture = crate::model::architecture::Architecture {
        states: ["one", "two"]
            .into_iter()
            .map(|s| (crate::model::StateId::new(s).unwrap(), graph.clone()))
            .collect(),
        relationships: Vec::new(),
    };
    let selected = crate::view::apply(&architecture, &Default::default()).unwrap();
    let output = crate::svg::render_states(&selected, crate::svg::Options::default());
    let doc = roxmltree::Document::parse(&output).unwrap();
    for href in doc.descendants().filter_map(|n| n.attribute("href")) {
        assert!(
            doc.descendants()
                .any(|n| n.attribute("id") == href.strip_prefix('#'))
        );
    }
    assert!(!output.contains("PRIVATE_"));
}

#[test]
fn inactive_variant_accepts_known_empty_collection_masks() {
    for (target, inactive, path) in [
        (
            8,
            "managed_instances_provider",
            "auto_scaling_group_provider.auto_scaling_group_arn",
        ),
        (9, "auto_scaling_group_provider", "cluster"),
    ] {
        for (mask, expected) in [
            (json!([]), true),
            (json!(true), false),
            (json!([true]), false),
        ] {
            let mut value: Value =
                serde_json::from_str(include_str!("../../fixtures/ecs-capacity-plan.json"))
                    .unwrap();
            value["resource_changes"][target]["change"]["after"][inactive] = json!([]);
            value["resource_changes"][target]["change"]["after_unknown"] = json!({inactive:mask});
            let graph = semantic::transform(&plan::parse(&value.to_string()).unwrap());
            let address = value["resource_changes"][target]["address"]
                .as_str()
                .unwrap();
            let node = graph.nodes.iter().find(|n| n.address == address).unwrap();
            assert_eq!(
                node.references.iter().any(|r| r.attribute == path
                    && r.complete
                    && Some(r.label) == super::label(&node.resource_type, path)),
                expected,
                "{inactive}"
            );
        }
    }
}
