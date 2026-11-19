use serde_json::{Value, json};
const FIXTURE: &str = include_str!("../../fixtures/dms-events-plan.json");
fn fixture() -> Value {
    serde_json::from_str(FIXTURE).unwrap()
}
fn graph(v: &Value) -> crate::model::ArchitectureGraph {
    crate::semantic::transform(&crate::plan::parse(&v.to_string()).unwrap())
}
fn proven(v: &Value, address: &str, path: &str) -> bool {
    graph(v)
        .nodes
        .iter()
        .find(|n| n.address == address)
        .is_some_and(|n| {
            n.references.iter().any(|r| {
                r.attribute == path
                    && Some(r.label) == super::label(&n.resource_type, path)
                    && r.complete
                    && !r.dependency
                    && !r.targets.is_empty()
            })
        })
}
#[test]
fn dms_events_preserve_configured_helpers_and_shared_endpoints() {
    let v = fixture();
    let raw = crate::plan::parse(FIXTURE).unwrap();
    let g = graph(&v);
    let mut checked = 0;
    for r in &raw.attributes {
        let n = &raw.nodes[r.target];
        if super::label(&n.resource_type, &r.attribute).is_some() {
            assert!(
                proven(&v, &n.address, &r.attribute),
                "{} {} {r:?}",
                n.address,
                r.attribute
            );
            checked += 1;
        }
    }
    assert!(checked >= 10, "{checked}");
    assert_eq!(g.nodes.len(), raw.nodes.len());
    assert_eq!(g.coverage.selected.resources, raw.nodes.len());
    for n in &raw.nodes {
        let nodes: Vec<_> = g.nodes.iter().filter(|a| a.address == n.address).collect();
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].action, n.action);
    }
}
#[test]
fn dms_event_links_reject_incompatible_missing_and_lifecycle_evidence() {
    let base = fixture();
    let raw = crate::plan::parse(FIXTURE).unwrap();
    for r in &raw.attributes {
        let n = &raw.nodes[r.target];
        if super::label(&n.resource_type, &r.attribute).is_none() {
            continue;
        }
        let si = base["resource_changes"]
            .as_array()
            .unwrap()
            .iter()
            .position(|v| v["address"] == raw.nodes[r.sources[0]].address)
            .unwrap();
        let ti = base["resource_changes"]
            .as_array()
            .unwrap()
            .iter()
            .position(|v| v["address"] == n.address)
            .unwrap();
        let field = r.source_attributes_by_instance[&r.sources[0]]
            .first()
            .unwrap();
        for case in 0..10 {
            let mut v = base.clone();
            match case {
                0 => v["resource_changes"][si]["change"]["after"][field] = json!("other"),
                1 => v["resource_changes"][si]["change"]["after_unknown"] = json!({field:true}),
                2 => v["resource_changes"][si]["change"]["after_sensitive"] = json!({field:true}),
                3 => v["resource_changes"][si]["mode"] = json!("data"),
                4 => v["resource_changes"][si]["deposed"] = json!("old"),
                5 => v["resource_changes"][si]["change"]["actions"] = json!(["delete"]),
                6 => v["resource_changes"][si]["type"] = json!("aws_sqs_queue"),
                7 => {
                    v["resource_changes"][si]["provider_name"] =
                        json!("registry.terraform.io/hashicorp/google")
                }
                8 => v["configuration"]["root_module"]["resources"][ti]["expressions"] = json!({}),
                _ => v["resource_changes"][ti]["change"]["after_unknown"] = json!(true),
            }
            assert!(
                !proven(&v, &n.address, &r.attribute),
                "{} {} case{case}",
                n.address,
                r.attribute
            );
        }
    }
}
fn index(v: &Value, address: &str) -> usize {
    v["resource_changes"]
        .as_array()
        .unwrap()
        .iter()
        .position(|n| n["address"] == address)
        .unwrap()
}
#[test]
fn event_sources_require_the_exact_type_and_complete_identifier_set() {
    let base = fixture();
    let address = "aws_dms_event_subscription.instances";
    let i = index(&base, address);
    for case in 0..8 {
        let mut v = base.clone();
        match case {
            0 => {
                v["resource_changes"][i]["change"]["after"]["source_type"] =
                    json!("replication-task")
            }
            1 => v["resource_changes"][i]["change"]["after_unknown"] = json!({"source_type":true}),
            2 => {
                v["resource_changes"][i]["change"]["after"]["source_ids"] =
                    json!(["instance-first", "missing"])
            }
            3 => {
                v["resource_changes"][i]["change"]["after"]["source_ids"] =
                    json!(["instance-first", "instance-first"])
            }
            4 => {
                v["configuration"]["root_module"]["resources"][i]["expressions"]["source_ids"] = json!({"references":["aws_dms_replication_instance.first.replication_instance_id"]})
            }
            5 => v["resource_changes"][i]["change"]["after"]["source_ids"] = json!([]),
            6 => v["resource_changes"][i]["change"]["after"]["source_ids"] = Value::Null,
            _ => {
                v["configuration"]["root_module"]["resources"][i]["expressions"]["source_ids"] = json!({"references":["aws_dms_replication_instance.first.replication_instance_arn","aws_dms_replication_instance.second.replication_instance_arn"]})
            }
        }
        assert!(!proven(&v, address, "source_ids"), "case{case}");
        assert!(proven(&v, address, "sns_topic_arn"));
    }
    let g = graph(&base);
    let n = g
        .nodes
        .iter()
        .find(|n| n.address == "aws_dms_event_subscription.all")
        .unwrap();
    assert!(
        !n.references
            .iter()
            .any(|r| r.attribute == "source_ids" && r.complete)
    );
    assert!(n.member_owner.is_none());
    assert_eq!(g.coverage.selected.resources, 11);
}
#[test]
fn kerberos_payloads_do_not_qualify_links_and_computed_keys_need_configuration() {
    let address = "aws_dms_replication_instance.first";
    let mut v = fixture();
    let i = index(&v, address);
    v["resource_changes"][i]["change"]["after_unknown"] =
        json!({"kerberos_authentication_settings":[{"krb5_file_contents":true}]});
    v["resource_changes"][i]["change"]["after_sensitive"] =
        json!({"kerberos_authentication_settings":[{"krb5_file_contents":true}]});
    assert!(proven(
        &v,
        address,
        "kerberos_authentication_settings.key_cache_secret_id"
    ));
    for case in 0..4 {
        let mut v = fixture();
        let i = index(&v, address);
        match case {
            0 => {
                v["configuration"]["root_module"]["resources"][i]["expressions"]["kms_key_arn"] =
                    json!({})
            }
            1 => v["resource_changes"][i]["change"]["after_unknown"] = json!({"kms_key_arn":true}),
            2 => {
                let field = &mut v["resource_changes"][i]["change"]["after"]["kms_key_arn"];
                *field = json!([field.clone()]);
            }
            _ => {
                v["resource_changes"][i]["change"]["after_sensitive"] = json!({"kms_key_arn":true})
            }
        }
        assert!(!proven(&v, address, "kms_key_arn"));
    }
    let mut v = fixture();
    let i = index(&v, address);
    let row =
        v["resource_changes"][i]["change"]["after"]["kerberos_authentication_settings"][0].clone();
    v["resource_changes"][i]["change"]["after"]["kerberos_authentication_settings"]
        .as_array_mut()
        .unwrap()
        .push(row);
    assert!(!proven(
        &v,
        address,
        "kerberos_authentication_settings.key_cache_secret_id"
    ));
}
#[test]
fn dms_events_preserve_scope_filtering_and_named_state_boundaries() {
    let base = fixture();
    let mut v = base.clone();
    let si = index(&v, "aws_sns_topic.events");
    v["configuration"]["provider_config"]["aws.other"] = json!({"full_name":"registry.terraform.io/hashicorp/aws","expressions":{"region":{"constant_value":"us-west-2"}}});
    v["configuration"]["root_module"]["resources"][si]["provider_config_key"] = json!("aws.other");
    assert!(!proven(
        &v,
        "aws_dms_event_subscription.instances",
        "sns_topic_arn"
    ));
    let mut raw = crate::plan::parse(FIXTURE).unwrap();
    for n in &mut raw.nodes {
        n.provider_configuration.as_mut().unwrap().account =
            crate::model::AwsAccountScope::new(if n.resource_type == "aws_sns_topic" {
                "222222222222"
            } else {
                "111111111111"
            });
    }
    let g = crate::semantic::transform(&raw);
    assert!(
        !g.nodes
            .iter()
            .flat_map(|n| &n.references)
            .any(|r| r.label == "DMS event topic" && r.complete)
    );
    let selected = crate::view::single(
        &graph(&base),
        &crate::view::Options {
            focus: Some("aws_dms_replication_instance.first".into()),
            depth: Some(0),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(selected.coverage.selected.resources, 1);
    let mut missing = base;
    missing["resource_changes"]
        .as_array_mut()
        .unwrap()
        .remove(si);
    for format in [crate::OutputFormat::Svg, crate::OutputFormat::Html] {
        let output = crate::generate_named(
            &[
                crate::NamedInput {
                    state: "first".into(),
                    json: FIXTURE.into(),
                },
                crate::NamedInput {
                    state: "second".into(),
                    json: missing.to_string(),
                },
            ],
            &crate::Options {
                format,
                ..Default::default()
            },
        )
        .unwrap()
        .content;
        assert!(!output.contains("PRIVATE_"));
        assert!(!output.contains("<metadata id=\"cross-state-relationships\">"));
    }
}
