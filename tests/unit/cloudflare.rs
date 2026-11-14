use super::*;
use crate::{plan, semantic};
use serde_json::json;

const FIXTURE: &str = include_str!("../fixtures/cloudflare-foundation-plan.json");

#[test]
fn inventory_modes_are_exact_and_complete() {
    let inventory: Value =
        serde_json::from_str(include_str!("../fixtures/cloudflare-inventory/5.28.1.json")).unwrap();
    let entries = inventory["resources"].as_array().unwrap();
    assert_eq!(entries.len(), types::TYPES.len());
    for entry in entries {
        let mode = if entry["mode"] == "managed" {
            EntityMode::Managed
        } else {
            EntityMode::Data
        };
        assert!(known(entry["type"].as_str().unwrap(), mode));
    }
    assert!(!known("cloudflare_zones", EntityMode::Managed));
    assert!(known("cloudflare_zones", EntityMode::Data));
    assert!(!known("cloudflare_future", EntityMode::Managed));
}

#[test]
fn exact_provider_modes_scopes_and_aws_remain_distinct() {
    let parsed = plan::parse(FIXTURE).unwrap();
    let graph = semantic::transform(&parsed);
    let node = |address| graph.nodes.iter().find(|n| n.address == address).unwrap();
    let zone = node("cloudflare_zone.example");
    assert!(zone.provider.is_cloudflare());
    assert_eq!(zone.placement, Placement::Spatial);
    assert!(zone.entity.scope.is_none());
    assert_eq!(
        zone.entity
            .cloudflare_scope
            .as_ref()
            .unwrap()
            .account
            .as_deref(),
        Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")
    );
    assert_eq!(
        zone.provider_configuration
            .as_ref()
            .unwrap()
            .alias
            .as_deref(),
        Some("edge")
    );
    assert_eq!(
        node("cloudflare_dns_record.www").placement,
        Placement::Logical
    );
    for address in [
        "cloudflare_zone.foreign",
        "cloudflare_zone.inferred",
        "cloudflare_future.unknown",
    ] {
        assert_eq!(node(address).role, ResourceRole::Unknown);
        assert!(node(address).entity.cloudflare_scope.is_none());
    }
    assert!(node("aws_s3_bucket.example").entity.scope.is_some());
    assert!(
        node("aws_s3_bucket.example")
            .entity
            .cloudflare_scope
            .is_none()
    );
    assert!(
        graph
            .edges
            .iter()
            .all(|e| e.kind != crate::model::EdgeKind::Containment)
    );
}

#[test]
fn scope_masks_are_checked_at_every_ancestor() {
    for mask in [
        json!(true),
        json!({"account": true, "id": true}),
        json!({"account": {"id": true}, "id": true}),
    ] {
        for field in ["after_unknown", "after_sensitive"] {
            let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
            v["resource_changes"][0]["change"][field] = mask.clone();
            let parsed = plan::parse(&v.to_string()).unwrap();
            let zone = parsed
                .nodes
                .iter()
                .find(|n| n.address == "cloudflare_zone.example")
                .unwrap();
            assert_eq!(zone.cloudflare_scope, Some(CloudflareScope::default()));
        }
    }
}

#[test]
fn invalid_scope_values_are_not_retained() {
    let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
    v["resource_changes"][0]["change"]["after"]["account"]["id"] = json!("PRIVATE_TOKEN<script>");
    v["resource_changes"][0]["change"]["after"]["id"] = json!("123456789012");
    let parsed = plan::parse(&v.to_string()).unwrap();
    assert!(!format!("{parsed:?}").contains("PRIVATE_"));
    assert_eq!(
        parsed
            .nodes
            .iter()
            .find(|n| n.address == "cloudflare_zone.example")
            .unwrap()
            .cloudflare_scope,
        Some(CloudflareScope::default())
    );
}

#[test]
fn named_states_and_deposed_instances_keep_identity() {
    let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
    let mut old = v["resource_changes"][0].clone();
    old["deposed"] = json!("old");
    old["change"]["actions"] = json!(["delete"]);
    old["change"]["before"] = old["change"]["after"].clone();
    old["change"]["after"] = Value::Null;
    v["resource_changes"].as_array_mut().unwrap().push(old);
    let inputs = ["edge", "other"].map(|name| crate::model::PlanInput {
        state_id: crate::model::StateId::new(name).unwrap(),
        plan_json: v.to_string(),
    });
    let plans = plan::parse_inputs(inputs.into()).unwrap();
    for parsed in plans.states.values() {
        let graph = semantic::transform(parsed);
        let zones: Vec<_> = graph
            .nodes
            .iter()
            .filter(|n| n.address == "cloudflare_zone.example")
            .collect();
        assert_eq!(zones.len(), 2);
        assert_ne!(zones[0].entity.id, zones[1].entity.id);
        assert_eq!(zones.iter().filter(|n| n.deposed_key.is_some()).count(), 1);
    }
}

#[test]
fn malformed_scope_masks_fail_closed() {
    for mask in [
        json!([true]),
        json!({"account": [true], "id": {}}),
        json!("invalid"),
    ] {
        let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
        v["resource_changes"][0]["change"]["after_sensitive"] = mask;
        let parsed = plan::parse(&v.to_string()).unwrap();
        let scope = &parsed
            .nodes
            .iter()
            .find(|n| n.address == "cloudflare_zone.example")
            .unwrap()
            .cloudflare_scope;
        assert_eq!(scope, &Some(CloudflareScope::default()));
    }
}
