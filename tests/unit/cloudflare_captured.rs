use crate::{
    model::{Action, ArchitectureGraph, EdgeKind, EntityMode, Node, ResourceReference},
    plan, semantic,
};

const CREATE: &str = include_str!("../fixtures/cloudflare-captured/create.json");
const UPDATE: &str = include_str!("../fixtures/cloudflare-captured/update.json");
const REPLACE: &str = include_str!("../fixtures/cloudflare-captured/replace.json");
const DATA: &str = include_str!("../fixtures/cloudflare-captured/data.json");

fn node<'a>(graph: &'a ArchitectureGraph, address: &str) -> &'a Node {
    graph.nodes.iter().find(|n| n.address == address).unwrap()
}

fn reference<'a>(
    graph: &'a ArchitectureGraph,
    address: &str,
    attribute: &str,
) -> &'a ResourceReference {
    node(graph, address)
        .references
        .iter()
        .find(|r| r.attribute == attribute && !r.dependency)
        .unwrap()
}

#[test]
fn initial_plan_keeps_provenance_without_claiming_unknown_identities() {
    let parsed = plan::parse(CREATE).unwrap();
    for (consumer, attribute, sources) in [
        ("cloudflare_dns_record.www", "zone_id", 1),
        ("cloudflare_worker_version.api", "worker_id", 1),
        ("cloudflare_worker_version.api", "bindings", 3),
    ] {
        let proof = parsed
            .attributes
            .iter()
            .find(|r| parsed.nodes[r.target].address == consumer && r.attribute == attribute)
            .unwrap();
        assert_eq!(proof.sources.len(), sources);
        assert!(proof.complete);
        assert!(!proof.service_reference_verified);
        let graph = semantic::transform(&parsed);
        let shown = reference(&graph, consumer, attribute);
        assert!(!shown.complete);
        assert!(shown.targets.is_empty());
    }
    let graph = semantic::transform(&parsed);
    assert_eq!(graph.nodes.len(), 7);
    assert!(graph.nodes.iter().all(|n| n.action == Action::Create));
    assert!(!graph.edges.is_empty());
    assert!(graph.edges.iter().all(|e| e.kind == EdgeKind::Dependency));
}

#[test]
fn update_verifies_known_zone_worker_and_complete_storage_collection() {
    let graph = semantic::transform(&plan::parse(UPDATE).unwrap());
    assert_eq!(
        node(&graph, "cloudflare_dns_record.www").action,
        Action::Update
    );
    assert_eq!(
        node(&graph, "cloudflare_workers_kv_namespace.cache").action,
        Action::Update
    );
    assert_eq!(
        node(&graph, "cloudflare_zone.example").action,
        Action::Unchanged
    );
    for (address, attribute, targets) in [
        ("cloudflare_dns_record.www", "zone_id", 1),
        ("cloudflare_worker_version.api", "worker_id", 1),
        ("cloudflare_worker_version.api", "bindings", 3),
    ] {
        let shown = reference(&graph, address, attribute);
        assert!(shown.complete, "{address}.{attribute}");
        assert_eq!(shown.targets.len(), targets);
    }
    assert_eq!(
        node(&graph, "cloudflare_dns_record.www")
            .entity
            .cloudflare_scope
            .as_ref()
            .unwrap()
            .account
            .as_deref(),
        Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")
    );
}

#[test]
fn replacement_does_not_reuse_prior_kv_identity_for_new_bindings() {
    let graph = semantic::transform(&plan::parse(REPLACE).unwrap());
    assert_eq!(
        node(&graph, "cloudflare_workers_kv_namespace.cache").action,
        Action::Replace
    );
    let bindings = reference(&graph, "cloudflare_worker_version.api", "bindings");
    assert!(!bindings.complete);
    assert!(bindings.targets.is_empty());
    assert!(reference(&graph, "cloudflare_worker_version.api", "worker_id").complete);
    assert!(reference(&graph, "cloudflare_dns_record.www", "zone_id").complete);
}

#[test]
fn provider_read_data_is_restored_but_deferred_lookup_stays_unverified() {
    let graph = semantic::transform(&plan::parse(DATA).unwrap());
    assert_eq!(graph.nodes.len(), 11);
    let existing = node(&graph, "data.cloudflare_zone.existing");
    assert_eq!(existing.mode, EntityMode::Data);
    assert_eq!(existing.action, Action::Unchanged);
    assert_eq!(
        node(&graph, "data.cloudflare_zone.deferred").action,
        Action::Read
    );
    let known = reference(&graph, "cloudflare_dns_record.lookup", "zone_id");
    assert!(known.complete);
    assert_eq!(known.targets.len(), 1);
    let unknown = reference(&graph, "cloudflare_dns_record.deferred", "zone_id");
    assert!(!unknown.complete);
    assert!(unknown.targets.is_empty());
}

#[test]
fn captured_payloads_do_not_survive_in_architecture_models() {
    for input in [CREATE, UPDATE, REPLACE, DATA] {
        let graph = semantic::transform(&plan::parse(input).unwrap());
        let debug = format!("{graph:?}");
        for payload in [
            "192.0.2.",
            "export default",
            "ZXhwb3J0IGRlZmF1bHQ",
            "fixture-only-not-a-real-token",
        ] {
            assert!(!debug.contains(payload), "{payload}");
        }
        assert!(graph.nodes.iter().all(|n| n.entity.scope.is_none()));
        assert!(graph.edges.iter().all(|e| e.kind == EdgeKind::Dependency));
    }
}
