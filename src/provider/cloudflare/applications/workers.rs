use super::target::{Kind, Target, jurisdiction};
use super::{Evidence, Failure, Result, collect, proof};
pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    match (kind, path) {
        ("cloudflare_workers_script" | "cloudflare_worker_version", "bindings") => {
            Some("Worker bindings")
        }
        ("cloudflare_workers_route", "script")
        | ("cloudflare_workers_custom_domain", "service") => Some("Configured Worker"),
        _ => None,
    }
}
pub(super) fn bindings(value: Evidence<'_>, version: bool) -> Result<Vec<Target<'_>>> {
    Ok(collect(value.rows()?.into_iter().map(|row| {
        let target = match row.text("type")? {
            "durable_object_namespace" => {
                row.at("environment")?.require_null()?;
                row.at("dispatch_namespace")?.require_null()?;
                row.text("class_name")?;
                let script = row.at("script_name")?;
                script.scalar()?;
                if script.value.is_null() {
                    return Ok(None);
                }
                Target::new(row.text("script_name")?, Kind::DurableObjectScript)
            }
            "service" => service(row)?,
            "queue" => Target::new(row.text("queue_name")?, Kind::QueueName),
            "hyperdrive" => Target::new(row.text("id")?, Kind::Hyperdrive),
            "kv_namespace" => Target::new(row.text("namespace_id")?, Kind::Kv),
            "r2_bucket" => {
                let jurisdiction = jurisdiction(row)?;
                if !version && jurisdiction == "us" {
                    return Err(Failure::UnsupportedContract);
                }
                Target {
                    value: row.text("bucket_name")?,
                    kind: Kind::R2,
                    jurisdiction: Some(jurisdiction),
                }
            }
            "d1" => {
                let id = row.text("id")?;
                let alias = row.at("database_id")?;
                if let Ok(alias) = row.text("database_id") {
                    if alias != id {
                        return Err(Failure::IdentityMismatch);
                    }
                } else if !alias.value.is_null() || (!version && !proof::unmasked(alias.mask)) {
                    alias.scalar()?;
                    return Err(Failure::IdentityMismatch);
                }
                Target::new(id, Kind::D1)
            }
            _ => return Ok(None),
        };
        Ok(Some(target))
    }))?
    .into_iter()
    .flatten()
    .collect())
}
pub(super) fn service(row: Evidence<'_>) -> Result<Target<'_>> {
    row.at("environment")?.require_null()?;
    let entrypoint = row.at("entrypoint")?;
    entrypoint.scalar()?;
    if !entrypoint.value.is_null() {
        row.text("entrypoint")?;
    }
    Ok(Target::new(row.text("service")?, Kind::WorkerName))
}
