use crate::{Options, OutputFormat, generate, plan, semantic};
use serde_json::{Value, json};

const FIXTURE: &str = include_str!("../fixtures/cloudflare-imported-plan.json");

fn count(value: &Value, attribute: &str) -> usize {
    let plan = plan::parse_with_value_matches(&value.to_string(), true).unwrap();
    plan.configured_value_matches
        .iter()
        .filter(|m| m.attribute == attribute)
        .count()
}

#[test]
fn imported_relationships_are_opt_in_and_distinct_from_verified_provenance() {
    let value: Value = serde_json::from_str(FIXTURE).unwrap();
    assert!(
        plan::parse(FIXTURE)
            .unwrap()
            .configured_value_matches
            .is_empty()
    );
    assert_eq!(count(&value, "zone_id"), 2);
    assert_eq!(count(&value, "script"), 1);
    assert_eq!(count(&value, "service"), 1);
    let graph = semantic::transform(&plan::parse_with_value_matches(FIXTURE, true).unwrap());
    assert_eq!(
        graph
            .relationships
            .iter()
            .filter(|r| r.provenance.iter().any(|p| matches!(
                p,
                crate::model::RelationshipProvenance::ConfiguredValue { .. }
            )))
            .count(),
        4
    );
    for format in [OutputFormat::Svg, OutputFormat::Html] {
        let ordinary = generate(
            FIXTURE,
            &Options {
                format,
                ..Default::default()
            },
        )
        .unwrap();
        let recovered = generate(
            FIXTURE,
            &Options {
                format,
                cloudflare_value_matches: true,
                omit_relationship_legend: false,
                ..Default::default()
            },
        )
        .unwrap();
        assert!(!ordinary.content.contains("configured-value"));
        assert!(recovered.content.contains("configured-value"));
        assert!(recovered.content.contains("Purple dotted lines"));
        assert!(!recovered.content.contains("PRIVATE_IMPORT_ID"));
        let mut job = crate::Generator::new(
            FIXTURE.into(),
            Options {
                format,
                cloudflare_value_matches: true,
                omit_relationship_legend: false,
                ..Default::default()
            },
        )
        .unwrap();
        loop {
            if let Some(result) = job.step(4096).unwrap() {
                assert_eq!(result.content, recovered.content);
                break;
            }
        }
    }
}

#[test]
fn typed_recovery_rejects_ambiguous_masked_conflicting_and_inactive_identities() {
    let original: Value = serde_json::from_str(FIXTURE).unwrap();
    for case in 0..11 {
        let mut value = original.clone();
        let changes = value["resource_changes"].as_array_mut().unwrap();
        let worker = changes
            .iter()
            .position(|n| n["type"] == "cloudflare_worker")
            .unwrap();
        let route = changes
            .iter()
            .position(|n| n["type"] == "cloudflare_workers_route")
            .unwrap();
        match case {
            0 => changes[worker]["change"]["after_unknown"]["name"] = json!(true),
            1 => changes[worker]["change"]["actions"] = json!(["delete"]),
            2 => changes[worker]["deposed"] = json!("old"),
            3 => {
                changes[worker]["change"]["after"]["account_id"] =
                    json!("dddddddddddddddddddddddddddddddd")
            }
            4 => changes[route]["change"]["after"]["script"] = json!("api:production"),
            5 => changes[route]["change"]["after"]["environment"] = json!("production"),
            6 => changes[route]["change"]["after_unknown"]["script"] = json!(true),
            7 => {
                let mut duplicate = changes[worker].clone();
                duplicate["address"] = json!("cloudflare_worker.duplicate");
                changes.push(duplicate);
            }
            8 => {
                let zone = changes
                    .iter()
                    .position(|n| n["type"] == "cloudflare_zone")
                    .unwrap();
                changes[zone]["change"]["after_sensitive"]["account"]["id"] = json!(true);
            }
            9 => {
                let zone = changes
                    .iter()
                    .position(|n| n["type"] == "cloudflare_zone")
                    .unwrap();
                let mut duplicate = changes[zone].clone();
                duplicate["address"] = json!("cloudflare_zone.duplicate");
                changes.push(duplicate);
            }
            10 => changes[route]["change"]["after_sensitive"]["zone_id"] = json!(true),
            _ => unreachable!(),
        }
        assert_eq!(count(&value, "script"), 0, "case {case}");
    }
}

