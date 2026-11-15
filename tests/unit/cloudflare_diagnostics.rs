use crate::{
    model::{CloudflareFailure as Failure, DiagnosticReason, EntityMode},
    plan, semantic,
};
use serde_json::{Value, json};

fn fixture(name: &str) -> Value {
    serde_json::from_str(match name {
        "apps" => include_str!("../fixtures/cloudflare-applications-plan.json"),
        "edge" => include_str!("../fixtures/cloudflare-edge-plan.json"),
        "access" => include_str!("../fixtures/cloudflare-zero-trust-plan.json"),
        "data" => include_str!("../fixtures/cloudflare-captured/data.json"),
        _ => include_str!("../fixtures/cloudflare-dns-plan.json"),
    })
    .unwrap()
}

fn resource<'a>(value: &'a mut Value, address: &str) -> &'a mut Value {
    value["resource_changes"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|r| r["address"] == address)
        .unwrap()
}

fn configuration<'a>(value: &'a mut Value, address: &str) -> &'a mut Value {
    value["configuration"]["root_module"]["resources"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|r| r["address"] == address)
        .unwrap()
}

fn reason(value: &Value, address: &str, attribute: &str) -> Option<Failure> {
    let parsed = plan::parse(&value.to_string()).unwrap();
    let reference = parsed
        .attributes
        .iter()
        .find(|r| parsed.nodes[r.target].address == address && r.attribute == attribute)
        .unwrap();
    assert_eq!(
        reference.service_reference_verified,
        reference.cloudflare_failure.is_none()
    );
    reference.cloudflare_failure
}

#[test]
fn verified_representative_contracts_do_not_gain_failure_findings() {
    for name in ["dns", "apps", "edge", "access"] {
        let parsed = plan::parse(&fixture(name).to_string()).unwrap();
        assert!(
            semantic::diagnostics::collect_instances(&parsed)
                .iter()
                .all(|d| !matches!(d.reason, DiagnosticReason::Cloudflare(_))),
            "{name}"
        );
    }
}

#[test]
fn application_failures_distinguish_masks_scope_identity_outputs_and_lifecycle() {
    let consumer = "cloudflare_worker_version.api";
    let endpoint = "cloudflare_workers_kv_namespace.cache";
    for (path, replacement, expected) in [
        (
            "/change/after_sensitive/id",
            json!(true),
            Failure::SensitiveValue,
        ),
        (
            "/change/after_unknown/id",
            json!(true),
            Failure::UnknownValue,
        ),
        (
            "/change/after_unknown/id",
            json!("malformed"),
            Failure::InvalidValue,
        ),
        (
            "/change/after/account_id",
            json!("eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee"),
            Failure::ScopeMismatch,
        ),
        (
            "/change/after/account_id",
            Value::Null,
            Failure::ScopeUnavailable,
        ),
        (
            "/change/after/id",
            json!("different"),
            Failure::IdentityMismatch,
        ),
        (
            "/change/actions",
            json!(["delete"]),
            Failure::InactiveEndpoint,
        ),
    ] {
        let mut value = fixture("apps");
        let (parent, field) = path.rsplit_once('/').unwrap();
        resource(&mut value, endpoint).pointer_mut(parent).unwrap()[field] = replacement;
        assert_eq!(
            reason(&value, consumer, "bindings"),
            Some(expected),
            "{path}"
        );
    }
    let mut value = fixture("apps");
    let refs = &mut configuration(&mut value, consumer)["expressions"]["bindings"]["references"];
    for reference in refs.as_array_mut().unwrap() {
        if reference == "cloudflare_workers_kv_namespace.cache.id" {
            *reference = json!("cloudflare_workers_kv_namespace.cache.title");
        }
    }
    assert_eq!(
        reason(&value, consumer, "bindings"),
        Some(Failure::OutputMismatch)
    );
    let mut value = fixture("apps");
    configuration(&mut value, consumer)["expressions"]["bindings"]["references"] =
        json!(["cloudflare_r2_bucket.assets.name"]);
    assert_eq!(
        reason(&value, consumer, "bindings"),
        Some(Failure::IncompleteProvenance)
    );
    let mut value = fixture("apps");
    for binding in resource(&mut value, consumer)["change"]["after"]["bindings"]
        .as_array_mut()
        .unwrap()
    {
        binding["type"] = json!("plain_text");
    }
    assert_eq!(
        reason(&value, consumer, "bindings"),
        Some(Failure::UnsupportedContract)
    );
}

