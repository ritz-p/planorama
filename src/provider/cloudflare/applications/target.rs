use super::{Evidence, Failure, Result, collect};
use crate::model::TerraformEntity;
#[derive(Clone, Copy)]
pub(super) enum Kind<'a> {
    Worker,
    WorkerName,
    DurableObjectScript,
    Workflow {
        class: Option<&'a str>,
        script: Option<&'a str>,
    },
    Version,
    Pages,
    R2,
    Kv,
    D1,
    QueueId,
    QueueName,
    Hyperdrive,
    AiGateway,
    SearchInstance(&'a str),
    SearchNamespace,
    SearchToken,
    PipelineStream,
}
pub(super) struct Target<'a> {
    pub(super) value: &'a str,
    pub(super) kind: Kind<'a>,
    pub(super) jurisdiction: Option<&'a str>,
}
impl<'a> Target<'a> {
    pub(super) fn new(value: &'a str, kind: Kind<'a>) -> Self {
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
        (Kind::Workflow { .. }, "cloudflare_workflow") => &["workflow_name", "name", "id"],
        (Kind::Version, "cloudflare_worker_version") => &["id"],
        (Kind::Pages, "cloudflare_pages_project") => &["id", "name"],
        (Kind::R2, "cloudflare_r2_bucket") => &["id", "name"],
        (Kind::Kv, "cloudflare_workers_kv_namespace") => &["id"],
        (Kind::D1, "cloudflare_d1_database") => &["id", "uuid"],
        (Kind::QueueId, "cloudflare_queue") => &["id", "queue_id"],
        (Kind::QueueName, "cloudflare_queue") => &["queue_name"],
        (Kind::SearchInstance(_), "cloudflare_ai_search_instance") => &["id"],
        (Kind::SearchNamespace, "cloudflare_ai_search_namespace") => &["name"],
        (Kind::PipelineStream, "cloudflare_pipeline_stream") => &["id"],
        (Kind::SearchToken, "cloudflare_ai_search_token") => &["id"],
        (Kind::AiGateway, "cloudflare_ai_gateway") => &["id"],
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
    if let Kind::SearchInstance(namespace) = target.kind {
        if value.text("namespace")? != namespace {
            return Err(Failure::ScopeMismatch);
        }
    }
    if let Kind::Workflow { class, script } = target.kind {
        if value.text("workflow_name")? != target.value {
            return Err(Failure::IdentityMismatch);
        }
        for field in ["name", "id"] {
            let alias = value.at(field)?;
            if output == "workflow_name"
                && node.mode == crate::model::EntityMode::Managed
                && alias.value.is_null()
                && alias.unknown == &serde_json::Value::Bool(true)
                && super::proof::unmasked(alias.sensitive)
            {
                continue;
            }
            if value.text(field)? != target.value {
                return Err(Failure::IdentityMismatch);
            }
        }
        for (field, expected) in [("class_name", class), ("script_name", script)] {
            if let Some(expected) = expected {
                if value.text(field)? != expected {
                    return Err(Failure::OwnerMismatch);
                }
            }
        }
        return Ok(());
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
    fields(target, node).is_ok_and(|fields| {
        fields
            .iter()
            .any(|output| identity(target, node, value, output).is_ok())
    })
}
