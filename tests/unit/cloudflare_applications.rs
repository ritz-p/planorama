use crate::{plan, semantic};
use serde_json::{Value, json};
const FIXTURE: &str = include_str!("../fixtures/cloudflare-applications-plan.json");
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
fn application_storage_and_version_contracts_are_explicit() {
    let v = fixture();
    for (index, path) in [
        (6, "worker_id"),
        (6, "bindings"),
        (7, "script_name"),
        (7, "versions"),
        (8, "bindings"),
        (9, "script"),
        (10, "service"),
        (11, "deployment_configs"),
        (12, "project_name"),
    ] {
        assert!(verified(&v, index, path), "{index}/{path}");
    }
    let graph = semantic::transform(&plan::parse(FIXTURE).unwrap());
    assert_eq!(graph.nodes.len(), 13);
    assert!(!format!("{graph:?}").contains("PRIVATE_"));
    assert!(
        graph
            .edges
            .iter()
            .all(|e| e.kind == crate::model::EdgeKind::Dependency)
    );
}
#[test]
fn storage_requires_account_jurisdiction_aliases_and_complete_provenance() {
    for (path, value) in [
        (
            "/resource_changes/2/change/after/account_id",
            json!("eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee"),
        ),
        (
            "/resource_changes/2/change/after/jurisdiction",
            json!("default"),
        ),
        (
            "/resource_changes/2/change/after_unknown/jurisdiction",
            json!(true),
        ),
        ("/resource_changes/2/change/after/id", json!("different")),
        ("/resource_changes/4/change/after/uuid", json!("different")),
        (
            "/resource_changes/6/change/after_unknown/bindings",
            json!(true),
        ),
        (
            "/resource_changes/6/change/after_sensitive/bindings",
            json!(true),
        ),
        (
            "/configuration/root_module/resources/6/expressions/bindings/references",
            json!(["cloudflare_r2_bucket.assets.name"]),
        ),
        (
            "/resource_changes/3/provider_name",
            json!("foreign/cloudflare"),
        ),
        ("/resource_changes/3/change/actions", json!(["delete"])),
    ] {
        let mut v = fixture();
        let (parent, key) = path.rsplit_once('/').unwrap();
        v.pointer_mut(parent).unwrap()[key] = value;
        assert!(!verified(&v, 6, "bindings"), "{path}");
    }
    let mut v = fixture();
    v["resource_changes"][6]["change"]["after"]["bindings"][0]["type"] = json!("plain_text");
    assert!(!verified(&v, 6, "bindings"));
    let mut v = fixture();
    v["resource_changes"][6]["change"]["after"]["bindings"][2]["database_id"] = json!("different");
    assert!(!verified(&v, 6, "bindings"));
}
#[test]
fn names_are_not_worker_ids_and_versions_must_belong_to_the_deployed_worker() {
    let mut v = fixture();
    v["resource_changes"][6]["change"]["after"]["worker_id"] = json!("api");
    assert!(!verified(&v, 6, "worker_id"));
    assert!(!verified(&v, 7, "versions"));
    v["configuration"]["root_module"]["resources"][6]["expressions"]["worker_id"]["references"] =
        json!(["cloudflare_worker.api.name"]);
    assert!(verified(&v, 6, "worker_id"));
    assert!(verified(&v, 7, "versions"));
    let mut v = fixture();
    v["resource_changes"][7]["change"]["after"]["script_name"] = json!("legacy");
    v["configuration"]["root_module"]["resources"][7]["expressions"]["script_name"]["references"] =
        json!(["cloudflare_workers_script.legacy.id"]);
    assert!(verified(&v, 7, "script_name"));
    assert!(!verified(&v, 7, "versions"));
    for percentage in [json!(99), json!(101), json!("100")] {
        let mut v = fixture();
        v["resource_changes"][7]["change"]["after"]["versions"][0]["percentage"] = percentage;
        assert!(!verified(&v, 7, "versions"));
    }
}
#[test]
fn unknown_collection_entries_and_duplicate_storage_are_not_skipped() {
    let mut v = fixture();
    v["resource_changes"][6]["change"]["after_unknown"] =
        json!({"bindings":[{}, {"namespace_id":true}, {}, {}]});
    assert!(!verified(&v, 6, "bindings"));
    let mut v = fixture();
    v["resource_changes"][11]["change"]["after_unknown"] =
        json!({"deployment_configs":{"preview":{"kv_namespaces":{"UNKNOWN":true}}}});
    assert!(!verified(&v, 11, "deployment_configs"));
    let mut v = fixture();
    v["resource_changes"][11]["change"]["after_sensitive"] =
        json!({"deployment_configs":{"production":{"r2_buckets":true}}});
    assert!(!verified(&v, 11, "deployment_configs"));
    let mut v = fixture();
    let mut duplicate = v["resource_changes"][2].clone();
    duplicate["address"] = json!("cloudflare_r2_bucket.duplicate");
    v["resource_changes"]
        .as_array_mut()
        .unwrap()
        .push(duplicate);
    assert!(!verified(&v, 6, "bindings"));
    assert!(!verified(&v, 11, "deployment_configs"));
}
#[test]
fn routes_require_zone_provenance_and_data_consumers_do_not_publish_configuration() {
    let mut v = fixture();
    v["configuration"]["root_module"]["resources"][9]["expressions"]["zone_id"]["references"] =
        json!([]);
    assert!(!verified(&v, 9, "script"));
    let mut v = fixture();
    v["resource_changes"][6]["mode"] = json!("data");
    assert!(!verified(&v, 6, "bindings"));
    let mut v = fixture();
    v["resource_changes"][6]["change"]["actions"] = json!(["delete", "create"]);
    let mut deposed = v["resource_changes"][2].clone();
    deposed["deposed"] = json!("old");
    deposed["change"]["actions"] = json!(["delete"]);
    v["resource_changes"].as_array_mut().unwrap().push(deposed);
    assert!(verified(&v, 6, "bindings"));
}
#[test]
fn resource_specific_aliases_and_computed_binding_fields_are_checked() {
    let mut v = fixture();
    v["configuration"]["root_module"]["resources"][6]["expressions"]["bindings"]["references"] =
        json!([
            "cloudflare_r2_bucket.assets.id",
            "cloudflare_workers_kv_namespace.cache.id",
            "cloudflare_d1_database.content.uuid"
        ]);
    assert!(verified(&v, 6, "bindings"));
    v["resource_changes"][6]["change"]["after_unknown"] =
        json!({"bindings":[{}, {}, {"database_id":true}, {}]});
    assert!(verified(&v, 6, "bindings"));
    let mut v = fixture();
    v["resource_changes"][8]["change"]["after_unknown"] =
        json!({"bindings":[{}, {}, {"database_id":true}, {}]});
    assert!(!verified(&v, 8, "bindings"));
    let mut v = fixture();
    v["resource_changes"][2]["change"]["after"]["jurisdiction"] = Value::Null;
    v["resource_changes"][6]["change"]["after"]["bindings"][0]["jurisdiction"] = Value::Null;
    assert!(verified(&v, 6, "bindings"));
    let mut v = fixture();
    v["resource_changes"][2]["change"]["after"]["jurisdiction"] = json!("us");
    v["resource_changes"][8]["change"]["after"]["bindings"][0]["jurisdiction"] = json!("us");
    assert!(!verified(&v, 8, "bindings"));
    v["resource_changes"][6]["change"]["after"]["bindings"][0]["jurisdiction"] = json!("us");
    assert!(verified(&v, 6, "bindings"));
    let mut v = fixture();
    v["resource_changes"][3]["mode"] = json!("data");
    assert!(verified(&v, 6, "bindings"));
    v["configuration"]["root_module"]["resources"][6]["provider_config_key"] =
        json!("cloudflare.alias");
    v["configuration"]["provider_config"]["cloudflare.alias"] = json!({"name":"cloudflare","full_name":"registry.terraform.io/cloudflare/cloudflare","alias":"alias"});
    assert!(verified(&v, 6, "bindings"));
}
