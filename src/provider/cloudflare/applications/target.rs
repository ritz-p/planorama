use super::{Evidence, Failure, Result, collect};
use crate::model::TerraformEntity;
#[derive(Clone, Copy)]
pub(super) enum Kind {
    Worker,
    WorkerName,
    DurableObjectScript,
    Version,
    Pages,
    R2,
    Kv,
    D1,
    QueueId,
    QueueName,
    Hyperdrive,
}
pub(super) struct Target<'a> {
    pub(super) value: &'a str,
    pub(super) kind: Kind,
    pub(super) jurisdiction: Option<&'a str>,
}
impl<'a> Target<'a> {
    pub(super) fn new(value: &'a str, kind: Kind) -> Self {
        Self {
            value,
            kind,
            jurisdiction: None,
        }
    }
}
pub(super) fn jurisdiction(value: Evidence<'_>) -> Result<&str> {
    let field = value.at("jurisdiction")?;
    field.scalar()?;
    match field.value.as_str() {
        None if field.value.is_null() => Ok("default"),
        Some(value @ ("default" | "eu" | "fedramp" | "us" | "fedramp-high")) => Ok(value),
        _ => Err(Failure::UnsupportedContract),
    }
}
pub(super) fn fields(
    target: &Target<'_>,
    node: &TerraformEntity,
) -> Result<&'static [&'static str]> {
    Ok(match (target.kind, node.resource_type.as_str()) {
        (Kind::Worker, "cloudflare_worker") => &["id", "name"],
        (Kind::WorkerName | Kind::DurableObjectScript, "cloudflare_worker") => &["name"],
        (Kind::WorkerName | Kind::DurableObjectScript, "cloudflare_workers_script") => {
            &["id", "script_name"]
        }
        (Kind::Version, "cloudflare_worker_version") => &["id"],
        (Kind::Pages, "cloudflare_pages_project") => &["id", "name"],
        (Kind::R2, "cloudflare_r2_bucket") => &["id", "name"],
        (Kind::Kv, "cloudflare_workers_kv_namespace") => &["id"],
        (Kind::D1, "cloudflare_d1_database") => &["id", "uuid"],
        (Kind::QueueId, "cloudflare_queue") => &["id", "queue_id"],
        (Kind::QueueName, "cloudflare_queue") => &["queue_name"],
        (Kind::Hyperdrive, "cloudflare_hyperdrive_config") => &["id"],
        _ => return Err(Failure::UnsupportedContract),
    })
}
pub(super) fn identity(
    target: &Target<'_>,
    node: &TerraformEntity,
    value: Evidence<'_>,
    output: &str,
) -> Result<()> {
    let fields = fields(target, node)?;
    if !fields.contains(&output) {
        return Err(Failure::OutputMismatch);
    }
    if value.text(output)? != target.value {
        return Err(Failure::IdentityMismatch);
    }
    if !matches!(target.kind, Kind::Worker)
        && collect(fields.iter().map(|field| value.text(field)))?
            .iter()
            .any(|id| *id != target.value)
    {
        return Err(Failure::IdentityMismatch);
    }
    if matches!(target.kind, Kind::R2) && Some(jurisdiction(value)?) != target.jurisdiction {
        return Err(Failure::IdentityMismatch);
    }
    Ok(())
}
pub(super) fn matching(target: &Target<'_>, node: &TerraformEntity, value: Evidence<'_>) -> bool {
    [
        "id",
        "name",
        "script_name",
        "uuid",
        "queue_id",
        "queue_name",
    ]
    .iter()
    .any(|output| identity(target, node, value, output).is_ok())
}