#[test]
fn recovery_never_replaces_explicit_expression_provenance_or_crosses_states() {
    let explicit = include_str!("../fixtures/cloudflare-applications-plan.json");
    assert!(
        plan::parse_with_value_matches(explicit, true)
            .unwrap()
            .configured_value_matches
            .is_empty()
    );
    let mut producer: Value = serde_json::from_str(FIXTURE).unwrap();
    let mut consumer = producer.clone();
    producer["resource_changes"]
        .as_array_mut()
        .unwrap()
        .retain(|n| {
            n["type"] != "cloudflare_workers_route"
                && n["type"] != "cloudflare_workers_custom_domain"
        });
    consumer["resource_changes"]
        .as_array_mut()
        .unwrap()
        .retain(|n| {
            n["type"] == "cloudflare_workers_route"
                || n["type"] == "cloudflare_workers_custom_domain"
        });
    let result = crate::generate_named(
        &[
            crate::NamedInput {
                state: "producer".into(),
                json: producer.to_string(),
            },
            crate::NamedInput {
                state: "consumer".into(),
                json: consumer.to_string(),
            },
        ],
        &Options {
            cloudflare_value_matches: true,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(
        !result
            .content
            .contains("data-evidence=\"configured-value\"")
    );
}

#[test]
fn large_literal_imports_use_state_local_typed_indexes() {
    let mut value: Value = serde_json::from_str(FIXTURE).unwrap();
    let route = value["resource_changes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["type"] == "cloudflare_workers_route")
        .unwrap()
        .clone();
    let resources = value["configuration"]["root_module"]["resources"]
        .as_array_mut()
        .unwrap();
    for index in 0..1000 {
        resources.push(json!({"address":format!("cloudflare_workers_route.batch_{index}"),"provider_config_key":"cloudflare"}));
    }
    for index in 0..1000 {
        let mut node = route.clone();
        node["address"] = json!(format!("cloudflare_workers_route.batch_{index}"));
        value["resource_changes"].as_array_mut().unwrap().push(node);
    }
    let start = std::time::Instant::now();
    assert_eq!(count(&value, "script"), 1001);
    eprintln!(
        "1000 imported routes parsed and indexed in {:?}",
        start.elapsed()
    );
}

#[test]
fn data_zones_aliases_and_domain_accounts_fail_closed() {
    let original: Value = serde_json::from_str(FIXTURE).unwrap();
    let mut data = original.clone();
    let zone = data["resource_changes"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|n| n["type"] == "cloudflare_zone")
        .unwrap();
    zone["address"] = json!("data.cloudflare_zone.example");
    zone["mode"] = json!("data");
    zone["change"]["after"]["zone_id"] = zone["change"]["after"]["id"].clone();
    assert_eq!(
        count(&data, "script"),
        0,
        "missing provider configuration is not the managed alias"
    );
    data["configuration"]["root_module"]["resources"].as_array_mut().unwrap().push(json!({"address":"data.cloudflare_zone.example", "mode":"data", "provider_config_key":"cloudflare"}));
    assert_eq!(count(&data, "script"), 1);
    let zone = data["resource_changes"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|n| n["type"] == "cloudflare_zone")
        .unwrap();
    zone["change"]["after"]["zone_id"] = json!("cccccccccccccccccccccccccccccccc");
    assert_eq!(count(&data, "script"), 0);
    let mut different_account = original;
    let domain = different_account["resource_changes"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|n| n["type"] == "cloudflare_workers_custom_domain")
        .unwrap();
    domain["change"]["after"]["account_id"] = json!("dddddddddddddddddddddddddddddddd");
    assert_eq!(count(&different_account, "service"), 0);
}

#[test]
fn recovery_requires_known_provider_configuration_at_each_endpoint() {
    let original: Value = serde_json::from_str(FIXTURE).unwrap();
    let mut absent = original.clone();
    absent.as_object_mut().unwrap().remove("configuration");
    assert!(
        plan::parse_with_value_matches(&absent.to_string(), true)
            .unwrap()
            .configured_value_matches
            .is_empty()
    );
    let mut missing_keys = original.clone();
    for resource in missing_keys["configuration"]["root_module"]["resources"]
        .as_array_mut()
        .unwrap()
    {
        resource
            .as_object_mut()
            .unwrap()
            .remove("provider_config_key");
    }
    assert!(
        plan::parse_with_value_matches(&missing_keys.to_string(), true)
            .unwrap()
            .configured_value_matches
            .is_empty()
    );
    for address in [
        "cloudflare_zone.example",
        "cloudflare_worker.api",
        "cloudflare_workers_route.api",
    ] {
        let mut missing = original.clone();
        let resource = missing["configuration"]["root_module"]["resources"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|resource| resource["address"] == address)
            .unwrap();
        resource
            .as_object_mut()
            .unwrap()
            .remove("provider_config_key");
        assert_eq!(count(&missing, "script"), 0, "{address}");
        if address == "cloudflare_zone.example" {
            assert_eq!(count(&missing, "zone_id"), 0);
        }
        if address == "cloudflare_workers_route.api" {
            assert_eq!(count(&missing, "zone_id"), 1);
        }
    }
}