#[test]
fn collection_and_resource_reordering_choose_the_same_primary_failure() {
    let mut value = fixture("apps");
    let address = "cloudflare_worker_version.api";
    resource(&mut value, address)["change"]["after_unknown"] =
        json!({"bindings":[{}, {"namespace_id": true}, {}, {}, {}, {}, {}]});
    resource(&mut value, address)["change"]["after_sensitive"] =
        json!({"bindings":[{}, {}, {"id": true}, {}, {}, {}, {}]});
    assert_eq!(
        reason(&value, address, "bindings"),
        Some(Failure::SensitiveValue)
    );
    for field in ["after", "after_unknown", "after_sensitive"] {
        resource(&mut value, address)["change"][field]["bindings"]
            .as_array_mut()
            .unwrap()
            .reverse();
    }
    value["resource_changes"].as_array_mut().unwrap().reverse();
    value["configuration"]["root_module"]["resources"]
        .as_array_mut()
        .unwrap()
        .reverse();
    assert_eq!(
        reason(&value, address, "bindings"),
        Some(Failure::SensitiveValue)
    );

    let mut value = fixture("apps");
    let mut duplicate = resource(&mut value, "cloudflare_workers_kv_namespace.cache").clone();
    duplicate["address"] = json!("cloudflare_workers_kv_namespace.duplicate");
    value["resource_changes"]
        .as_array_mut()
        .unwrap()
        .push(duplicate);
    assert_eq!(
        reason(&value, address, "bindings"),
        Some(Failure::AmbiguousEndpoint)
    );
    value["resource_changes"].as_array_mut().unwrap().reverse();
    assert_eq!(
        reason(&value, address, "bindings"),
        Some(Failure::AmbiguousEndpoint)
    );
}

#[test]
fn masks_only_describe_fields_required_by_the_rejected_contract() {
    let mut value = fixture("dns");
    let address = "cloudflare_dns_record.www";
    resource(&mut value, address)["change"]["after_unknown"] = json!(true);
    resource(&mut value, address)["change"]["after_sensitive"] = json!({"content": true});
    assert_eq!(
        reason(&value, address, "zone_id"),
        Some(Failure::UnknownValue)
    );
    resource(&mut value, address)["change"]["after_sensitive"] = json!({"zone_id": true});
    assert_eq!(
        reason(&value, address, "zone_id"),
        Some(Failure::SensitiveValue)
    );
    let mut value = fixture("apps");
    resource(&mut value, "cloudflare_worker_version.api")["change"]["after_sensitive"] =
        json!({"modules": true, "bindings":[{}, {}, {"database_id":true}, {}, {}, {}, {}]});
    assert_eq!(
        reason(&value, "cloudflare_worker_version.api", "bindings"),
        None
    );
}

#[test]
fn administration_edge_access_and_deployment_failures_retain_typed_reasons() {
    let mut value = fixture("dns");
    resource(&mut value, "cloudflare_zone.example")["change"]["after_sensitive"] =
        json!({"id":true});
    assert_eq!(
        reason(&value, "cloudflare_dns_record.www", "zone_id"),
        Some(Failure::SensitiveValue)
    );

    let mut value = fixture("edge");
    resource(&mut value, "cloudflare_load_balancer.public")["change"]["after"]["default_pools"] =
        json!([
            "22222222222222222222222222222222",
            "22222222222222222222222222222222"
        ]);
    assert_eq!(
        reason(&value, "cloudflare_load_balancer.public", "default_pools"),
        Some(Failure::IncompleteCollection)
    );
    let mut value = fixture("edge");
    resource(&mut value, "cloudflare_list.redirects")["change"]["after"]["kind"] = json!("ip");
    assert_eq!(
        reason(&value, "cloudflare_ruleset.redirects", "rules"),
        Some(Failure::UnsupportedContract)
    );

    let mut value = fixture("access");
    resource(&mut value, "cloudflare_zero_trust_access_policy.staff")["change"]["after"]["account_id"] =
        json!("eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee");
    assert_eq!(
        reason(
            &value,
            "cloudflare_zero_trust_access_application.portal",
            "policies"
        ),
        Some(Failure::ScopeMismatch)
    );
    let mut value = fixture("access");
    resource(
        &mut value,
        "cloudflare_zero_trust_tunnel_cloudflared.office",
    )["change"]["after_unknown"] = json!({"id":true});
    assert_eq!(
        reason(
            &value,
            "cloudflare_zero_trust_tunnel_cloudflared_config.office",
            "tunnel_id"
        ),
        Some(Failure::UnknownValue)
    );

    let mut value = fixture("apps");
    resource(&mut value, "cloudflare_worker_version.api")["change"]["after_unknown"] =
        json!({"worker_id":true});
    assert_eq!(
        reason(&value, "cloudflare_workers_deployment.api", "versions"),
        Some(Failure::OwnerMismatch)
    );
}

