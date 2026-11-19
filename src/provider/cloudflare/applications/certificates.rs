use super::target::{Kind, Target};
use super::{Evidence, Failure, Result, collect};

pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    match (kind, path) {
        ("cloudflare_authenticated_origin_pulls", "config") => {
            Some("Origin pull hostname certificates")
        }
        ("cloudflare_api_shield_operation_schema_validation_settings", "operation_id") => {
            Some("API Shield operation")
        }
        ("cloudflare_certificate_authorities_hostname_associations", "mtls_certificate_id") => {
            Some("Configured mTLS certificate")
        }
        _ => None,
    }
}

pub(super) fn zone(source: Evidence<'_>, endpoint: Evidence<'_>) -> Result<()> {
    (source.id("zone_id")? == endpoint.id("zone_id")?)
        .then_some(())
        .ok_or(Failure::ScopeMismatch)
}

pub(super) fn targets<'a>(value: Evidence<'a>, path: &str) -> Result<Vec<Target<'a>>> {
    match path {
        "config" => collect(value.at(path)?.rows()?.into_iter().map(|row| {
            row.text("hostname")?;
            let enabled = row.at("enabled")?;
            enabled.scalar()?;
            if !enabled.value.is_boolean() {
                return Err(Failure::UnsupportedContract);
            }
            Ok(Target::new(row.text("cert_id")?, Kind::HostnameCertificate))
        })),
        "operation_id" => Ok(vec![Target::new(value.text(path)?, Kind::ShieldOperation)]),
        "mtls_certificate_id" => Ok(vec![Target::new(value.text(path)?, Kind::Mtls)]),
        _ => Err(Failure::UnsupportedContract),
    }
}
