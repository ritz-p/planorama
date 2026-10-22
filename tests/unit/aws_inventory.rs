use super::*;
use crate::{model, plan, semantic};
use serde_json::json;
use std::collections::BTreeSet;

#[test]
fn every_registered_managed_type_has_basic_capabilities() {
    let inventory: serde_json::Value =
        serde_json::from_str(include_str!("../fixtures/aws-inventory/6.70.0.json")).unwrap();
    let resources = inventory["resources"].as_array().unwrap();
    let types: BTreeSet<_> = resources
        .iter()
        .map(|r| r["type"].as_str().unwrap())
        .collect();
    assert_eq!(types.len(), 1742);
    assert_eq!(ENTRIES.len(), types.len());
    assert!(
        ENTRIES
            .windows(2)
            .all(|pair| pair[0].resource_type < pair[1].resource_type)
    );
    for resource_type in types {
        let entry = get(resource_type).unwrap_or_else(|| panic!("Missing {resource_type}"));
        assert!(!entry.service.is_empty(), "{resource_type}");
        assert_ne!(
            super::super::classify(resource_type),
            ResourceRole::Unknown,
            "{resource_type}"
        );
        assert_ne!(
            super::super::placement(resource_type),
            Placement::Unresolved,
            "{resource_type}"
        );
        assert!(
            super::super::service_name(resource_type).is_some(),
            "{resource_type}"
        );
        if let Some(icon) = super::super::icon_for(resource_type) {
            assert!(!icon.svg().is_empty(), "{resource_type}");
            assert!(
                icon.svg().contains(&format!("id=\"{}\"", icon.id())),
                "{resource_type}"
            );
        }
        let future = format!("{resource_type}_unregistered_future");
        assert_eq!(super::super::classify(&future), ResourceRole::Unknown);
        assert!(super::super::icon_for(&future).is_none());
        if super::super::classification::classify(resource_type) == ResourceRole::Unknown {
            let mut parsed = plan::parse(&input(resource_type, "managed").to_string()).unwrap();
            parsed.nodes[0].provider_configuration = Some(model::ProviderConfiguration {
                key: "aws".into(),
                alias: None,
                region: Some(model::RegionScope("us-east-1".into())),
                account: None,
                account_conflict: false,
            });
            let scope = crate::provider::scope(&parsed.nodes[0]);
            assert_eq!(
                scope.as_ref().is_some_and(|s| s.global),
                entry.scope == Scope::Global,
                "{resource_type}"
            );
            assert_eq!(
                scope.as_ref().is_some_and(|s| s.region.is_some()),
                entry.scope == Scope::Regional,
                "{resource_type}"
            );
            if entry.scope == Scope::Unknown {
                assert!(
                    unknown_scope_reason(resource_type).is_some(),
                    "{resource_type}"
                );
            }
            parsed.nodes[0].mode = model::EntityMode::Data;
            assert_eq!(
                crate::classification::classify_entity(&parsed.nodes[0]),
                ResourceRole::Unknown,
                "{resource_type}"
            );
        }
    }
}

fn input(resource_type: &str, mode: &str) -> serde_json::Value {
    json!({"format_version":"1.2", "resource_changes":[{
        "address":format!("{resource_type}.example"),"type":resource_type,"mode":mode,
        "provider_name":"registry.terraform.io/hashicorp/aws", "change":{"actions":["create"]}
    }]})
}