#[test]
fn generic_resolution_failures_are_not_duplicated_by_cloudflare_findings() {
    let mut value = fixture("dns");
    configuration(&mut value, "cloudflare_dns_record.www")["expressions"]["zone_id"]["references"] =
        json!(["cloudflare_zone.missing.id"]);
    let parsed = plan::parse(&value.to_string()).unwrap();
    let diagnostics = semantic::diagnostics::collect_instances(&parsed);
    let matching: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.address == "cloudflare_dns_record.www" && d.attribute == "zone_id")
        .collect();
    assert!(!matching.is_empty());
    assert!(
        matching
            .iter()
            .all(|d| !matches!(d.reason, DiagnosticReason::Cloudflare(_)))
    );
    let graph = semantic::transform(&parsed);
    let reference = graph
        .nodes
        .iter()
        .find(|n| n.address == "cloudflare_dns_record.www")
        .unwrap()
        .references
        .iter()
        .find(|r| r.attribute == "zone_id")
        .unwrap();
    assert!(
        reference
            .issues
            .iter()
            .all(|d| !matches!(d, DiagnosticReason::Cloudflare(_)))
    );
}

#[test]
fn data_restoration_and_historical_consumers_keep_private_evidence_out_of_diagnostics() {
    let mut value = fixture("data");
    value["prior_state"]["values"]["root_module"]["resources"][0]["sensitive_values"] =
        json!({"id":true});
    assert_eq!(
        reason(&value, "cloudflare_dns_record.lookup", "zone_id"),
        Some(Failure::SensitiveValue)
    );
    let mut value = fixture("apps");
    resource(&mut value, "cloudflare_worker_version.api")["mode"] = json!("data");
    assert_eq!(
        reason(&value, "cloudflare_worker_version.api", "bindings"),
        Some(Failure::UnsupportedContract)
    );
    let mut value = fixture("dns");
    let mut old = resource(&mut value, "cloudflare_dns_record.www").clone();
    old["deposed"] = json!("old");
    old["change"]["actions"] = json!(["delete"]);
    value["resource_changes"].as_array_mut().unwrap().push(old);
    resource(&mut value, "cloudflare_dns_record.www")["change"]["actions"] = json!(["delete"]);
    assert_eq!(
        reason(&value, "cloudflare_dns_record.www", "zone_id"),
        Some(Failure::InactiveEndpoint)
    );
    let parsed = plan::parse(&value.to_string()).unwrap();
    let finding = semantic::diagnostics::collect_instances(&parsed)
        .into_iter()
        .find(|d| d.address == "cloudflare_dns_record.www" && d.attribute == "zone_id")
        .unwrap();
    assert_eq!(finding.deposed_key, None);
    assert!(
        semantic::diagnostics::collect_instances(&parsed)
            .iter()
            .all(
                |d| !matches!(d.reason, DiagnosticReason::Cloudflare(_)) || d.deposed_key.is_none()
            )
    );
    assert!(
        semantic::transform(&parsed)
            .nodes
            .iter()
            .any(|n| n.deposed_key.as_deref() == Some("old"))
    );
    assert_eq!(
        parsed
            .nodes
            .iter()
            .find(|n| n.address == finding.address)
            .unwrap()
            .mode,
        EntityMode::Managed
    );
    assert!(!format!("{parsed:?}").contains("PRIVATE_"));
}
