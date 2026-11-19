use super::target::{Kind, Target, jurisdiction};
use super::{Evidence, Failure, Result};

pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    match (kind, path) {
        ("cloudflare_pipeline_sink", "config") => Some("Pipeline sink bucket"),
        ("cloudflare_r2_data_catalog", "bucket_name") => Some("Catalog bucket"),
        _ => None,
    }
}

pub(super) fn targets<'a>(value: Evidence<'a>, path: &str) -> Result<Vec<Target<'a>>> {
    let (name, jurisdiction) = match path {
        "config" => {
            if !matches!(
                value.text("type")?,
                "r2" | "r2_data_catalog" | "basin_catalog"
            ) {
                return Err(Failure::UnsupportedContract);
            }
            let config = value.at("config")?;
            if value.id("account_id")? != config.id("account_id")? {
                return Err(Failure::ScopeMismatch);
            }
            (config.text("bucket")?, jurisdiction(config)?)
        }
        "bucket_name" => (value.text(path)?, "default"),
        _ => return Err(Failure::UnsupportedContract),
    };
    Ok(vec![Target {
        value: name,
        kind: Kind::R2,
        jurisdiction: Some(jurisdiction),
    }])
}
