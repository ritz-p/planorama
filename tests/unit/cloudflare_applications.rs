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
                || (r.attribute == path
                    && r.complete
                    && path == "bindings"
                    && matches!(r.label, "Storage bindings" | "Service bindings"))
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
    assert_eq!(graph.nodes.len(), 17);
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
        json!({"bindings":[{}, {"namespace_id":true}, {}, {}, {}, {}, {}, {}]});
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
            "cloudflare_d1_database.content.uuid",
            "cloudflare_worker.backend.name",
            "cloudflare_workers_script.downstream.id",
            "cloudflare_hyperdrive_config.database.id"
        ]);
    assert!(verified(&v, 6, "bindings"));
    v["resource_changes"][6]["change"]["after_unknown"] =
        json!({"bindings":[{}, {}, {"database_id":true}, {}, {}, {}, {}, {}]});
    assert!(verified(&v, 6, "bindings"));
    let mut v = fixture();
    v["resource_changes"][8]["change"]["after_unknown"] =
        json!({"bindings":[{}, {}, {"database_id":true}, {}, {}, {}, {}, {}]});
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

#[test]
fn service_and_storage_roles_split_only_after_complete_collection_verification() {
    let v = fixture();
    let graph = semantic::transform(&plan::parse(&v.to_string()).unwrap());
    for index in [6, 8] {
        let node = graph
            .nodes
            .iter()
            .find(|n| n.address == v["resource_changes"][index]["address"])
            .unwrap();
        let refs: Vec<_> = node
            .references
            .iter()
            .filter(|r| r.attribute == "bindings" && !r.dependency)
            .collect();
        assert_eq!(refs.len(), 3);
        for (label, count) in [
            ("Storage bindings", 3),
            ("Service bindings", 2),
            ("Hyperdrive bindings", 1),
        ] {
            let r = refs.iter().find(|r| r.label == label).unwrap();
            assert!(r.complete);
            assert_eq!(r.targets.len(), count);
        }
    }
    let mut missing = v.clone();
    missing["configuration"]["root_module"]["resources"][6]["expressions"]["bindings"]["references"] =
        json!(["cloudflare_workers_kv_namespace.cache.id"]);
    let graph = semantic::transform(&plan::parse(&missing.to_string()).unwrap());
    let node = graph
        .nodes
        .iter()
        .find(|n| n.address == "cloudflare_worker_version.api")
        .unwrap();
    assert!(
        node.references
            .iter()
            .filter(|r| r.attribute == "bindings" && !r.dependency)
            .all(|r| !r.complete && r.targets.is_empty())
    );
}

#[test]
fn service_bindings_require_known_unqualified_names_and_compatible_current_endpoints() {
    for field in ["service", "environment", "entrypoint"] {
        for mask in ["after_unknown", "after_sensitive"] {
            let mut v = fixture();
            v["resource_changes"][6]["change"][mask] =
                json!({"bindings":[{},{},{},{},{field:true},{},{}, {}]});
            assert!(!verified(&v, 6, "bindings"), "{mask}/{field}");
        }
    }
    for environment in [json!("production"), json!("staging"), json!("")] {
        let mut v = fixture();
        v["resource_changes"][6]["change"]["after"]["bindings"][4]["environment"] = environment;
        assert!(!verified(&v, 6, "bindings"));
    }
    for (field, value) in [
        ("account_id", json!("eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee")),
        ("name", json!("other")),
    ] {
        let mut v = fixture();
        v["resource_changes"][13]["change"]["after"][field] = value;
        assert!(!verified(&v, 6, "bindings"));
    }
    let mut v = fixture();
    v["resource_changes"][14]["change"]["after"]["id"] = json!("conflicting-alias");
    assert!(!verified(&v, 6, "bindings"));
    let mut v = fixture();
    v["resource_changes"][13]["change"]["actions"] = json!(["delete"]);
    assert!(!verified(&v, 6, "bindings"));
    let mut v = fixture();
    let mut duplicate = v["resource_changes"][13].clone();
    duplicate["address"] = json!("cloudflare_worker.duplicate");
    duplicate["name"] = json!("duplicate");
    v["resource_changes"]
        .as_array_mut()
        .unwrap()
        .push(duplicate);
    assert!(!verified(&v, 6, "bindings"));
    v["resource_changes"][17]["deposed"] = json!("old");
    v["resource_changes"][17]["change"]["actions"] = json!(["delete"]);
    assert!(verified(&v, 6, "bindings"));
}

#[test]
fn service_bindings_do_not_treat_worker_uuid_as_name_and_support_singleton_data_names() {
    let mut v = fixture();
    let refs = &mut v["configuration"]["root_module"]["resources"][6]["expressions"]["bindings"]["references"];
    for r in refs.as_array_mut().unwrap() {
        if r == "cloudflare_worker.backend.name" {
            *r = json!("cloudflare_worker.backend.id");
        }
    }
    assert!(!verified(&v, 6, "bindings"));
    let mut v = fixture();
    for path in [
        "/resource_changes/13",
        "/configuration/root_module/resources/13",
    ] {
        let r = v.pointer_mut(path).unwrap();
        r["mode"] = json!("data");
        r["address"] = json!("data.cloudflare_worker.backend");
    }
    for r in
        v["configuration"]["root_module"]["resources"][6]["expressions"]["bindings"]["references"]
            .as_array_mut()
            .unwrap()
    {
        if r == "cloudflare_worker.backend.name" {
            *r = json!("data.cloudflare_worker.backend.name");
        } else if r == "cloudflare_worker.backend" {
            *r = json!("data.cloudflare_worker.backend");
        }
    }
    assert!(verified(&v, 6, "bindings"));
    v["resource_changes"][6]["mode"] = json!("data");
    assert!(!verified(&v, 6, "bindings"));
}

