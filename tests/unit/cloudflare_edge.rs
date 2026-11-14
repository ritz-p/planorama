use crate::{plan, semantic};
use serde_json::{Value, json};
const FIXTURE: &str = include_str!("../fixtures/cloudflare-edge-plan.json");
fn fixture() -> Value {
    serde_json::from_str(FIXTURE).unwrap()
}
fn verified(v: &Value, index: usize, path: &str) -> bool {
    let graph = semantic::transform(&plan::parse(&v.to_string()).unwrap());
    graph
        .nodes
        .iter()
        .find(|n| n.address == v["resource_changes"][index]["address"])
        .unwrap()
        .references
        .iter()
        .any(|r| {
            r.attribute == path
                && r.complete
                && super::label(v["resource_changes"][index]["type"].as_str().unwrap(), path)
                    == Some(r.label)
        })
}
#[test]
fn edge_configuration_uses_typed_targets_and_administration_only() {
    let v = fixture();
    for (index, path) in [
        (5, "monitor"),
        (6, "monitor_group"),
        (7, "default_pools"),
        (7, "fallback_pool"),
        (7, "region_pools"),
        (10, "rules"),
        (12, "rules"),
        (13, "members"),
    ] {
        assert!(verified(&v, index, path), "{index}/{path}");
    }
    let g = semantic::transform(&plan::parse(FIXTURE).unwrap());
    assert_eq!(g.nodes.len(), 14);
    assert!(
        g.edges
            .iter()
            .all(|e| e.kind == crate::model::EdgeKind::Dependency)
    );
    assert!(!format!("{g:?}").contains("PRIVATE_"));
}
#[test]
fn pool_collections_require_complete_values_provenance_and_matching_account() {
    for (path, value) in [
        (
            "/resource_changes/5/change/after/account_id",
            json!("eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee"),
        ),
        (
            "/resource_changes/7/change/after_unknown/default_pools",
            json!([false, true]),
        ),
        (
            "/resource_changes/7/change/after_sensitive/default_pools",
            json!(true),
        ),
        (
            "/resource_changes/7/change/after/default_pools",
            json!([
                "22222222222222222222222222222222",
                "22222222222222222222222222222222"
            ]),
        ),
        (
            "/configuration/root_module/resources/7/expressions/default_pools/references",
            json!(["cloudflare_load_balancer_pool.primary.id"]),
        ),
        (
            "/configuration/root_module/resources/7/expressions/zone_id/references",
            json!([]),
        ),
        (
            "/resource_changes/5/provider_name",
            json!("example/cloudflare"),
        ),
        ("/resource_changes/5/change/actions", json!(["delete"])),
    ] {
        let mut v = fixture();
        let (parent, key) = path.rsplit_once('/').unwrap();
        v.pointer_mut(parent).unwrap()[key] = value;
        assert!(!verified(&v, 7, "default_pools"), "{path}");
    }
}
#[test]
fn malformed_maps_sets_and_ambiguous_endpoints_fail_closed() {
    let mut v = fixture();
    v["resource_changes"][7]["change"]["after_unknown"] = json!({"region_pools":{"ENAM":true}});
    assert!(!verified(&v, 7, "region_pools"));
    let mut v = fixture();
    v["resource_changes"][13]["change"]["after_sensitive"] =
        json!({"members":[{"monitor_id":true}]});
    assert!(!verified(&v, 13, "members"));
    let mut v = fixture();
    let mut duplicate = v["resource_changes"][5].clone();
    duplicate["address"] = json!("cloudflare_load_balancer_pool.duplicate");
    v["resource_changes"]
        .as_array_mut()
        .unwrap()
        .push(duplicate);
    assert!(!verified(&v, 7, "default_pools"));
}
#[test]
fn rule_targets_require_action_kind_scope_and_typed_output() {
    for (index, path, value) in [
        (
            12,
            "/resource_changes/12/change/after/rules/0/action",
            json!("block"),
        ),
        (
            12,
            "/resource_changes/8/change/after/zone_id",
            json!("dddddddddddddddddddddddddddddddd"),
        ),
        (
            12,
            "/resource_changes/12/change/after/account_id",
            json!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
        ),
        (10, "/resource_changes/9/change/after/kind", json!("ip")),
        (
            10,
            "/configuration/root_module/resources/10/expressions/rules/references",
            json!(["cloudflare_list.redirects.id"]),
        ),
    ] {
        let mut v = fixture();
        let (parent, key) = path.rsplit_once('/').unwrap();
        v.pointer_mut(parent).unwrap()[key] = value;
        assert!(!verified(&v, index, "rules"), "{path}");
    }
}

#[test]
fn edge_data_aliases_and_lifecycle_never_change_owner_contracts() {
    let mut v = fixture();
    v["resource_changes"][5]["mode"] = json!("data");
    assert!(!verified(&v, 5, "monitor"));
    let mut v = fixture();
    v["configuration"]["root_module"]["resources"][5]["provider_config_key"] =
        json!("cloudflare.other");
    assert!(verified(&v, 7, "default_pools"));
    let mut v = fixture();
    v["resource_changes"][7]["change"]["actions"] = json!(["delete", "create"]);
    let mut old = v["resource_changes"][5].clone();
    old["deposed"] = json!("old");
    old["change"]["actions"] = json!(["delete"]);
    v["resource_changes"].as_array_mut().unwrap().push(old);
    assert!(verified(&v, 7, "default_pools"));
}

#[test]
fn an_unknown_rule_target_cannot_hide_behind_another_reference_to_the_same_list() {
    let mut v = fixture();
    let row = v["resource_changes"][10]["change"]["after"]["rules"][0].clone();
    v["resource_changes"][10]["change"]["after"]["rules"]
        .as_array_mut()
        .unwrap()
        .push(row);
    v["resource_changes"][10]["change"]["after_unknown"] =
        json!({"rules":[{}, {"action_parameters":true}]});
    assert!(!verified(&v, 10, "rules"));
}
