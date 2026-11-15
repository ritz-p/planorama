use crate::{model::CloudflareFailure as Failure, plan, semantic};
use serde_json::{Value, json};
const FIXTURE: &str = include_str!("../fixtures/cloudflare-queues-plan.json");
fn fixture() -> Value {
    serde_json::from_str(FIXTURE).unwrap()
}
fn change<'a>(v: &'a mut Value, address: &str) -> &'a mut Value {
    &mut v["resource_changes"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|r| r["address"] == address)
        .unwrap()["change"]
}
fn expressions<'a>(v: &'a mut Value, address: &str) -> &'a mut Value {
    &mut v["configuration"]["root_module"]["resources"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|r| r["address"] == address)
        .unwrap()["expressions"]
}
fn result(v: &Value, address: &str, path: &str) -> Result<(), Failure> {
    let plan = plan::parse(&v.to_string()).unwrap();
    let r = plan
        .attributes
        .iter()
        .find(|r| plan.nodes[r.target].address == address && r.attribute == path)
        .unwrap();
    if r.service_reference_verified {
        Ok(())
    } else {
        Err(r.cloudflare_failure.unwrap())
    }
}
const PRODUCER: &str = "cloudflare_worker_version.backend";
const CONSUMER: &str = "cloudflare_queue_consumer.worker";
const QUEUE: &str = "cloudflare_queue.jobs";
#[test]
fn mixed_producers_and_typed_consumers_preserve_roles_and_lifecycles() {
    let v = fixture();
    for address in [PRODUCER, "cloudflare_workers_script.legacy"] {
        assert_eq!(result(&v, address, "bindings"), Ok(()));
    }
    for name in ["worker", "legacy", "pull", "notification"] {
        assert_eq!(
            result(&v, &format!("cloudflare_queue_consumer.{name}"), "queue_id"),
            Ok(())
        );
    }
    for name in ["worker", "legacy"] {
        assert_eq!(
            result(
                &v,
                &format!("cloudflare_queue_consumer.{name}"),
                "script_name"
            ),
            Ok(())
        );
    }
    let graph = semantic::transform(&plan::parse(FIXTURE).unwrap());
    assert_eq!(graph.nodes.len(), 13);
    assert!(!format!("{graph:?}").contains("PRIVATE_"));
    for address in [PRODUCER, "cloudflare_workers_script.legacy"] {
        let node = graph.nodes.iter().find(|n| n.address == address).unwrap();
        let rows: Vec<_> = node
            .references
            .iter()
            .filter(|r| r.attribute == "bindings" && !r.dependency)
            .collect();
        assert_eq!(rows.len(), 3);
        for (label, count) in [
            ("Storage bindings", 1),
            ("Service bindings", 1),
            ("Queue producer bindings", 2),
        ] {
            let row = rows.iter().find(|r| r.label == label).unwrap();
            assert!(row.complete);
            assert_eq!(row.targets.len(), count);
        }
    }
}
#[test]
fn producer_names_and_consumer_ids_require_their_own_outputs() {
    for output in ["id", "queue_id"] {
        let mut v = fixture();
        let refs = expressions(&mut v, PRODUCER)["bindings"]["references"]
            .as_array_mut()
            .unwrap();
        for r in refs {
            if r == "cloudflare_queue.jobs.queue_name" {
                *r = json!(format!("{QUEUE}.{output}"));
            }
        }
        assert_eq!(
            result(&v, PRODUCER, "bindings"),
            Err(Failure::OutputMismatch)
        );
    }
    for (field, value) in [("queue_id", json!("jobs")), ("queue_id", json!("missing"))] {
        let mut v = fixture();
        change(&mut v, CONSUMER)["after"][field] = value;
        assert_eq!(
            result(&v, CONSUMER, "queue_id"),
            Err(Failure::IdentityMismatch)
        );
    }
    let mut v = fixture();
    expressions(&mut v, CONSUMER)["queue_id"] =
        json!({"references":["cloudflare_queue.jobs.queue_name"]});
    assert_eq!(
        result(&v, CONSUMER, "queue_id"),
        Err(Failure::OutputMismatch)
    );
    let mut v = fixture();
    change(&mut v, QUEUE)["after"]["id"] = json!("conflicting");
    assert_eq!(
        result(&v, CONSUMER, "queue_id"),
        Err(Failure::IdentityMismatch)
    );
    let mut v = fixture();
    expressions(&mut v, CONSUMER)["script_name"] =
        json!({"references":["cloudflare_worker.backend.id"]});
    assert_eq!(
        result(&v, CONSUMER, "script_name"),
        Err(Failure::OutputMismatch)
    );
}
#[test]
fn consumer_discriminators_do_not_infer_workers_from_pull_or_notification() {
    for kind in ["http_pull", "notification", "unsupported"] {
        let mut v = fixture();
        change(&mut v, CONSUMER)["after"]["type"] = json!(kind);
        assert_eq!(
            result(&v, CONSUMER, "script_name"),
            Err(Failure::UnsupportedContract)
        );
        assert_eq!(
            result(&v, CONSUMER, "queue_id").is_ok(),
            kind != "unsupported"
        );
    }
    for (mask, failure) in [
        ("after_unknown", Failure::UnknownValue),
        ("after_sensitive", Failure::SensitiveValue),
    ] {
        let mut v = fixture();
        change(&mut v, CONSUMER)[mask] = json!({"type":true});
        for path in ["queue_id", "script_name"] {
            assert_eq!(result(&v, CONSUMER, path), Err(failure));
        }
    }
    let mut v = fixture();
    expressions(&mut v, CONSUMER)
        .as_object_mut()
        .unwrap()
        .remove("script_name");
    let plan = plan::parse(&v.to_string()).unwrap();
    assert!(
        !plan
            .attributes
            .iter()
            .any(|r| plan.nodes[r.target].address == CONSUMER
                && r.attribute == "script_name"
                && r.service_reference_verified)
    );
}
#[test]
fn queue_masks_and_incomplete_mixed_provenance_never_promote_a_subset() {
    for (mask, failure) in [
        ("after_unknown", Failure::UnknownValue),
        ("after_sensitive", Failure::SensitiveValue),
    ] {
        for (address, field, target, path) in [
            (QUEUE, "queue_name", PRODUCER, "bindings"),
            (QUEUE, "queue_id", CONSUMER, "queue_id"),
            (QUEUE, "id", CONSUMER, "queue_id"),
            (CONSUMER, "queue_id", CONSUMER, "queue_id"),
            (CONSUMER, "script_name", CONSUMER, "script_name"),
        ] {
            let mut v = fixture();
            change(&mut v, address)[mask] = json!({field:true});
            assert_eq!(
                result(&v, target, path),
                Err(failure),
                "{mask}/{address}/{field}"
            );
        }
        let mut v = fixture();
        change(&mut v, PRODUCER)[mask] = json!({"bindings":[{},{},{"queue_name":true},{},{}]});
        assert_eq!(result(&v, PRODUCER, "bindings"), Err(failure));
    }
    let mut v = fixture();
    expressions(&mut v, PRODUCER)["bindings"] = json!({"references":["cloudflare_workers_kv_namespace.cache.id","cloudflare_worker.backend.name","cloudflare_queue.jobs.queue_name"]});
    assert_eq!(
        result(&v, PRODUCER, "bindings"),
        Err(Failure::IncompleteProvenance)
    );
    let graph = semantic::transform(&plan::parse(&v.to_string()).unwrap());
    let node = graph.nodes.iter().find(|n| n.address == PRODUCER).unwrap();
    let rows: Vec<_> = node
        .references
        .iter()
        .filter(|r| r.attribute == "bindings" && !r.dependency)
        .collect();
    assert_eq!(rows.len(), 1);
    assert!(!rows[0].complete);
    assert!(rows[0].targets.is_empty());
    let mut v = fixture();
    change(&mut v, PRODUCER)["after_unknown"] = json!({"bindings":[{},{}]});
    assert!(result(&v, PRODUCER, "bindings").is_err());
}
#[test]
fn queue_scope_ambiguity_and_current_instance_checks_are_typed() {
    let mut v = fixture();
    change(&mut v, QUEUE)["after"]["account_id"] = json!("e".repeat(32));
    for (target, path) in [(PRODUCER, "bindings"), (CONSUMER, "queue_id")] {
        assert_eq!(result(&v, target, path), Err(Failure::ScopeMismatch));
    }
    let mut v = fixture();
    let mut duplicate = v["resource_changes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["address"] == QUEUE)
        .unwrap()
        .clone();
    duplicate["address"] = json!("cloudflare_queue.duplicate");
    duplicate["name"] = json!("duplicate");
    v["resource_changes"]
        .as_array_mut()
        .unwrap()
        .push(duplicate);
    for (target, path) in [(PRODUCER, "bindings"), (CONSUMER, "queue_id")] {
        assert_eq!(result(&v, target, path), Err(Failure::AmbiguousEndpoint));
    }
    v["resource_changes"]
        .as_array_mut()
        .unwrap()
        .last_mut()
        .unwrap()["change"]["after"]["queue_name"] = json!("other-name");
    assert_eq!(result(&v, PRODUCER, "bindings"), Ok(()));
    assert_eq!(
        result(&v, CONSUMER, "queue_id"),
        Err(Failure::AmbiguousEndpoint)
    );
    v["resource_changes"]
        .as_array_mut()
        .unwrap()
        .last_mut()
        .unwrap()["deposed"] = json!("old");
    assert_eq!(result(&v, CONSUMER, "queue_id"), Ok(()));
    change(&mut v, QUEUE)["actions"] = json!(["delete"]);
    assert!(result(&v, CONSUMER, "queue_id").is_err());
    change(&mut v, QUEUE)["actions"] = json!(["delete", "create"]);
    assert_eq!(result(&v, CONSUMER, "queue_id"), Ok(()));
}
#[test]
fn singleton_data_queues_are_endpoints_and_data_consumers_are_not_managed() {
    let mut v = fixture();
    for section in ["resource_changes", "configuration"] {
        let resources = if section == "resource_changes" {
            &mut v[section]
        } else {
            &mut v[section]["root_module"]["resources"]
        };
        let r = resources
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|r| r["address"] == QUEUE)
            .unwrap();
        r["address"] = json!(format!("data.{QUEUE}"));
        r["mode"] = json!("data");
    }
    for r in v["configuration"]["root_module"]["resources"]
        .as_array_mut()
        .unwrap()
    {
        for expr in r["expressions"].as_object_mut().unwrap().values_mut() {
            if let Some(refs) = expr["references"].as_array_mut() {
                for r in refs {
                    if r.as_str().unwrap().starts_with(QUEUE) {
                        *r = json!(format!("data.{}", r.as_str().unwrap()));
                    }
                }
            }
        }
    }
    assert_eq!(result(&v, PRODUCER, "bindings"), Ok(()));
    assert_eq!(result(&v, CONSUMER, "queue_id"), Ok(()));
    let r = v["resource_changes"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|r| r["address"] == CONSUMER)
        .unwrap();
    r["mode"] = json!("data");
    assert_eq!(
        result(&v, CONSUMER, "queue_id"),
        Err(Failure::UnsupportedContract)
    );
}
