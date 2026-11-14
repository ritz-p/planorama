use crate::{plan, semantic};
use serde_json::{Value, json};
const FIXTURE: &str = include_str!("../fixtures/cloudflare-zero-trust-plan.json");
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
fn access_and_tunnel_references_are_configuration_not_traffic() {
    let v = fixture();
    for (index, path) in [
        (4, "allowed_idps"),
        (4, "policies"),
        (6, "tunnel_id"),
        (8, "tunnel_id"),
        (8, "virtual_network_id"),
    ] {
        assert!(verified(&v, index, path), "{index}/{path}");
    }
    let graph = semantic::transform(&plan::parse(FIXTURE).unwrap());
    assert_eq!(graph.nodes.len(), 9);
    assert!(
        graph
            .edges
            .iter()
            .all(|e| e.kind == crate::model::EdgeKind::Dependency)
    );
    let representation = format!("{graph:?}");
    for private in ["PRIVATE_", "192.0.2.", "portal.example.invalid"] {
        assert!(!representation.contains(private), "{private}");
    }
    let policy = graph
        .nodes
        .iter()
        .find(|n| n.resource_type == "cloudflare_zero_trust_access_policy")
        .unwrap();
    assert_eq!(policy.role, crate::model::ResourceRole::Policy);
    assert_eq!(policy.placement, crate::model::Placement::Logical);
}
#[test]
fn access_scope_requires_an_unambiguous_account_or_zone() {
    for field in ["account_id", "zone_id"] {
        let mut v = fixture();
        v["resource_changes"][4]["change"]["after_unknown"][field] = json!(true);
        assert!(!verified(&v, 4, "allowed_idps"));
        assert!(!verified(&v, 4, "policies"));
    }
    let mut v = fixture();
    v["resource_changes"][4]["change"]["after"]["account_id"] =
        json!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
    assert!(!verified(&v, 4, "allowed_idps"));
    assert!(!verified(&v, 4, "policies"));
    let mut v = fixture();
    v["resource_changes"][2]["change"]["after"]["account_id"] =
        json!("eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee");
    assert!(!verified(&v, 4, "allowed_idps"));
    let mut v = fixture();
    v["resource_changes"][3]["change"]["after"]["account_id"] =
        json!("eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee");
    assert!(!verified(&v, 4, "policies"));
    let mut v = fixture();
    v["configuration"]["root_module"]["resources"][4]["expressions"]["zone_id"]["references"] =
        json!([]);
    assert!(!verified(&v, 4, "allowed_idps"));
    assert!(!verified(&v, 4, "policies"));
}
#[test]
fn zone_identity_provider_is_not_shared_with_other_zones_or_account_apps() {
    let mut v = fixture();
    v["resource_changes"][2]["change"]["after"]["account_id"] = Value::Null;
    v["resource_changes"][2]["change"]["after"]["zone_id"] =
        json!("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb");
    v["configuration"]["root_module"]["resources"][2]["expressions"] =
        json!({"zone_id":{"references":["cloudflare_zone.example.id"]}});
    assert!(verified(&v, 4, "allowed_idps"));
    v["resource_changes"][2]["change"]["after"]["zone_id"] =
        json!("cccccccccccccccccccccccccccccccc");
    assert!(!verified(&v, 4, "allowed_idps"));
    v["resource_changes"][2]["change"]["after"]["zone_id"] =
        json!("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb");
    v["resource_changes"][4]["change"]["after"]["zone_id"] = Value::Null;
    v["resource_changes"][4]["change"]["after"]["account_id"] =
        json!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
    assert!(!verified(&v, 4, "allowed_idps"));
    v["resource_changes"][2]["change"]["after"]["account_id"] =
        json!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
    v["resource_changes"][2]["change"]["after"]["zone_id"] = Value::Null;
    assert!(verified(&v, 4, "allowed_idps"));
}
#[test]
fn policy_arrays_and_identity_provider_sets_fail_closed() {
    for (field, value) in [
        ("allowed_idps", json!([true])),
        ("policies", json!([{"id":true}])),
    ] {
        let mut v = fixture();
        v["resource_changes"][4]["change"]["after_unknown"][field] = value;
        assert!(!verified(&v, 4, field));
        let mut v = fixture();
        v["resource_changes"][4]["change"]["after_sensitive"][field] = json!(true);
        assert!(!verified(&v, 4, field));
    }
    let mut v = fixture();
    v["resource_changes"][4]["change"]["after"]["policies"]
        .as_array_mut()
        .unwrap()
        .push(json!({"decision":"allow","include":[{"everyone":{}}]}));
    assert!(!verified(&v, 4, "policies"));
    let mut v = fixture();
    let id = v["resource_changes"][4]["change"]["after"]["allowed_idps"][0].clone();
    v["resource_changes"][4]["change"]["after"]["allowed_idps"]
        .as_array_mut()
        .unwrap()
        .push(id);
    assert!(!verified(&v, 4, "allowed_idps"));
    let mut v = fixture();
    v["configuration"]["root_module"]["resources"][4]["expressions"]["policies"]["references"] =
        json!(["cloudflare_zero_trust_access_policy.staff.name"]);
    assert!(!verified(&v, 4, "policies"));
}
#[test]
fn tunnel_helpers_require_current_unique_scoped_ids() {
    for (path, value) in [
        (
            "/resource_changes/5/change/after/account_id",
            json!("eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee"),
        ),
        ("/resource_changes/5/change/after_unknown/id", json!(true)),
        ("/resource_changes/5/change/after_sensitive/id", json!(true)),
        (
            "/resource_changes/5/provider_name",
            json!("foreign/cloudflare"),
        ),
        ("/resource_changes/5/change/actions", json!(["delete"])),
        (
            "/resource_changes/8/change/after/tunnel_id",
            json!("192.0.2.10"),
        ),
        (
            "/configuration/root_module/resources/8/expressions/tunnel_id/references",
            json!([]),
        ),
    ] {
        let mut v = fixture();
        let (parent, key) = path.rsplit_once('/').unwrap();
        v.pointer_mut(parent).unwrap()[key] = value;
        assert!(!verified(&v, 8, "tunnel_id"), "{path}");
    }
    let mut v = fixture();
    v["resource_changes"][7]["change"]["after"]["account_id"] =
        json!("eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee");
    assert!(!verified(&v, 8, "virtual_network_id"));
    let mut v = fixture();
    let mut duplicate = v["resource_changes"][5].clone();
    duplicate["address"] = json!("cloudflare_zero_trust_tunnel_cloudflared.duplicate");
    v["resource_changes"]
        .as_array_mut()
        .unwrap()
        .push(duplicate);
    assert!(!verified(&v, 8, "tunnel_id"));
}
#[test]
fn lifecycle_data_and_ambiguous_access_targets_preserve_boundaries() {
    let mut v = fixture();
    let mut duplicate = v["resource_changes"][2].clone();
    duplicate["address"] = json!("cloudflare_zero_trust_access_identity_provider.duplicate");
    v["resource_changes"]
        .as_array_mut()
        .unwrap()
        .push(duplicate);
    assert!(!verified(&v, 4, "allowed_idps"));
    let mut v = fixture();
    v["resource_changes"][4]["mode"] = json!("data");
    assert!(!verified(&v, 4, "policies"));
    let mut v = fixture();
    v["resource_changes"][3]["mode"] = json!("data");
    assert!(verified(&v, 4, "policies"));
    v["resource_changes"][4]["change"]["actions"] = json!(["delete", "create"]);
    let mut old = v["resource_changes"][3].clone();
    old["deposed"] = json!("old");
    old["change"]["actions"] = json!(["delete"]);
    v["resource_changes"].as_array_mut().unwrap().push(old);
    assert!(verified(&v, 4, "policies"));
}
