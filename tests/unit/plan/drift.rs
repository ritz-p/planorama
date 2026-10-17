use super::*;
use crate::{plan, semantic};
use serde_json::json;

#[test]
fn drift_only_provider_binding_preserves_alias_and_scope_without_reusing_deposed_evidence() {
    let input = json!({
        "format_version":"1.2",
        "resource_drift":[
            {"address":"aws_vpc.main","type":"aws_vpc","change":{"actions":["update"]}},
            {"address":"aws_vpc.main","type":"aws_vpc","deposed":"old","change":{"actions":["delete"]}}
        ],
        "configuration":{
            "provider_config":{"aws.paris":{
                "full_name":"hashicorp/aws","alias":"paris",
                "expressions":{"region":{"constant_value":"eu-west-3"}}
            }},
            "root_module":{"resources":[{"address":"aws_vpc.main","provider_config_key":"aws.paris"}]}
        }
    });
    let raw = plan::parse(&input.to_string()).unwrap();
    let current = raw.nodes.iter().find(|n| n.deposed_key.is_none()).unwrap();
    let binding = current.provider_configuration.as_ref().unwrap();
    assert!(current.provider.is_explicit());
    assert_eq!(binding.key, "aws.paris");
    assert_eq!(binding.alias.as_deref(), Some("paris"));
    assert_eq!(binding.region.as_ref().unwrap().0, "eu-west-3");
    assert!(
        raw.nodes
            .iter()
            .find(|n| n.deposed_key.is_some())
            .unwrap()
            .provider_configuration
            .is_none()
    );
    let graph = semantic::transform(&raw);
    let current = graph
        .nodes
        .iter()
        .find(|n| n.deposed_key.is_none())
        .unwrap();
    assert_eq!(
        current
            .entity
            .scope
            .as_ref()
            .unwrap()
            .region
            .as_ref()
            .unwrap()
            .0,
        "eu-west-3"
    );
}

#[test]
fn drift_does_not_overwrite_any_planned_action_or_expose_values() {
    for actions in [
        json!(["create"]),
        json!(["update"]),
        json!(["delete"]),
        json!(["delete", "create"]),
        json!(["no-op"]),
    ] {
        let input = json!({"format_version":"1.2","resource_changes":[{"address":"terraform_data.main","type":"terraform_data","change":{"actions":actions}}],
            "resource_drift":[{"address":"terraform_data.main","type":"terraform_data","change":{"actions":["update"],"before":{"secret":"TOP_SECRET"},"after":{"secret":"TOP_SECRET"}}}]});
        let raw = plan::parse(&input.to_string()).unwrap();
        assert_eq!(raw.nodes[0].action, super::super::parse_action(&actions));
        assert_eq!(raw.nodes[0].metadata.drift, [Action::Update].into());
        assert!(raw.drift[0].matched);
        assert!(!format!("{raw:?}").contains("TOP_SECRET"));
        assert_eq!(
            semantic::transform(&raw).nodes[0].metadata.drift,
            [Action::Update].into()
        );
    }
}

#[test]
fn drift_only_and_deposed_objects_are_distinct_and_deterministic() {
    let mut input = json!({"format_version":"1.2","resource_drift":[
        {"address":"aws_instance.main","type":"aws_instance","previous_address":"aws_instance.old","change":{"actions":["update"]}},
        {"address":"aws_instance.main","type":"aws_instance","provider_name":"registry.terraform.io/hashicorp/aws","change":{"actions":["update"]}},
        {"address":"aws_instance.main","type":"aws_instance","deposed":"old","change":{"actions":["delete"]}}
    ]});
    let raw = plan::parse(&input.to_string()).unwrap();
    assert_eq!(raw.nodes.len(), 2);
    assert!(
        raw.nodes
            .iter()
            .all(|n| n.action == Action::Unchanged && n.previous_address.is_none())
    );
    assert!(raw.nodes[0].provider.is_explicit());
    assert_eq!(raw.nodes[0].metadata.drift, [Action::Update].into());
    assert_eq!(raw.nodes[1].metadata.drift, [Action::Delete].into());
    assert!(
        raw.drift
            .iter()
            .any(|d| d.previous_address.as_deref() == Some("aws_instance.old"))
    );
    input["resource_drift"].as_array_mut().unwrap().reverse();
    assert_eq!(raw, plan::parse(&input.to_string()).unwrap());
}

#[test]
fn conflicting_drift_is_retained_but_not_attached_to_a_different_resource() {
    for foreign in [false, true] {
        let mut drift = json!({"address":"aws_instance.main","type":"aws_instance","change":{"actions":["delete"]}});
        if foreign {
            drift["provider_name"] = json!("registry.terraform.io/custom/aws");
        } else {
            drift["type"] = json!("aws_vpc");
        }
        let input = json!({"format_version":"1.2","resource_changes":[{"address":"aws_instance.main","type":"aws_instance","change":{"actions":["create"]}}],"resource_drift":[drift]});
        let raw = plan::parse(&input.to_string()).unwrap();
        assert!(raw.nodes[0].metadata.drift.is_empty());
        assert_eq!(raw.nodes[0].action, Action::Create);
        assert_eq!(raw.drift.len(), 1);
        assert!(!raw.drift[0].matched);
        assert!(
            semantic::diagnostics::collect(&raw)
                .iter()
                .any(|d| d.reason == crate::model::DiagnosticReason::DriftUnmatched)
        );
    }
}

#[test]
fn lowering_preserves_drift_separately_from_the_association_action() {
    let mut input: Value =
        serde_json::from_str(include_str!("../../fixtures/association-plan.json")).unwrap();
    input["resource_drift"] = json!([{"address":"aws_route_table_association.private","type":"aws_route_table_association","change":{"actions":["delete"]}}]);
    let raw = plan::parse(&input.to_string()).unwrap();
    let graph = semantic::transform(&raw);
    let change = graph.edges.iter().find_map(|e| e.change.as_ref()).unwrap();
    assert_eq!(change.action, Action::Create);
    assert_eq!(change.metadata.drift, [Action::Delete].into());
}
