use super::target::{Kind, Target, jurisdiction};
use super::workers::service;
use super::{Evidence, Failure, Result, collect};
use serde_json::Value;
pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    match (kind, path) {
        ("cloudflare_pages_domain", "project_name") => Some("Pages project"),
        ("cloudflare_pages_project", "deployment_configs") => Some("Storage and service bindings"),
        _ => None,
    }
}
fn optional<'a>(value: Evidence<'a>, path: &str) -> Result<Option<Evidence<'a>>> {
    let field = value.at(path)?;
    if field.value.is_null() && matches!(field.mask, Value::Null | Value::Bool(false)) {
        Ok(None)
    } else if !field.value.is_null() {
        Ok(Some(field))
    } else {
        field.scalar()?;
        Err(Failure::InvalidValue)
    }
}
pub(super) fn targets(value: Evidence<'_>) -> Result<Vec<Target<'_>>> {
    Ok(
        collect(["preview", "production"].into_iter().map(|environment| {
            let Some(env) = optional(value, environment)? else {
                return Ok(Vec::new());
            };
            Ok(collect(
                [
                    ("d1_databases", "id", Kind::D1),
                    ("kv_namespaces", "namespace_id", Kind::Kv),
                    ("r2_buckets", "name", Kind::R2),
                    ("services", "service", Kind::WorkerName),
                ]
                .into_iter()
                .map(|(field, identity, kind)| {
                    let Some(map) = optional(env, field)? else {
                        return Ok(Vec::new());
                    };
                    collect(map.map()?.into_iter().map(|(_, row)| {
                        if matches!(kind, Kind::WorkerName) {
                            return service(row);
                        }
                        Ok(Target {
                            value: row.text(identity)?,
                            kind,
                            jurisdiction: if matches!(kind, Kind::R2) {
                                Some(jurisdiction(row)?)
                            } else {
                                None
                            },
                        })
                    }))
                }),
            )?
            .into_iter()
            .flatten()
            .collect())
        }))?
        .into_iter()
        .flatten()
        .collect(),
    )
}
