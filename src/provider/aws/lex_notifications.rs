use super::configured_links::{Evidence, Rule};
use crate::model::AttributeReference;

const BOT: &str = "aws_lexv2models_bot";
const LOCALE: &str = "aws_lexv2models_bot_locale";
const VERSION: &str = "aws_lexv2models_bot_version";

pub(super) fn indexed_kind(kind: &str) -> bool {
    matches!(kind, BOT)
}

pub(super) fn rule(kind: &str, path: &str) -> Option<Rule> {
    if kind == VERSION && locale_map_key(path).is_some() {
        return Some(Rule::new(
            "Lex source locale version",
            &[(VERSION, &["bot_version"])],
            false,
            &[],
        ));
    }
    Some(match (kind, path) {
        (BOT, "role_arn") => Rule::new(
            "Lex execution role",
            &[("aws_iam_role", &["arn"])],
            false,
            &[],
        ),
        (BOT, "members.id") => {
            Rule::new("Lex network member", &[(BOT, &["id"])], false, &["members"])
        }
        (LOCALE | VERSION, "bot_id") => Rule::new("Lex bot", &[(BOT, &["id"])], false, &[]),
        (LOCALE, "bot_version") => Rule::new(
            "Lex locale version",
            &[(VERSION, &["bot_version"])],
            false,
            &[],
        ),
        _ => return None,
    })
}

pub(super) fn owner(kind: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match kind {
        LOCALE => Some(("bot_id", "Lex bot", "Locales")),
        VERSION => Some(("bot_id", "Lex bot", "Versions")),
        _ => None,
    }
}

fn bot(e: &Evidence<'_>, node: usize) -> Option<usize> {
    super::communications::endpoint(e, node, "bot_id", BOT, "id")
        .filter(|&owner| super::communications::same_region(&e.nodes[node], &e.nodes[owner]))
}

fn same_bot(e: &Evidence<'_>, left: usize, right: usize) -> bool {
    bot(e, left).is_some_and(|owner| bot(e, right) == Some(owner))
}

pub(super) fn qualifies(
    e: &Evidence<'_>,
    r: &AttributeReference,
    source: usize,
    path: &str,
) -> bool {
    let target = r.target;
    match (e.nodes[target].resource_type.as_str(), r.attribute.as_str()) {
        (BOT, "role_arn") => {
            super::communications::input_arn(e, target, source, path, "iam", "role/")
        }
        (LOCALE, "bot_id") if path == "bot_id" => {
            e.text(target, "bot_version") == Some("DRAFT")
                || super::communications::endpoint(e, target, "bot_version", VERSION, "bot_version")
                    .is_some_and(|version| {
                        same_bot(e, target, version)
                            && super::communications::same_region(
                                &e.nodes[target],
                                &e.nodes[version],
                            )
                    })
        }
        (BOT, "members.id") => member(e, target, source, path),
        (_, "bot_version") => {
            same_bot(e, target, source) && e.text(target, path).is_some_and(|v| v != "DRAFT")
        }
        _ => true,
    }
}

fn locale_map_key(path: &str) -> Option<&str> {
    let locale = path
        .strip_prefix("locale_specification.")?
        .strip_suffix(".source_bot_version")?;
    (!locale.is_empty()
        && locale
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_'))
    .then_some(locale)
}

pub(super) fn annotate_map(
    nodes: &[crate::model::TerraformEntity],
    refs: &mut [AttributeReference],
    values: &[(&serde_json::Value, &serde_json::Value)],
) {
    let e = Evidence {
        nodes,
        refs,
        values,
    };
    let matches: Vec<_> = refs
        .iter()
        .map(|r| {
            if nodes[r.target].resource_type != VERSION {
                return None;
            }
            let locale = locale_map_key(&r.attribute)?;
            Some(
                super::communications::endpoint(&e, r.target, &r.attribute, VERSION, "bot_version")
                    .is_some_and(|source| {
                        same_bot(&e, r.target, source)
                            && super::communications::same_region(&nodes[r.target], &nodes[source])
                            && e.text(r.target, &r.attribute)
                                .is_some_and(|v| v.parse::<u32>().is_ok_and(|n| n > 0))
                            && e.value(source, &format!("locale_specification.{locale}"))
                                .is_some_and(serde_json::Value::is_object)
                    }),
            )
        })
        .collect();
    for (r, valid) in refs.iter_mut().zip(matches) {
        if let Some(valid) = valid {
            r.service_reference_verified = valid;
        }
    }
}

fn member(e: &Evidence<'_>, target: usize, source: usize, path: &str) -> bool {
    let Some((row, _)) = path.rsplit_once('.') else {
        return false;
    };
    let Some(version) = e.text(target, &format!("{row}.version")) else {
        return false;
    };
    e.text(target, "type") == Some("BotNetwork")
        && e.text(source, "type") == Some("Bot")
        && version.parse::<u32>().is_ok_and(|n| n > 0)
        && e.text(target, &format!("{row}.alias_id")).is_some()
        && e.nodes
            .iter()
            .enumerate()
            .filter(|(i, n)| {
                n.resource_type == VERSION
                    && super::communications::current(n)
                    && bot(e, *i) == Some(source)
                    && e.text(*i, "bot_version") == Some(version)
                    && super::communications::same_region(&e.nodes[source], n)
            })
            .count()
            == 1
}

#[cfg(test)]
#[path = "../../../tests/unit/semantic/lex_notifications.rs"]
mod tests;
