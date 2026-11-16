use crate::{svg, view};
use serde_json::{Value, json};

pub(crate) fn attach(
    document: String,
    filter: &view::Options,
    rendering: svg::Options,
    diagnostics: bool,
    states: Vec<&str>,
    mut mappings: Vec<Value>,
) -> String {
    if !document.starts_with("<!doctype html>") {
        return document;
    }
    let document = if filter.anonymize {
        document.replacen(
            "<div id=\"generation-details\"></div>",
            "<div id=\"generation-details\"><p>Identifiers are pseudonyms for this export. Original selection and mapping settings and free-text details are withheld. Topology and resource types remain visible; source plan JSON is not anonymized.</p></div>",
            1,
        )
    } else {
        document
    };
    let modules: std::collections::BTreeSet<_> = filter.modules.iter().collect();
    let spatial_types: std::collections::BTreeSet<_> = filter.spatial_types.iter().collect();
    mappings.sort_by_cached_key(Value::to_string);
    mappings.dedup();
    let mut settings = json!({
        "changed_only": filter.changed_only,
        "cloudflare_accounts": filter.cloudflare_accounts,
        "cloudflare_zones": filter.cloudflare_zones,
        "cloudflare_groups": rendering.cloudflare_groups,
        "modules": modules,
        "focus": filter.focus,
        "focus_depth": filter.focus.as_ref().map(|_| filter.depth.unwrap_or(1)),
        "spatial_types": spatial_types,
        "address_format": match rendering.address_format {
            svg::AddressFormat::Qualified => "qualified",
            svg::AddressFormat::Terraform => "terraform",
        },
        "icon_mode": "service-name",
        "relationship_legend": if rendering.omit_relationship_legend { "omitted" }
            else if rendering.hide_relationship_legend { "action-colors-only" } else { "full" },
        "html_diagnostics": diagnostics,
        "html_semantic_diagnostics": filter.html_semantic_diagnostics,
        "states": states,
        "remote_state_mappings": mappings,
    });
    if filter.anonymize {
        settings["anonymize"] = true.into();
        settings["modules"] = json!([]);
        settings["cloudflare_accounts"] = json!([]);
        settings["cloudflare_zones"] = json!([]);
        settings["focus"] = Value::Null;
        settings["focus_depth"] = Value::Null;
        settings["remote_state_mappings"] = json!([]);
    }
    if filter.cloudflare_value_matches {
        settings["cloudflare_value_matches"] = true.into();
    }
    let metadata = json!({
        "schema_version": 1,
        "generator": { "name": "Planorama", "version": env!("CARGO_PKG_VERSION"), "revision": crate::build_revision() },
        "setting_origins": null,
        "selected_view": filter.changed_only || filter.focus.is_some() || !filter.modules.is_empty() || !filter.cloudflare_accounts.is_empty() || !filter.cloudflare_zones.is_empty(),
        "settings": settings,
    });
    document.replacen(
        "<script id=\"viewer-generation\" type=\"application/json\">null</script>",
        &format!(
            "<script id=\"viewer-generation\" type=\"application/json\">{}</script>",
            super::safe_json(&metadata)
        ),
        1,
    )
}