#[test]
fn dns_pages_cname_requires_typed_scoped_current_explicit_evidence() {
    let base = fixture();
    assert!(verified(&base, 15, "content"));
    for (path, value) in [
        ("/resource_changes/15/change/after/type", json!("A")),
        (
            "/resource_changes/15/change/after/content",
            json!("other.pages.dev"),
        ),
        (
            "/resource_changes/15/change/after_unknown/content",
            json!(true),
        ),
        (
            "/resource_changes/15/change/after_sensitive/content",
            json!(true),
        ),
        (
            "/resource_changes/15/change/after_unknown/proxied",
            json!(true),
        ),
        (
            "/resource_changes/11/change/after/account_id",
            json!("ffffffffffffffffffffffffffffffff"),
        ),
        (
            "/resource_changes/11/change/after_unknown/subdomain",
            json!(true),
        ),
        (
            "/resource_changes/11/change/after_sensitive/subdomain",
            json!(true),
        ),
        ("/resource_changes/11/change/actions", json!(["delete"])),
        ("/resource_changes/15/mode", json!("data")),
        (
            "/configuration/root_module/resources/15/expressions/content/references",
            json!([]),
        ),
        (
            "/configuration/root_module/resources/15/expressions/content/references",
            json!(["cloudflare_pages_project.site.name"]),
        ),
        (
            "/configuration/root_module/resources/15/expressions/zone_id/references",
            json!([]),
        ),
    ] {
        let mut v = base.clone();
        let (parent, key) = path.rsplit_once('/').unwrap();
        v.pointer_mut(parent).unwrap()[key] = value;
        assert!(!verified(&v, 15, "content"), "{path}");
    }
    let mut v = base.clone();
    v["resource_changes"][15]["change"]["after"]["proxied"] = json!(false);
    assert!(verified(&v, 15, "content"));
    let mut duplicate = v["resource_changes"][11].clone();
    duplicate["address"] = json!("cloudflare_pages_project.duplicate");
    v["resource_changes"]
        .as_array_mut()
        .unwrap()
        .push(duplicate);
    assert!(!verified(&v, 15, "content"));
    v["resource_changes"][17]["deposed"] = json!("old");
    assert!(verified(&v, 15, "content"));
    let mut v = base;
    for collection in ["resource_changes", "configuration/root_module/resources"] {
        let row = v.pointer_mut(&format!("/{collection}/11")).unwrap();
        row["address"] = json!("data.cloudflare_pages_project.site");
        row["mode"] = json!("data");
    }
    v["configuration"]["root_module"]["resources"][15]["expressions"]["content"]["references"] =
        json!(["data.cloudflare_pages_project.site.subdomain"]);
    assert!(verified(&v, 15, "content"));
}

#[test]
fn hyperdrive_bindings_require_current_scoped_id_and_never_publish_origin_secrets() {
    let base = fixture();
    for consumer in [6, 8] {
        assert!(verified(&base, consumer, "bindings"));
        for (section, field, value) in [
            ("after", "id", json!("other")),
            (
                "after",
                "account_id",
                json!("ffffffffffffffffffffffffffffffff"),
            ),
            ("after_unknown", "id", json!(true)),
            ("after_sensitive", "id", json!(true)),
            ("after_sensitive", "account_id", json!(true)),
        ] {
            let mut v = base.clone();
            v["resource_changes"][16]["change"][section][field] = value;
            assert!(
                !verified(&v, consumer, "bindings"),
                "{consumer}/{section}/{field}"
            );
        }
        let mut v = base.clone();
        v["resource_changes"][consumer]["change"]["after_unknown"]["bindings"] =
            json!([{}, {}, {}, {}, {}, {}, {}, {"id":true}]);
        assert!(!verified(&v, consumer, "bindings"));
        let mut v = base.clone();
        v["resource_changes"][16]["change"]["actions"] = json!(["delete"]);
        assert!(!verified(&v, consumer, "bindings"));
        let mut v = base.clone();
        let mut duplicate = v["resource_changes"][16].clone();
        duplicate["address"] = json!("cloudflare_hyperdrive_config.duplicate");
        v["resource_changes"]
            .as_array_mut()
            .unwrap()
            .push(duplicate);
        assert!(!verified(&v, consumer, "bindings"));
    }
    let g = semantic::transform(&plan::parse(&base.to_string()).unwrap());
    assert!(!format!("{g:?}").contains("PRIVATE_"));
    assert!(g.nodes.iter().any(|n| {
        n.references
            .iter()
            .any(|r| r.label == "Hyperdrive bindings" && r.complete)
    }));
}
