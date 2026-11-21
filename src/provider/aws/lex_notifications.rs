use super::configured_links::{Evidence, Rule};
use crate::model::AttributeReference;

const BOT: &str = "aws_lexv2models_bot";
const LOCALE: &str = "aws_lexv2models_bot_locale";
const VERSION: &str = "aws_lexv2models_bot_version";

pub(super) fn indexed_kind(kind: &str) -> bool {
    matches!(kind, BOT | INTENT | SLOT_TYPE)
}

const INTENT: &str = "aws_lexv2models_intent";
const SLOT: &str = "aws_lexv2models_slot";
const SLOT_TYPE: &str = "aws_lexv2models_slot_type";

const CHANNEL: &str = "aws_notifications_channel_association";
const EVENT: &str = "aws_notifications_event_rule";
const ORG: &str = "aws_notifications_organizational_unit_association";
const MANAGED: &str = "aws_notifications_managed_notification_additional_channel_association";
const CONFIG: &str = "aws_notifications_notification_configuration";

type Outputs = &'static [(&'static str, &'static [&'static str])];
const CHANNELS: Outputs = &[
    ("aws_notificationscontacts_email_contact", &["arn"]),
    (
        "aws_chatbot_slack_channel_configuration",
        &["chat_configuration_arn"],
    ),
    (
        "aws_chatbot_teams_channel_configuration",
        &["chat_configuration_arn"],
    ),
];
const ORGANIZATIONS: Outputs = &[
    ("aws_organizations_organization", &["id", "roots[].id"]),
    ("aws_organizations_organizational_unit", &["id"]),
];
pub(super) fn rule(kind: &str, path: &str) -> Option<Rule> {
    if kind == VERSION && path == "locale_specification" {
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
        (INTENT | SLOT | SLOT_TYPE, "bot_id") => {
            Rule::new("Lex bot", &[(BOT, &["id"])], false, &[])
        }
        (INTENT | SLOT | SLOT_TYPE, "locale_id") => {
            Rule::new("Lex bot locale", &[(LOCALE, &["locale_id"])], false, &[])
        }
        (INTENT | SLOT | SLOT_TYPE, "bot_version") => Rule::new(
            "Lex bot version",
            &[(VERSION, &["bot_version"])],
            false,
            &[],
        ),
        (INTENT, "slot_priority.slot_id") => Rule::new(
            "Lex priority slot",
            &[(SLOT, &["slot_id"])],
            false,
            &["slot_priority"],
        ),
        (INTENT, "kendra_configuration.kendra_index") => Rule::new(
            "Lex Kendra index",
            &[("aws_kendra_index", &["arn"])],
            false,
            &[],
        ),
        (SLOT, "intent_id") => {
            Rule::new("Lex slot intent", &[(INTENT, &["intent_id"])], false, &[])
        }
        (SLOT, "slot_type_id") => Rule::new(
            "Lex custom slot type",
            &[(SLOT_TYPE, &["slot_type_id"])],
            false,
            &[],
        ),
        (SLOT_TYPE, "composite_slot_type_setting.sub_slots.slot_type_id") => Rule::new(
            "Lex composite subtype",
            &[(SLOT_TYPE, &["slot_type_id"])],
            false,
            &["composite_slot_type_setting.sub_slots"],
        ),
        (SLOT_TYPE, "external_source_setting.grammar_slot_type_setting.source.s3_bucket_name") => {
            Rule::new(
                "Lex grammar bucket",
                &[("aws_s3_bucket", &["bucket", "id"])],
                false,
                &[],
            )
        }
        (SLOT_TYPE, "external_source_setting.grammar_slot_type_setting.source.kms_key_arn") => {
            Rule::new("Lex grammar key", &[("aws_kms_key", &["arn"])], false, &[])
        }
        (CHANNEL | EVENT | ORG, "notification_configuration_arn") => Rule::new(
            "Notification configuration",
            &[(CONFIG, &["arn"])],
            false,
            &[],
        )
        .in_control_plane_account(),
        (CHANNEL, "arn") => {
            Rule::new("Notification channel", CHANNELS, false, &[]).in_control_plane_account()
        }
        (MANAGED, "channel_arn") => Rule::new("Managed notification channel", CHANNELS, false, &[])
            .in_control_plane_account(),
        (ORG, "organizational_unit_id") => Rule::new(
            "Notification organization target",
            ORGANIZATIONS,
            false,
            &[],
        )
        .in_control_plane_account(),
        _ => return None,
    })
}

