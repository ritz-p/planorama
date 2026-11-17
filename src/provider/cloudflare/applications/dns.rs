use super::{AttributeReference, Evidence, Failure, Result, TerraformEntity, proof};
pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    match (kind, path) {
        ("cloudflare_dns_record", "content") => Some("Configured CNAME target"),
        _ => None,
    }
}
pub(super) fn verify(
    nodes: &[TerraformEntity],
    refs: &[AttributeReference],
    r: &AttributeReference,
    evidence: &[Evidence<'_>],
) -> Result<()> {
    let value = evidence[r.target];
    if value.text("type")? != "CNAME" {
        return Err(Failure::UnsupportedContract);
    }
    let proxied = value.at("proxied")?;
    proxied.scalar()?;
    if !proxied.value.is_boolean() {
        return Err(Failure::InvalidValue);
    }
    if !refs.iter().any(|owner| {
        owner.target == r.target && owner.attribute == "zone_id" && owner.service_reference_verified
    }) {
        return Err(Failure::OwnerMismatch);
    }
    let [source] = r.sources.as_slice() else {
        return Err(Failure::IncompleteProvenance);
    };
    let content = value.text("content")?;
    let target = match nodes[*source].resource_type.as_str() {
        "cloudflare_pages_project" => proof::Target {
            value: content,
            kind: "cloudflare_pages_project",
            output: "subdomain",
        },
        "cloudflare_zero_trust_tunnel_cloudflared" => {
            let id = content
                .strip_suffix(".cfargotunnel.com")
                .ok_or(Failure::IdentityMismatch)?;
            if proxied.value.as_bool() != Some(true)
                || id.len() != 36
                || !id.bytes().enumerate().all(|(i, b)| {
                    if matches!(i, 8 | 13 | 18 | 23) {
                        b == b'-'
                    } else {
                        b.is_ascii_hexdigit()
                    }
                })
            {
                return Err(Failure::UnsupportedContract);
            }
            proof::Target {
                value: id,
                kind: "cloudflare_zero_trust_tunnel_cloudflared",
                output: "id",
            }
        }
        _ => return Err(Failure::UnsupportedContract),
    };
    proof::verify(nodes, refs, r, evidence, &[target], |a, b| {
        proof::account(&nodes[a], &nodes[b], evidence[a], evidence[b])
    })
}