#[test]
fn role_presentation_scope_and_provider_boundaries_are_independent() {
    for (resource_type, role, placement, global, regional) in [
        (
            "aws_mq_broker",
            ResourceRole::Node,
            Placement::Spatial,
            false,
            true,
        ),
        (
            "aws_mq_configuration",
            ResourceRole::Controller,
            Placement::Logical,
            false,
            true,
        ),
        (
            "aws_identitystore_user",
            ResourceRole::Identity,
            Placement::Logical,
            false,
            true,
        ),
        (
            "aws_identitystore_group_membership",
            ResourceRole::Association,
            Placement::Logical,
            false,
            true,
        ),
        (
            "aws_acmpca_policy",
            ResourceRole::Policy,
            Placement::Logical,
            false,
            true,
        ),
        (
            "aws_iam_service_linked_role",
            ResourceRole::Identity,
            Placement::Logical,
            true,
            false,
        ),
        (
            "aws_rolesanywhere_profile",
            ResourceRole::Node,
            Placement::Logical,
            false,
            true,
        ),
        (
            "aws_cloudcontrolapi_resource",
            ResourceRole::Node,
            Placement::Logical,
            false,
            false,
        ),
    ] {
        let mut parsed = plan::parse(&input(resource_type, "managed").to_string()).unwrap();
        parsed.nodes[0].provider_configuration = Some(model::ProviderConfiguration {
            key: "aws".into(),
            alias: None,
            region: Some(model::RegionScope("ap-northeast-1".into())),
            account: None,
            account_conflict: false,
        });
        let graph = semantic::transform(&parsed);
        let node = &graph.nodes[0];
        assert_eq!(node.role, role, "{resource_type}");
        assert_eq!(node.placement, placement, "{resource_type}");
        assert_eq!(
            node.entity.scope.as_ref().is_some_and(|scope| scope.global),
            global
        );
        assert_eq!(
            node.entity
                .scope
                .as_ref()
                .is_some_and(|scope| scope.region.is_some()),
            regional
        );
        assert_eq!(graph.coverage.selected.resources, 1);
        assert!(graph.edges.is_empty());
        parsed.nodes[0].action = model::Action::Delete;
        assert!(
            crate::provider::scope(&parsed.nodes[0]).is_none_or(|scope| scope.region.is_none())
        );
        parsed.nodes[0].action = model::Action::Create;
        parsed.nodes[0].has_region_override = true;
        parsed.nodes[0].region_override = None;
        assert!(
            crate::provider::scope(&parsed.nodes[0]).is_none_or(|scope| scope.region.is_none())
        );

        let data = plan::parse(&input(resource_type, "data").to_string()).unwrap();
        assert_eq!(
            crate::classification::classify_entity(&data.nodes[0]),
            super::super::classification::classify(resource_type)
        );
        for provider in [
            "example.com/foreign/aws",
            "registry.terraform.io/hashicorp/awscc",
        ] {
            let mut value = input(resource_type, "managed");
            value["resource_changes"][0]["provider_name"] = json!(provider);
            let graph = semantic::transform(&plan::parse(&value.to_string()).unwrap());
            assert_eq!(graph.nodes[0].role, ResourceRole::Unknown);
            assert_eq!(graph.nodes[0].placement, Placement::Unresolved);
            assert!(crate::icons::for_node(&graph.nodes[0]).is_none());
            assert!(graph.nodes[0].entity.scope.is_none());
        }
        let mut value = input(resource_type, "managed");
        value["resource_changes"][0]
            .as_object_mut()
            .unwrap()
            .remove("provider_name");
        value["configuration"] = json!({"root_module":{"resources":[{"address":format!("{resource_type}.example"),"provider_config_key":"aws.missing"}]}});
        let graph = semantic::transform(&plan::parse(&value.to_string()).unwrap());
        assert_eq!(graph.nodes[0].role, ResourceRole::Unknown);
    }
}

#[test]
fn basic_fixture_preserves_references_counts_and_private_values() {
    let fixture = include_str!("../fixtures/aws-basic-plan.json");
    let parsed = plan::parse(fixture).unwrap();
    let graph = semantic::transform(&parsed);
    assert_eq!(graph.nodes.len(), 10);
    assert_eq!(graph.coverage.selected.resources, 10);
    let membership = graph
        .nodes
        .iter()
        .find(|n| n.resource_type == "aws_identitystore_group_membership")
        .unwrap();
    assert_eq!(membership.references.len(), 1);
    assert_eq!(membership.references[0].label, "References");
    assert!(graph.edges.iter().all(|e| !matches!(
        e.kind,
        model::EdgeKind::Association | model::EdgeKind::Containment | model::EdgeKind::Connection
    )));
    for icon_mode in [crate::IconMode::Icon, crate::IconMode::ServiceName] {
        for format in [crate::OutputFormat::Svg, crate::OutputFormat::Html] {
            let generated = crate::generate(
                fixture,
                &crate::Options {
                    icon_mode,
                    format,
                    ..Default::default()
                },
            )
            .unwrap();
            assert!(!generated.content.contains("PRIVATE_"));
            assert!(generated.content.contains("AWS Account Access"));
            if matches!(icon_mode, crate::IconMode::ServiceName) {
                assert!(generated.content.contains("Amazon MQ"));
            } else {
                assert!(
                    generated
                        .content
                        .contains("planorama-icon-aws-service-amazon-mq")
                );
            }
            assert!(
                generated
                    .content
                    .contains("aws_identitystore_group_membership.operators")
            );
        }
    }
    let selected = crate::view::single(
        &graph,
        &crate::view::Options {
            focus: Some("aws_identitystore_group_membership.operators".into()),
            depth: Some(0),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(selected.nodes.len(), 1);
    assert_eq!(selected.coverage.selected.resources, 1);
}
