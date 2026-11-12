use crate::model::{Action, AttributeReference, EntityMode, TerraformEntity};
use crate::semantic::Input;
use serde_json::Value;
use std::collections::BTreeSet;
#[cfg(test)]
#[path = "../../../tests/unit/semantic/datazone.rs"]
mod tests;
pub(super) const DOMAIN: &str = "aws_datazone_domain";
pub(super) const PROJECT: &str = "aws_datazone_project";
pub(super) const ASSET: &str = "aws_datazone_asset_type";
pub(super) const FORM: &str = "aws_datazone_form_type";
pub(super) const GLOSSARY: &str = "aws_datazone_glossary";
pub(super) const TERM: &str = "aws_datazone_glossary_term";
pub(crate) fn resource(k: &str) -> bool {
    matches!(k, DOMAIN | PROJECT | ASSET | FORM | GLOSSARY | TERM)
}
pub(super) fn owner(k: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match k {
        PROJECT => Some(("domain_identifier", "DataZone domain", "Projects")),
        ASSET | FORM | GLOSSARY => Some((
            "owning_project_identifier",
            "DataZone owning project",
            "Catalog metadata",
        )),
        TERM => Some(("glossary_identifier", "DataZone glossary", "Glossary terms")),
        _ => None,
    }
}
fn canonical(p: &str) -> String {
    p.split('.')
        .filter(|s| s.parse::<usize>().is_err())
        .collect::<Vec<_>>()
        .join(".")
}
pub(crate) fn indexed(k: &str, p: &str) -> bool {
    p.split('.').any(|s| s.parse::<usize>().is_ok()) && label(k, &canonical(p)).is_some()
}
pub(super) fn label(k: &str, p: &str) -> Option<&'static str> {
    match (k, p) {
        (DOMAIN, "domain_execution_role") => Some("DataZone execution role"),
        (DOMAIN, "service_role") => Some("DataZone V2 service role"),
        (DOMAIN, "kms_key_identifier") => Some("DataZone encryption key"),
        (PROJECT | ASSET | FORM | GLOSSARY | TERM, "domain_identifier") => Some("DataZone domain"),
        (ASSET | FORM | GLOSSARY, "owning_project_identifier") => Some("DataZone owning project"),
        (PROJECT, "glossary_terms") => Some("DataZone project glossary terms"),
        (ASSET, "forms_input.type_identifier") => Some("Revision-qualified asset forms"),
        (ASSET, "forms_input.type_revision") => Some("Qualified asset form revisions"),
        (TERM, "glossary_identifier") => Some("DataZone glossary"),
        (TERM, "term_relations.classifies") => Some("Classified glossary terms"),
        (TERM, "term_relations.is_a") => Some("Parent taxonomy terms"),
        _ => None,
    }
}
fn fields(k: &str, p: &str, s: &str) -> &'static [&'static str] {
    match (k, p, s) {
        (DOMAIN, "domain_execution_role" | "service_role", "aws_iam_role")
        | (DOMAIN, "kms_key_identifier", "aws_kms_key") => &["arn"],
        (PROJECT | ASSET | FORM | GLOSSARY | TERM, "domain_identifier", DOMAIN)
        | (ASSET | FORM | GLOSSARY, "owning_project_identifier", PROJECT)
        | (PROJECT, "glossary_terms", TERM)
        | (TERM, "glossary_identifier", GLOSSARY)
        | (TERM, "term_relations.classifies" | "term_relations.is_a", TERM) => &["id"],
        (ASSET, "forms_input.type_identifier", FORM) => &["name"],
        (ASSET, "forms_input.type_revision", FORM) => &["revision"],
        _ => &[],
    }
}
pub(super) fn current(n: &TerraformEntity) -> bool {
    n.provider.is_aws()
        && n.mode == EntityMode::Managed
        && n.action != Action::Delete
        && n.deposed_key.is_none()
}
pub(super) fn value<'a>(e: &[(&'a Value, &Value)], t: usize, p: &str) -> Option<&'a Value> {
    let (v, m) = e[t];
    super::nested_references::value(v, m, p)
}
pub(super) fn string<'a>(e: &[(&'a Value, &Value)], t: usize, p: &str) -> Option<&'a str> {
    value(e, t, p)?.as_str().filter(|s| !s.is_empty())
}
fn simple(
    nodes: &[TerraformEntity],
    refs: &[AttributeReference],
    e: &[(&Value, &Value)],
    t: usize,
    p: &str,
) -> Option<usize> {
    let r = super::lambda_core::unique(refs, t, p)?;
    let [s] = r.sources.as_slice() else {
        return None;
    };
    super::nested_references::verify(nodes, refs, e, r, fields, |_, _| true)?;
    Some(*s)
}
pub(super) fn direct_domain(
    nodes: &[TerraformEntity],
    refs: &[AttributeReference],
    e: &[(&Value, &Value)],
    t: usize,
) -> Option<usize> {
    simple(nodes, refs, e, t, "domain_identifier")
}
pub(super) fn domain(
    nodes: &[TerraformEntity],
    refs: &[AttributeReference],
    e: &[(&Value, &Value)],
    t: usize,
) -> Option<usize> {
    if !current(&nodes[t]) {
        return None;
    }
    match nodes[t].resource_type.as_str() {
        DOMAIN => Some(t),
        TERM => {
            let glossary = simple(nodes, refs, e, t, "glossary_identifier")?;
            let domain = direct_domain(nodes, refs, e, glossary)?;
            let configured = value(e, t, "domain_identifier")?;
            if !configured.is_null() && configured.as_str() != string(e, domain, "id") {
                return None;
            }
            if super::lambda_core::unique(refs, t, "domain_identifier")
                .is_some_and(|r| r.has_references)
                && direct_domain(nodes, refs, e, t) != Some(domain)
            {
                return None;
            }
            Some(domain)
        }
        PROJECT | ASSET | FORM | GLOSSARY => direct_domain(nodes, refs, e, t),
        _ => None,
    }
}
pub(super) fn same_domain(
    nodes: &[TerraformEntity],
    refs: &[AttributeReference],
    e: &[(&Value, &Value)],
    a: usize,
    b: usize,
) -> bool {
    domain(nodes, refs, e, a)
        .zip(domain(nodes, refs, e, b))
        .is_some_and(|(a, b)| a == b)
}
fn shape(v: &Value, p: &str) -> bool {
    match p {
        "forms_input.type_identifier" | "forms_input.type_revision" => {
            v["forms_input"].as_array().is_some_and(|rows| {
                !rows.is_empty()
                    && rows
                        .iter()
                        .all(|v| v["type_identifier"].is_string() && v["type_revision"].is_string())
            })
        }
        "term_relations.classifies" | "term_relations.is_a" => {
            v["term_relations"].as_array().is_some_and(|a| {
                a.len() == 1
                    && a[0][p.rsplit('.').next().unwrap()]
                        .as_array()
                        .is_some_and(|a| {
                            !a.is_empty() && a.len() <= 10 && a.iter().all(Value::is_string)
                        })
            })
        }
        "glossary_terms" => v[p]
            .as_array()
            .is_some_and(|a| !a.is_empty() && a.iter().all(Value::is_string)),
        _ => v[p].is_string(),
    }
}
fn form(
    nodes: &[TerraformEntity],
    refs: &[AttributeReference],
    e: &[(&Value, &Value)],
    r: &AttributeReference,
    s: usize,
    path: &str,
) -> bool {
    let t = r.target;
    let Some(index) = path.split('.').nth(1) else {
        return false;
    };
    let base = format!("forms_input.{index}");
    if string(e, t, &format!("{base}.type_identifier")) != string(e, s, "name")
        || string(e, t, &format!("{base}.type_revision")) != string(e, s, "revision")
    {
        return false;
    }
    let Some(rows) = e[t].0["forms_input"].as_array() else {
        return false;
    };
    let keys: Option<BTreeSet<_>> = (0..rows.len())
        .map(|i| string(e, t, &format!("forms_input.{i}.map_block_key")))
        .collect();
    if keys.is_none_or(|keys| keys.len() != rows.len()) {
        return false;
    }
    for (field, output) in [("type_identifier", "name"), ("type_revision", "revision")] {
        if let Some(other) = super::lambda_core::unique(refs, t, &format!("{base}.{field}"))
            .filter(|r| r.has_references)
        {
            if other.sources != [s]
                || other.source_attributes.len() != 1
                || !other.source_attributes.contains(output)
            {
                return false;
            }
        }
    }
    same_domain(nodes, refs, e, t, s)
}
pub(super) fn annotate(
    nodes: &[TerraformEntity],
    refs: &mut [AttributeReference],
    e: &[(&Value, &Value)],
) {
    let results: Vec<_> = refs
        .iter()
        .map(|r| {
            let k = nodes[r.target].resource_type.as_str();
            let p = r.attribute.as_str();
            label(k, p)?;
            let valid = |s, path: &str| match (k, p) {
                (DOMAIN, "service_role") => string(e, r.target, "domain_version") == Some("V2"),
                (DOMAIN, _) | (_, "domain_identifier") => true,
                (ASSET, "forms_input.type_identifier" | "forms_input.type_revision") => {
                    form(nodes, refs, e, r, s, path)
                }
                _ => same_domain(nodes, refs, e, r.target, s),
            };
            Some(
                shape(e[r.target].0, p)
                    && super::nested_references::verify_collections(
                        nodes,
                        refs,
                        e,
                        r,
                        fields,
                        valid,
                        &["forms_input"],
                    )
                    .is_some(),
            )
        })
        .collect();
    for (r, ok) in refs.iter_mut().zip(results) {
        if let Some(ok) = ok {
            r.service_reference_verified = ok;
            if ok {
                r.complete = true;
            }
        }
    }
}
pub(super) fn compatible(raw: &Input, a: usize, b: usize) -> bool {
    let (l, r) = (&raw.nodes[a], &raw.nodes[b]);
    super::eventbridge::bindings(
        l.provider_configuration.as_ref(),
        r.provider_configuration.as_ref(),
        l.entity.scope.as_ref().and_then(|s| s.region.as_ref()),
        r.entity.scope.as_ref().and_then(|s| s.region.as_ref()),
        r.resource_type == "aws_iam_role",
    )
}
pub(super) fn domain_bound(raw: &Input, t: usize) -> bool {
    let k = raw.nodes[t].resource_type.as_str();
    if k == DOMAIN {
        return true;
    }
    let path = if k == TERM {
        "glossary_identifier"
    } else {
        "domain_identifier"
    };
    super::lambda_core::unique(&raw.attributes, t, path).is_some_and(|r| {
        r.service_reference_verified
            && r.sources.len() == 1
            && compatible(raw, t, r.sources[0])
            && (k != TERM || domain_bound(raw, r.sources[0]))
    })
}
pub(super) fn bindings(raw: &Input, r: &AttributeReference) -> bool {
    let k = raw.nodes[r.target].resource_type.as_str();
    if label(k, &r.attribute).is_none() {
        return true;
    }
    r.sources.iter().all(|&s| compatible(raw, r.target, s))
        && (k == DOMAIN
            || r.attribute == "domain_identifier"
            || (domain_bound(raw, r.target) && r.sources.iter().all(|&s| domain_bound(raw, s))))
}
