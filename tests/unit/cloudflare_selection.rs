use crate::{Options, OutputFormat, generate, plan, semantic, view};
use serde_json::{Value, json};

const FIXTURE: &str = include_str!("../fixtures/cloudflare-imported-plan.json");
const ACCOUNT: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const ZONE: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

fn selected(value: &Value, accounts: &[&str], zones: &[&str]) -> crate::model::architecture::Graph {
    let graph = semantic::transform(&plan::parse(&value.to_string()).unwrap());
    super::single(
        &graph,
        &view::Options {
            cloudflare_accounts: accounts.iter().map(|s| s.to_string()).collect(),
            cloudflare_zones: zones.iter().map(|s| s.to_string()).collect(),
            ..Default::default()
        },
    )
    .unwrap()
    .0
}

#[test]
fn scope_selection_uses_known_values_without_owners_or_value_recovery() {
    let mut value: Value = serde_json::from_str(FIXTURE).unwrap();
    let graph = selected(&value, &[], &[ZONE]);
    assert_eq!(graph.nodes.len(), 3);
    assert_eq!(selected(&value, &[ACCOUNT], &[ZONE]).nodes.len(), 2);
    assert_eq!(selected(&value, &["unknown"], &[ZONE]).nodes.len(), 1);
    value["resource_changes"]
        .as_array_mut()
        .unwrap()
        .retain(|node| node["type"] != "cloudflare_zone");
    assert_eq!(selected(&value, &[], &[ZONE]).nodes.len(), 2);
    assert_eq!(selected(&value, &[ACCOUNT], &[]).nodes.len(), 3);
    value["resource_changes"][3]["change"]["after_sensitive"]["zone_id"] = json!(true);
    assert_eq!(selected(&value, &[], &[ZONE]).nodes.len(), 1);
}

#[test]
fn management_cards_preserve_alias_and_lifecycle_and_anonymize_scope() {
    let mut value: Value = serde_json::from_str(FIXTURE).unwrap();
    let mut alias = value["resource_changes"][1].clone();
    alias["address"] = json!("cloudflare_worker.alias");
    alias["name"] = json!("alias");
    value["configuration"]["root_module"]["resources"].as_array_mut().unwrap().push(json!({"address":"cloudflare_worker.alias", "mode":"managed", "type":"cloudflare_worker", "name":"alias", "provider_config_key":"cloudflare.other", "expressions":{}}));
    let mut deposed = alias.clone();
    deposed["deposed"] = json!("deadbeef");
    deposed["change"]["actions"] = json!(["delete"]);
    value["resource_changes"]
        .as_array_mut()
        .unwrap()
        .extend([alias, deposed]);
    for format in [OutputFormat::Svg, OutputFormat::Html] {
        let output = generate(
            &value.to_string(),
            &Options {
                format,
                cloudflare_groups: true,
                ..Default::default()
            },
        )
        .unwrap()
        .content;
        let start = output.find("<svg").unwrap();
        let end = output[start..].find("</svg>").unwrap() + start + 6;
        let doc = roxmltree::Document::parse(&output[start..end]).unwrap();
        assert_eq!(
            doc.descendants()
                .filter(|n| n.attribute("data-cloudflare-management-member").is_some())
                .count(),
            7
        );
        for member in doc
            .descendants()
            .filter(|n| n.attribute("data-cloudflare-management-member").is_some())
        {
            let target = &member.attribute("href").unwrap()[1..];
            assert!(doc.descendants().any(|n| n.attribute("id") == Some(target)));
        }
        assert!(output.contains("cloudflare.other"));
        assert!(output.contains("deposed: deadbeef"));
        assert!(output.contains("not physical containment"));
        let anonymized = generate(
            &value.to_string(),
            &Options {
                format,
                cloudflare_groups: true,
                anonymize: true,
                cloudflare_accounts: vec![ACCOUNT.into()],
                ..Default::default()
            },
        )
        .unwrap()
        .content;
        for secret in [ACCOUNT, ZONE, "deadbeef", "cloudflare.other"] {
            assert!(!anonymized.contains(secret), "{secret}");
        }
    }
}

#[test]
fn invalid_scope_selectors_are_rejected() {
    for selector in [
        "",
        "example.invalid",
        "account_1",
        "gggggggggggggggggggggggggggggggg",
    ] {
        assert!(
            generate(
                FIXTURE,
                &Options {
                    cloudflare_accounts: vec![selector.into()],
                    ..Default::default()
                }
            )
            .is_err()
        );
    }
}

#[test]
fn multiple_scopes_do_not_select_aws_or_invent_zone_account_context() {
    let value: Value = serde_json::from_str(include_str!(
        "../fixtures/cloudflare-administration-plan.json"
    ))
    .unwrap();
    let account = selected(&value, &[ACCOUNT, "dddddddddddddddddddddddddddddddd"], &[]);
    assert_eq!(account.nodes.len(), 7);
    assert!(account.nodes.iter().all(|n| n.provider.is_cloudflare()));
    let zones = selected(&value, &[], &[ZONE, "cccccccccccccccccccccccccccccccc"]);
    assert_eq!(zones.nodes.len(), 4);
    let unknown = selected(&value, &["unknown"], &[]);
    assert_eq!(unknown.nodes.len(), 2);
    assert!(
        unknown
            .nodes
            .iter()
            .all(|n| n.resource_type == "cloudflare_workers_route")
    );
    let raw = plan::parse_with_value_matches(&value.to_string(), true).unwrap();
    let graph = semantic::transform(&raw);
    let recovered = super::single(
        &graph,
        &view::Options {
            cloudflare_accounts: vec!["unknown".into()],
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(recovered.nodes.len(), 2);
    let explicit = semantic::transform(
        &plan::parse(include_str!(
            "../fixtures/cloudflare-applications-plan.json"
        ))
        .unwrap(),
    );
    let route = explicit
        .nodes
        .iter()
        .find(|n| n.resource_type == "cloudflare_workers_route")
        .unwrap();
    assert!(
        route
            .entity
            .cloudflare_scope
            .as_ref()
            .unwrap()
            .account
            .is_some()
    );
}
