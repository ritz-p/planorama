use crate::model::EntityMode;
use serde_json::Value;
use std::fmt::Write;

fn endpoint(kind: &str, mode: EntityMode) -> &'static str {
    match kind {
        "cloudflare_account" => "Administration ID",
        "cloudflare_zone" if mode == EntityMode::Data => "Known matching id/zone_id",
        "cloudflare_zone" => "Zone ID",
        "cloudflare_load_balancer_pool"
        | "cloudflare_load_balancer_monitor"
        | "cloudflare_load_balancer_monitor_group" => "Account-scoped ID",
        "cloudflare_ruleset" => "ID with matching account and zone",
        "cloudflare_list" => "Redirect list name and kind, matching account",
        "cloudflare_worker" => "Account-scoped Worker ID or name (distinct outputs)",
        "cloudflare_worker_version" if mode == EntityMode::Managed => {
            "Version ID and explicitly proven Worker owner"
        }
        "cloudflare_workers_script" => "Known matching id/script_name, matching account",
        "cloudflare_pages_project" => "Known matching id/name, matching account",
        "cloudflare_r2_bucket" => "Known matching id/name, account and jurisdiction",
        "cloudflare_workers_kv_namespace" => "Account-scoped namespace ID",
        "cloudflare_d1_database" => "Known matching id/uuid, matching account",
        "cloudflare_zero_trust_access_identity_provider" => "ID with compatible account/zone scope",
        "cloudflare_zero_trust_access_policy"
        | "cloudflare_zero_trust_tunnel_cloudflared"
        | "cloudflare_zero_trust_tunnel_cloudflared_virtual_network" => "Account-scoped ID",
        _ => "",
    }
}
fn fields(entry: &Value, mode: EntityMode) -> Vec<String> {
    let kind = entry["type"].as_str().unwrap();
    entry["attributes"]
        .as_object()
        .unwrap()
        .iter()
        .filter_map(|(field, shape)| {
            if shape["required"] != true && shape["optional"] != true {
                return None;
            }
            let administration = super::administration::label(kind, field)
                .filter(|_| !(kind == "cloudflare_zone" && mode == EntityMode::Data));
            let label = administration.or_else(|| {
                if mode != EntityMode::Managed {
                    return None;
                }
                super::edge::label(kind, field)
                    .or_else(|| super::applications::label(kind, field))
                    .or_else(|| super::zero_trust::label(kind, field))
            });
            label.map(|label| format!("`{field}`: {label}"))
        })
        .collect()
}
fn level(kind: &str, mode: EntityMode, fields: &[String]) -> &'static str {
    if !super::known(kind, mode) {
        return "Unknown";
    }
    if mode == EntityMode::Managed
        && matches!(
            kind,
            "cloudflare_zone"
                | "cloudflare_dns_record"
                | "cloudflare_zone_dnssec"
                | "cloudflare_zero_trust_tunnel_cloudflared_config"
                | "cloudflare_zero_trust_tunnel_cloudflared_route"
        )
    {
        return "Dedicated";
    }
    if !fields.is_empty() || !endpoint(kind, mode).is_empty() {
        "Partial"
    } else {
        "Basic"
    }
}
pub(crate) fn matrix(filter: &str) -> String {
    if !filter.is_empty() && !filter.starts_with("cloudflare_") {
        return String::new();
    }
    let inventory: Value =
        serde_json::from_str(include_str!("../fixtures/cloudflare-inventory/5.28.1.json")).unwrap();
    let mut output = String::from(
        "\n## Cloudflare\n\nProvider source: `registry.terraform.io/cloudflare/cloudflare`. Pinned v5.28.1; 272 managed and 468 data types. [Contracts, evidence and update procedure](../docs/cloudflare.md).\n\nBasic means display only; Partial means the listed subset; Dedicated means supported documented reference contracts, not all API behavior. Unknown types or unproven provider identities receive no Cloudflare semantics. All links still require complete explicit provenance, known compatible scope and unique current endpoints. Data rows never acquire managed binding, rule or authorization semantics.\n\n| Resource type | Mode | Level | Role | Display | Service name | Conditional reference attributes | Verified endpoint contract |\n| --- | --- | --- | --- | --- | --- | --- | --- |\n",
    );
    for entry in inventory["resources"].as_array().unwrap() {
        let kind = entry["type"].as_str().unwrap();
        if !kind.starts_with(filter) {
            continue;
        }
        let mode = if entry["mode"] == "managed" {
            EntityMode::Managed
        } else {
            EntityMode::Data
        };
        let fields = fields(entry, mode);
        let level = level(kind, mode, &fields);
        let endpoint = endpoint(kind, mode);
        writeln!(
            output,
            "| `{kind}` | {} | {level} | {:?} | {:?} | {} | {} | {} |",
            entry["mode"].as_str().unwrap(),
            super::classify(kind, mode),
            super::placement(kind, mode),
            super::service_name(kind, mode).unwrap_or("Unknown"),
            if fields.is_empty() {
                "—".into()
            } else {
                fields.join("; ")
            },
            if endpoint.is_empty() { "—" } else { endpoint }
        )
        .unwrap();
    }
    output
}
#[test]
fn generated_capabilities_cover_inventory_modes_without_claiming_full_api_support() {
    let output = matrix("cloudflare_");
    assert_eq!(
        output
            .lines()
            .filter(|line| line.starts_with("| `cloudflare_"))
            .count(),
        740
    );
    assert!(output.contains("| `cloudflare_dns_record` | managed | Dedicated |"));
    assert!(output.contains("| `cloudflare_account_member` | managed | Basic |"));
    assert_eq!(
        level("cloudflare_invented", EntityMode::Managed, &[]),
        "Unknown"
    );
    assert!(matrix("aws_").is_empty());
    let workers = matrix("cloudflare_workers_script");
    let data = workers
        .lines()
        .find(|line| line.contains("| data |"))
        .unwrap();
    assert!(!data.contains("Storage bindings"));
    assert!(data.contains("id/script_name"));
    assert!(
        workers
            .lines()
            .any(|line| line.contains("| managed |") && line.contains("Storage bindings"))
    );
    let versions = matrix("cloudflare_worker_version");
    let data = versions
        .lines()
        .find(|line| line.contains("| data |"))
        .unwrap();
    assert!(!data.contains("Version ID and explicitly proven Worker owner"));
    assert!(!data.contains("Storage bindings"));
}
