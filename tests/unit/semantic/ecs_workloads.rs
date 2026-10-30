use crate::{plan, semantic};
use serde_json::{Value, json};
const FIXTURE: &str = include_str!("../../fixtures/ecs-workloads-plan.json");

#[test]
fn workloads_verify_explicit_endpoints_preserving_service_ownership() {
    let graph = semantic::transform(&plan::parse(FIXTURE).unwrap());
    assert_eq!(graph.coverage.selected.resources, 18);
    for (address, fields) in [
        (
            "aws_ecs_daemon.agent",
            vec![
                "cluster_arn",
                "capacity_provider_arns",
                "daemon_task_definition_arn",
            ],
        ),
        (
            "aws_ecs_daemon_task_definition.agent",
            vec!["execution_role_arn", "task_role_arn"],
        ),
        (
            "aws_ecs_task_definition.app",
            vec!["execution_role_arn", "task_role_arn"],
        ),
        (
            "aws_ecs_express_gateway_service.web",
            vec![
                "cluster",
                "execution_role_arn",
                "infrastructure_role_arn",
                "task_role_arn",
                "network_configuration.subnets",
                "network_configuration.security_groups",
            ],
        ),
        (
            "aws_ecs_service.app",
            vec!["task_definition", "service_registries.registry_arn"],
        ),
        (
            "aws_ecs_task_set.canary",
            vec![
                "cluster",
                "service",
                "task_definition",
                "network_configuration.subnets",
                "network_configuration.security_groups",
                "service_registries.registry_arn",
            ],
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
    }
    let task = graph
        .nodes
        .iter()
        .find(|n| n.address == "aws_ecs_task_set.canary")
        .unwrap();
    assert_eq!(
        task.member_owner.as_ref().unwrap().target.address,
        "aws_ecs_service.app"
    );
    assert!(
        graph
            .edges
            .iter()
            .any(|e| e.kind == crate::model::EdgeKind::Containment
                && graph.nodes[e.from].address == "aws_vpc.main"
                && graph.nodes[e.to].address == "aws_ecs_express_gateway_service.web")
    );
    assert!(
        !graph
            .edges
            .iter()
            .any(|e| e.kind == crate::model::EdgeKind::Containment
                && graph.nodes[e.to].address == "aws_ecs_daemon.agent")
    );
    for format in [crate::OutputFormat::Svg, crate::OutputFormat::Html] {
        let out = crate::generate(
            FIXTURE,
            &crate::Options {
                format,
                ..Default::default()
            },
        )
        .unwrap()
        .content;
        assert!(!out.contains("PRIVATE_"));
    }
}

#[test]
fn task_revisions_and_same_cluster_membership_fail_closed() {
    for case in [
        "family",
        "wrong-revision",
        "unknown",
        "different-cluster",
        "literal-cluster",
    ] {
        let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
        let field = match case {
            "family" => {
                v["resource_changes"][17]["change"]["after"]["task_definition"] =
                    json!("PRIVATE_TASK:3");
                "task_definition"
            }
            "wrong-revision" => {
                v["resource_changes"][12]["change"]["after"]["arn"] =
                    json!("arn:aws:ecs:us-east-1:123456789012:task-definition/PRIVATE_TASK:4");
                "task_definition"
            }
            "unknown" => {
                v["resource_changes"][17]["change"]["after_unknown"] =
                    json!({"task_definition":true});
                "task_definition"
            }
            "different-cluster" => {
                v["resource_changes"][11]["change"]["after"]["cluster"] = json!("different");
                "service"
            }
            _ => {
                v["configuration"]["root_module"]["resources"][17]["expressions"]["cluster"] =
                    json!({"constant_value":"PRIVATE_CLUSTER"});
                "service"
            }
        };
        let graph = semantic::transform(&plan::parse(&v.to_string()).unwrap());
        let node = graph
            .nodes
            .iter()
            .find(|n| n.address == "aws_ecs_task_set.canary")
            .unwrap();
        assert!(
            !node.references.iter().any(|r| r.attribute == field
                && r.complete
                && Some(r.label) == super::label(&node.resource_type, field)),
            "{case}"
        );
        if field == "service" {
            assert!(node.member_owner.is_none());
        }
    }
}

#[test]
fn workload_endpoints_reject_missing_unknown_and_incompatible_provenance() {
    for (target, source, path, output) in [
        (14, 0, "cluster_arn", "arn"),
        (14, 9, "capacity_provider_arns", "arn"),
        (14, 13, "daemon_task_definition_arn", "arn"),
        (15, 2, "infrastructure_role_arn", "arn"),
        (15, 5, "network_configuration.subnets", "id"),
        (17, 16, "service_registries.registry_arn", "arn"),
    ] {
        for case in [
            "unknown",
            "different",
            "data",
            "delete",
            "deposed",
            "type",
            "binding",
            "unknown-input",
        ] {
            let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
            match case {
                "unknown" => {
                    v["resource_changes"][source]["change"]["after_unknown"] = json!({output:true})
                }
                "different" => {
                    v["resource_changes"][source]["change"]["after"][output] = json!("different")
                }
                "data" => v["resource_changes"][source]["mode"] = json!("data"),
                "delete" => v["resource_changes"][source]["change"]["actions"] = json!(["delete"]),
                "deposed" => v["resource_changes"][source]["deposed"] = json!("old"),
                "type" => v["resource_changes"][source]["type"] = json!("aws_s3_bucket"),
                "binding" => {
                    v["configuration"]["provider_config"]["aws.other"] = json!({"full_name":"hashicorp/aws","expressions":{"region":{"constant_value":"eu-west-1"}}});
                    v["configuration"]["root_module"]["resources"][source]["provider_config_key"] =
                        json!("aws.other");
                }
                _ => {
                    v["resource_changes"][target]["change"]["after_unknown"] =
                        json!({path.split('.').next().unwrap():true})
                }
            }
            let graph = semantic::transform(&plan::parse(&v.to_string()).unwrap());
            let address = v["resource_changes"][target]["address"].as_str().unwrap();
            let node = graph.nodes.iter().find(|n| n.address == address).unwrap();
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
fn daemon_revision_shape_and_network_collections_are_not_inferred() {
    for arn in [
        "family:2",
        "arn:aws:ecs:us-east-1:123456789012:daemon-task-definition/family",
        "arn:aws:ecs:us-east-1:123456789012:task-definition/family:2",
        "arn:aws:ecs:us-east-1:123456789012:daemon-task-definition/family:0",
        "arn:aws:ecs:us-east-1:123456789012:daemon-task-definition/family:+2",
    ] {
        assert!(!super::revision(arn, true), "{arn}");
    }
    for values in [
        json!([]),
        json!(["subnet-a", "unproven"]),
        json!(["subnet-a", "subnet-a"]),
    ] {
        let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
        v["resource_changes"][15]["change"]["after"]["network_configuration"][0]["subnets"] =
            values;
        let graph = semantic::transform(&plan::parse(&v.to_string()).unwrap());
        assert!(
            !graph
                .edges
                .iter()
                .any(|e| e.kind == crate::model::EdgeKind::Containment
                    && graph.nodes[e.to].address == "aws_ecs_express_gateway_service.web")
        );
    }
}

#[test]
fn workload_selection_preserves_coverage_and_filtered_service_owner() {
    let graph = semantic::transform(&plan::parse(FIXTURE).unwrap());
    let selected = crate::view::single(
        &graph,
        &crate::view::Options {
            focus: Some("aws_ecs_task_set.canary".into()),
            depth: Some(0),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(selected.coverage.selected.resources, 1);
    assert_eq!(selected.nodes.len(), 1);
    let svg = crate::svg::render(&selected, &crate::layout::Layout::new(&selected));
    assert!(svg.contains("aws_ecs_task_set.canary"));
    assert!(!svg.contains("PRIVATE_"));
    let architecture = crate::model::architecture::Architecture {
        states: ["one", "two"]
            .into_iter()
            .map(|s| (crate::model::StateId::new(s).unwrap(), graph.clone()))
            .collect(),
        relationships: Vec::new(),
    };
    let selected = crate::view::apply(&architecture, &Default::default()).unwrap();
    let svg = crate::svg::render_states(&selected, crate::svg::Options::default());
    let document = roxmltree::Document::parse(&svg).unwrap();
    for href in document.descendants().filter_map(|n| n.attribute("href")) {
        assert!(
            document
                .descendants()
                .any(|n| n.attribute("id") == href.strip_prefix('#'))
        );
    }
}

#[test]
fn unproven_existing_service_task_definition_keeps_one_ordinary_reference() {
    let graph = semantic::transform(
        &plan::parse(include_str!("../../fixtures/ecs-clusters-plan.json")).unwrap(),
    );
    for address in ["aws_ecs_service.a", "aws_ecs_service.b"] {
        let node = graph.nodes.iter().find(|n| n.address == address).unwrap();
        let refs: Vec<_> = node
            .references
            .iter()
            .filter(|r| r.attribute == "task_definition" && !r.dependency)
            .collect();
        assert_eq!(refs.len(), 1);
        assert_ne!(refs[0].label, "Task definition revision");
        assert!(!refs[0].targets.is_empty());
    }
}

#[test]
fn task_set_ownership_preserves_known_account_scope() {
    let mut parsed = plan::parse(FIXTURE).unwrap();
    for node in &mut parsed.nodes {
        node.provider_configuration.as_mut().unwrap().account =
            crate::model::AwsAccountScope::new("123456789012");
    }
    let graph = semantic::transform(&parsed);
    let task = graph
        .nodes
        .iter()
        .find(|n| n.address == "aws_ecs_task_set.canary")
        .unwrap();
    let service = graph
        .nodes
        .iter()
        .find(|n| n.address == "aws_ecs_service.app")
        .unwrap();
    assert!(service.entity.scope.as_ref().unwrap().account.is_some());
    assert_eq!(task.entity.scope, service.entity.scope);
    assert_eq!(
        task.member_owner.as_ref().unwrap().target.address,
        service.address
    );
    for address in ["aws_ecs_task_set.canary", "aws_ecs_service.app"] {
        for case in [
            "unknown-account",
            "different-account",
            "conflicting-account",
        ] {
            let mut input = parsed.clone();
            let config = input
                .nodes
                .iter_mut()
                .find(|n| n.address == address)
                .unwrap()
                .provider_configuration
                .as_mut()
                .unwrap();
            match case {
                "conflicting-account" => config.account_conflict = true,
                "different-account" => {
                    config.account = crate::model::AwsAccountScope::new("999999999999")
                }
                _ => config.account = None,
            }
            let graph = semantic::transform(&input);
            assert!(
                graph
                    .nodes
                    .iter()
                    .find(|n| n.address == "aws_ecs_task_set.canary")
                    .unwrap()
                    .member_owner
                    .is_none(),
                "{case} {address}"
            );
        }
    }
}
