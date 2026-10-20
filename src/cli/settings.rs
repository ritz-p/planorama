use super::OutputFormat;
use crate::{icons::IconMode, svg::AddressFormat};
use serde::de::DeserializeOwned;
use std::collections::BTreeMap;

#[derive(Default)]
pub(super) struct Generation {
    pub spatial_types: Option<Vec<String>>,
    pub format: Option<OutputFormat>,
    pub icon_mode: Option<IconMode>,
    pub address_format: Option<AddressFormat>,
    pub hide_relationship_legend: Option<bool>,
    pub changed_only: Option<bool>,
    pub modules: Option<Vec<String>>,
    pub focus: Option<Vec<String>>,
    pub focus_depth: Option<usize>,
    pub diagnostics: Option<bool>,
    pub timings: Option<bool>,
    pub html_diagnostics: Option<bool>,
    pub html_semantic_diagnostics: Option<bool>,
    pub layout_diagnostics: Option<bool>,
}

impl Generation {
    pub fn overlay(mut self, higher: Self) -> Self {
        if higher.focus.as_ref().is_some_and(Vec::is_empty) {
            self.focus_depth = None;
        }
        macro_rules! overlay {
            ($($field:ident),+ $(,)?) => {
                $(if higher.$field.is_some() { self.$field = higher.$field; })+
            };
        }
        overlay!(
            spatial_types,
            format,
            icon_mode,
            address_format,
            hide_relationship_legend,
            changed_only,
            modules,
            focus,
            focus_depth,
            diagnostics,
            timings,
            html_diagnostics,
            html_semantic_diagnostics,
            layout_diagnostics
        );
        self
    }

    fn parse(mut table: toml::Table) -> Result<Self, String> {
        let result = Self {
            spatial_types: take(&mut table, "spatial-types")?,
            format: take(&mut table, "format")?,
            icon_mode: take(&mut table, "icon-mode")?,
            address_format: take(&mut table, "address-format")?,
            hide_relationship_legend: take(&mut table, "hide-relationship-legend")?,
            changed_only: take(&mut table, "changed-only")?,
            modules: take(&mut table, "modules")?,
            focus: take(&mut table, "focus")?,
            focus_depth: take(&mut table, "focus-depth")?,
            diagnostics: take(&mut table, "diagnostics")?,
            timings: take(&mut table, "timings")?,
            html_diagnostics: take(&mut table, "html-diagnostics")?,
            html_semantic_diagnostics: take(&mut table, "html-semantic-diagnostics")?,
            layout_diagnostics: take(&mut table, "layout-diagnostics")?,
        };
        if !table.is_empty() {
            return Err("unknown generation setting key; see docs/configuration.md".into());
        }
        if result.focus.as_ref().is_some_and(|values| values.len() > 1) {
            return Err("config focus must contain at most one address".into());
        }
        if let Some(types) = &result.spatial_types {
            crate::view::purpose::validate(types)?;
        }
        Ok(result)
    }
}

fn take<T: DeserializeOwned>(table: &mut toml::Table, key: &str) -> Result<Option<T>, String> {
    table.remove(key).map(|value| value.try_into()
        .map_err(|_| format!("invalid config setting '{key}': unexpected type or value; see docs/configuration.md")))
        .transpose()
}

pub(super) fn load(path: Option<&str>, preset: Option<&str>) -> Result<Generation, String> {
    let Some(path) = path else {
        return if preset.is_some() {
            Err("--preset requires --config".into())
        } else {
            Ok(Generation::default())
        };
    };
    let source =
        std::fs::read_to_string(path).map_err(|e| format!("cannot read config file: {e}"))?;
    parse(&source, preset)
}

fn parse(source: &str, preset: Option<&str>) -> Result<Generation, String> {
    let mut table: toml::Table = toml::from_str(source).map_err(|error: toml::de::Error| {
        let offset = error.span().map(|span| span.start).unwrap_or(0);
        format!("invalid TOML config near byte {offset}")
    })?;
    let common = take::<toml::Table>(&mut table, "generation")?.unwrap_or_default();
    let presets = take::<BTreeMap<String, toml::Table>>(&mut table, "presets")?.unwrap_or_default();
    if !table.is_empty() {
        return Err("unknown config key; expected [generation] or [presets.NAME]".into());
    }
    let common = Generation::parse(common)?;
    let mut selected = None;
    for (name, table) in presets {
        let parsed = Generation::parse(table)?;
        if Some(name.as_str()) == preset {
            selected = Some(parsed);
        }
    }
    match (preset, selected) {
        (Some(_), None) => Err("requested preset does not exist in config".into()),
        (_, Some(selected)) => Ok(common.overlay(selected)),
        _ => Ok(common),
    }
}