pub(super) fn owner(kind: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match kind {
        LOCALE => Some(("bot_id", "Lex bot", "Locales")),
        VERSION => Some(("bot_id", "Lex bot", "Versions")),
        INTENT => Some(("locale_id", "Lex bot locale", "Intents")),
        SLOT => Some(("intent_id", "Lex slot intent", "Slots")),
        SLOT_TYPE => Some(("locale_id", "Lex bot locale", "Slot types")),
        EVENT => Some((
            "notification_configuration_arn",
            "Notification configuration",
            "Event rules",
        )),
        CHANNEL => Some((
            "notification_configuration_arn",
            "Notification configuration",
            "Channels",
        )),
        ORG => Some((
            "notification_configuration_arn",
            "Notification configuration",
            "Organization associations",
        )),
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
        (INTENT | SLOT | SLOT_TYPE, "locale_id") => {
            same_bot(e, target, source)
                && e.text(target, "bot_version")
                    .is_some_and(|v| e.text(source, "bot_version") == Some(v))
        }
        (INTENT, "slot_priority.slot_id") => {
            same_locale(e, target, source)
                && super::communications::endpoint(e, source, "intent_id", INTENT, "intent_id")
                    == Some(target)
        }
        (SLOT, "intent_id" | "slot_type_id")
        | (SLOT_TYPE, "composite_slot_type_setting.sub_slots.slot_type_id") => {
            same_locale(e, target, source)
        }
        (INTENT, "kendra_configuration.kendra_index") => {
            e.text(target, "parent_intent_signature") == Some("AMAZON.KendraSearchIntent")
                && super::communications::input_arn(e, target, source, path, "kendra", "index/")
        }
        (SLOT_TYPE, "external_source_setting.grammar_slot_type_setting.source.kms_key_arn") => {
            super::communications::input_arn(e, target, source, path, "kms", "key/")
        }
        (CHANNEL | EVENT | ORG, "notification_configuration_arn") => e
            .text(target, path)
            .is_some_and(|v| notification_arn(e, source, v, "notifications", "configuration/")),
        (CHANNEL, "arn") | (MANAGED, "channel_arn") => e.text(target, path).is_some_and(|v| {
            let prefix =
                if e.nodes[source].resource_type == "aws_notificationscontacts_email_contact" {
                    "notifications-contacts"
                } else {
                    "chatbot"
                };
            notification_arn(
                e,
                source,
                v,
                prefix,
                if prefix == "chatbot" {
                    "chat-configuration/"
                } else {
                    "emailcontact/"
                },
            )
        }),
        (ORG, "organizational_unit_id") => e.text(target, path).is_some_and(|v| {
            let prefix = if e.nodes[source].resource_type == "aws_organizations_organizational_unit"
            {
                "ou-"
            } else if v.starts_with("r-") {
                "r-"
            } else {
                "o-"
            };
            v.strip_prefix(prefix).is_some_and(|id| {
                !id.is_empty() && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
            })
        }),
        _ => true,
    }
}

fn source_versions(e: &Evidence<'_>, r: &AttributeReference) -> Option<()> {
    let target = r.target;
    if !super::communications::current(&e.nodes[target])
        || !r.complete
        || !r.instances_explicit
        || !r.issues.is_empty()
        || r.sources.is_empty()
    {
        return None;
    }
    let locales = e.values[target].0["locale_specification"].as_object()?;
    let mut verified = std::collections::BTreeSet::new();
    for locale in locales.keys() {
        if locale.is_empty()
            || !locale
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_')
        {
            return None;
        }
        let path = format!("locale_specification.{locale}.source_bot_version");
        let version = e.text(target, &path)?;
        let mut candidates = r.sources.iter().copied().filter(|&source| {
            let node = &e.nodes[source];
            let attributes = r.source_attributes_by_instance.get(&source);
            source != target
                && super::communications::current(node)
                && attributes.is_some_and(|a| a.len() == 1 && a.contains("bot_version"))
                && same_bot(e, target, source)
                && super::communications::same_region(&e.nodes[target], node)
                && e.text(source, "bot_version") == Some(version)
                && match node.resource_type.as_str() {
                    LOCALE if version == "DRAFT" => e.text(source, "locale_id") == Some(locale),
                    VERSION if version.parse::<u32>().is_ok_and(|v| v > 0) => {
                        e.text(source, &path).is_some()
                    }
                    _ => false,
                }
        });
        if let Some(source) = candidates.next() {
            if candidates.next().is_some() {
                return None;
            }
            verified.insert(source);
        } else if version != "DRAFT" {
            return None;
        }
    }
    (verified.len() == r.sources.len()).then_some(())
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
            (nodes[r.target].resource_type == VERSION && r.attribute == "locale_specification")
                .then(|| source_versions(&e, r).is_some())
        })
        .collect();
    for (r, valid) in refs.iter_mut().zip(matches) {
        if let Some(valid) = valid {
            r.service_reference_verified = valid;
        }
    }
}

fn same_locale(e: &Evidence<'_>, a: usize, b: usize) -> bool {
    same_bot(e, a, b)
        && ["locale_id", "bot_version"].iter().all(|field| {
            e.text(a, field)
                .is_some_and(|v| e.text(b, field) == Some(v))
        })
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

fn notification_arn(
    e: &Evidence<'_>,
    source: usize,
    value: &str,
    service: &str,
    prefix: &str,
) -> bool {
    let parts: Vec<_> = value.splitn(6, ':').collect();
    let ["arn", partition, actual_service, region, account, resource] = parts.as_slice() else {
        return false;
    };
    partition.starts_with("aws")
        && *actual_service == service
        && region.is_empty()
        && crate::model::AwsAccountScope::new(account).is_some()
        && e.nodes[source]
            .provider_configuration
            .as_ref()
            .and_then(|c| c.account.as_ref())
            .is_none_or(|a| a.as_str() == *account)
        && resource
            .strip_prefix(prefix)
            .is_some_and(|id| !id.is_empty())
}

#[cfg(test)]
#[path = "../../../tests/unit/semantic/lex_notifications.rs"]
mod tests;
