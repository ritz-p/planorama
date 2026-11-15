use crate::{model::EdgeKind, plan, semantic, view};
use serde_json::{Value, json};
const FIXTURE: &str = include_str!("../../fixtures/glue-workloads-plan.json");
const REFERENCES: &[(usize, usize, &str, &str)] = &[
    (12, 1, "connections", "name"),
    (12, 2, "security_configuration", "name"),
    (13, 3, "classifiers", "name"),
    (13, 2, "security_configuration", "name"),
    (13, 1, "jdbc_target.connection_name", "name"),
    (13, 1, "mongodb_target.connection_name", "name"),
    (13, 1, "s3_target.connection_name", "name"),
    (13, 1, "catalog_target.connection_name", "name"),
    (13, 1, "delta_target.connection_name", "name"),
    (13, 1, "hudi_target.connection_name", "name"),
    (13, 1, "iceberg_target.connection_name", "name"),
    (13, 5, "s3_target.event_queue_arn", "arn"),
    (13, 6, "s3_target.dlq_event_queue_arn", "arn"),
    (13, 5, "catalog_target.event_queue_arn", "arn"),
    (13, 6, "catalog_target.dlq_event_queue_arn", "arn"),
    (13, 7, "dynamodb_target.path", "name"),
    (14, 0, "role_arn", "arn"),
    (14, 8, "input_record_tables.database_name", "name"),
    (14, 9, "input_record_tables.table_name", "name"),
    (14, 1, "input_record_tables.connection_name", "name"),
    (15, 11, "workflow_name", "name"),
    (15, 12, "actions.job_name", "name"),
    (15, 13, "actions.crawler_name", "name"),
    (15, 2, "actions.security_configuration", "name"),
    (15, 12, "predicate.conditions.job_name", "name"),
    (15, 13, "predicate.conditions.crawler_name", "name"),
];
fn fixture() -> Value {
    serde_json::from_str(FIXTURE).unwrap()
}
fn graph(v: &Value) -> crate::model::ArchitectureGraph {
    semantic::transform(&plan::evidence::tests::parse_fixture(&v.to_string()).unwrap())
}
fn complete(v: &Value, t: usize, p: &str) -> bool {
    graph(v)
        .nodes
        .iter()
        .find(|n| n.address == v["resource_changes"][t]["address"])
        .unwrap()
        .references
        .iter()
        .any(|r| {
            r.attribute == p
                && r.complete
                && Some(r.label)
                    == super::label(v["resource_changes"][t]["type"].as_str().unwrap(), p)
        })
}
fn set(root: &mut Value, path: &str, value: Value) {
    if path == "actions.crawler_name" {
        root["actions"][1]["crawler_name"] = value;
        return;
    }
    if path == "predicate.conditions.crawler_name" {
        root["predicate"][0]["conditions"][1]["crawler_name"] = value;
        return;
    }
    let mut at = root;
    let mut parts = path.split('.').peekable();
    while let Some(p) = parts.next() {
        if parts.peek().is_none() {
            at[p] = value;
            return;
        }
        at = &mut at[p][0];
    }
}
#[test]
fn glue_workloads_verify_qualified_references_and_keep_consumers_independent() {
    let v = fixture();
    for &(t, _, p, _) in REFERENCES {
        assert!(complete(&v, t, p), "{t} {p}");
    }
    let g = graph(&v);
    assert_eq!(g.coverage.selected.resources, 20);
    for t in [9, 10, 12, 13, 14] {
        let n = g
            .nodes
            .iter()
            .find(|n| n.address == v["resource_changes"][t]["address"])
            .unwrap();
        assert!(n.member_owner.is_none());
        assert!(
            !g.edges
                .iter()
                .any(|e| e.kind == EdgeKind::Containment && g.nodes[e.to].address == n.address)
        );
    }
    assert!(
        g.nodes
            .iter()
            .find(|n| n.resource_type == "aws_glue_trigger")
            .unwrap()
            .member_owner
            .is_some()
    );
    for (t, p) in [(12, "role_arn"), (13, "role")] {
        assert!(
            g.nodes
                .iter()
                .find(|n| n.address == v["resource_changes"][t]["address"])
                .unwrap()
                .references
                .iter()
                .any(|r| r.attribute == p && r.complete && r.label == "Execution role reference")
        );
    }
    for format in [crate::OutputFormat::Svg, crate::OutputFormat::Html] {
        let out = crate::generate(
            FIXTURE,
            &crate::Options {
                format,
                ..Default::default()
            },
        )
        .unwrap()
        .content;
        assert!(!out.contains("PRIVATE_"));
        assert!(out.contains("Configured job"));
    }
}
#[test]
fn glue_workloads_reject_unknown_literal_foreign_historical_and_wrong_outputs() {
    for &(t, s, p, output) in REFERENCES {
        for case in [
            "literal",
            "unknown",
            "source-unknown",
            "wrong-output",
            "wrong-type",
            "mismatch",
            "delete",
            "deposed",
            "data",
            "consumer-delete",
            "consumer-deposed",
            "consumer-data",
            "provider",
            "region",
            "account",
            "ambiguous",
        ] {
            let mut v = fixture();
            match case {
                "literal" | "ambiguous" => set(
                    &mut v["configuration"]["root_module"]["resources"][t]["expressions"],
                    p,
                    if case == "literal" {
                        json!({"constant_value":"literal"})
                    } else {
                        json!({"references":["var.dynamic"]})
                    },
                ),
                "unknown" => {
                    v["resource_changes"][t]["change"]["after_unknown"] =
                        json!({p.split('.').next().unwrap():true})
                }
                "source-unknown" => {
                    v["resource_changes"][s]["change"]["after_unknown"] = json!({output:true})
                }
                "wrong-output" => {
                    let a = v["resource_changes"][s]["address"]
                        .as_str()
                        .unwrap()
                        .to_string();
                    set(
                        &mut v["configuration"]["root_module"]["resources"][t]["expressions"],
                        p,
                        json!({"references":[format!("{a}.wrong")]}),
                    );
                }
                "wrong-type" => v["resource_changes"][s]["type"] = json!("aws_s3_bucket"),
                "mismatch" => {
                    v["resource_changes"][s]["change"]["after"][output] = json!("different")
                }
                "delete" | "consumer-delete" => {
                    v["resource_changes"][if case == "delete" { s } else { t }]["change"]["actions"] =
                        json!(["delete"])
                }
                "deposed" | "consumer-deposed" => {
                    v["resource_changes"][if case == "deposed" { s } else { t }]["deposed"] =
                        json!("old")
                }
                "data" | "consumer-data" => {
                    v["resource_changes"][if case == "data" { s } else { t }]["mode"] =
                        json!("data")
                }
                "provider" => {
                    v["resource_changes"][s]["provider_name"] = json!("example.org/foreign/aws")
                }
                "region" | "account" => {
                    let mut config = v["configuration"]["provider_config"]["aws"].clone();
                    config["alias"] = json!("other");
                    if case == "region" {
                        config["expressions"]["region"]["constant_value"] = json!("us-west-2");
                    } else {
                        config["expressions"]["assume_role"][0]["role_arn"]["constant_value"] =
                            json!("arn:aws:iam::999999999999:role/Test");
                    }
                    v["configuration"]["provider_config"]["aws.other"] = config;
                    v["configuration"]["root_module"]["resources"][s]["provider_config_key"] =
                        json!("aws.other");
                }
                _ => unreachable!(),
            }
            assert!(!complete(&v, t, p), "{t} {p} {case}");
        }
    }
}
#[test]
fn glue_workloads_connection_names_need_catalog_account_evidence() {
    let v = fixture();
    let plain = semantic::transform(&plan::parse(FIXTURE).unwrap());
    for &(t, _, p, _) in REFERENCES.iter().filter(|(t, _, p, _)| {
        super::needs_catalog_account(v["resource_changes"][*t]["type"].as_str().unwrap(), p)
    }) {
        assert!(
            !plain
                .nodes
                .iter()
                .find(|n| n.address == v["resource_changes"][t]["address"])
                .unwrap()
                .references
                .iter()
                .any(|r| r.attribute == p && r.label == "Glue connections" && r.complete)
        );
        for catalog in [json!(null), json!("999999999999"), json!("invalid")] {
            let mut v = fixture();
            v["resource_changes"][1]["change"]["after"]["catalog_id"] = catalog;
            assert!(!complete(&v, t, p));
        }
    }
    for field in ["catalog_id", "database_name"] {
        let mut v = fixture();
        v["resource_changes"][14]["change"]["after"]["input_record_tables"][0][field] =
            json!("different");
        assert!(!complete(&v, 14, "input_record_tables.table_name"));
    }
    let mut v = fixture();
    v["resource_changes"][14]["change"]["after"]["input_record_tables"][1]["catalog_id"] =
        Value::Null;
    assert!(!complete(&v, 14, "input_record_tables.connection_name"));
    let g = graph(&fixture());
    let crawler = g
        .nodes
        .iter()
        .find(|n| n.resource_type == "aws_glue_crawler")
        .unwrap();
    assert!(
        !crawler
            .references
            .iter()
            .any(|r| r.attribute == "database_name" && r.label == "Transform input database")
    );
}
#[test]
fn glue_workloads_collections_require_every_value_and_indexed_row() {
    for (t, p) in [(12, "connections"), (13, "classifiers")] {
        for values in [
            json!([]),
            json!(["shared", "literal"]),
            json!(["shared", "shared"]),
            json!("shared"),
        ] {
            let mut v = fixture();
            v["resource_changes"][t]["change"]["after"][p] = values;
            assert!(!complete(&v, t, p));
        }
    }
    for (t, block, path) in [
        (13, "jdbc_target", "jdbc_target.connection_name"),
        (14, "input_record_tables", "input_record_tables.table_name"),
        (15, "actions", "actions.job_name"),
    ] {
        let mut v = fixture();
        let row = v["resource_changes"][t]["change"]["after"][block][0].clone();
        v["resource_changes"][t]["change"]["after"][block] = row;
        assert!(!complete(&v, t, path));
        let mut v = fixture();
        v["configuration"]["root_module"]["resources"][t]["expressions"][block]
            .as_array_mut()
            .unwrap()
            .remove(0);
        assert!(!complete(&v, t, path));
    }
    let mut v = fixture();
    v["configuration"]["root_module"]["resources"][13]["expressions"]["jdbc_target"]
        .as_array_mut()
        .unwrap()
        .reverse();
    assert!(!complete(&v, 13, "jdbc_target.connection_name"));
}
#[test]
fn glue_workloads_trigger_modes_and_target_unions_do_not_infer_execution() {
    for mode in [json!(null), json!("FUTURE"), json!("ON_DEMAND")] {
        let mut v = fixture();
        v["resource_changes"][15]["change"]["after"]["type"] = mode;
        assert!(!complete(&v, 15, "predicate.conditions.job_name"));
    }
    for (block, path) in [
        ("actions", "actions.job_name"),
        ("predicate", "predicate.conditions.job_name"),
    ] {
        let mut v = fixture();
        let row = if block == "actions" {
            &mut v["resource_changes"][15]["change"]["after"]["actions"][0]
        } else {
            &mut v["resource_changes"][15]["change"]["after"]["predicate"][0]["conditions"][0]
        };
        row["crawler_name"] = json!("crawler");
        assert!(!complete(&v, 15, path));
    }
    let mut v = fixture();
    let row = v["resource_changes"][15]["change"]["after"]["predicate"][0].clone();
    v["resource_changes"][15]["change"]["after"]["predicate"] = json!([row.clone(), row]);
    assert!(!complete(&v, 15, "predicate.conditions.job_name"));
    let mut v = fixture();
    v["resource_changes"][15]["change"]["after"]["actions"][0]["job_name"] = Value::Null;
    assert!(!complete(&v, 15, "actions.security_configuration"));
}
#[test]
fn glue_workloads_retain_actions_filter_fallback_and_named_states() {
    let mut v = fixture();
    v["resource_changes"][15]["previous_address"] = json!("aws_glue_trigger.old");
    v["resource_changes"][15]["change"]["actions"] = json!(["delete", "create"]);
    let parsed = plan::evidence::tests::parse_fixture(&v.to_string()).unwrap();
    let g = semantic::transform(&parsed);
    for n in &parsed.nodes {
        let matches: Vec<_> = g
            .nodes
            .iter()
            .filter(|other| other.address == n.address && other.deposed_key == n.deposed_key)
            .collect();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].action, n.action);
        assert_eq!(matches[0].previous_address, n.previous_address);
    }
    let selected = view::single(
        &g,
        &view::Options {
            focus: Some("aws_glue_trigger.trigger".into()),
            depth: Some(0),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(selected.coverage.selected.resources, 1);
    assert_eq!(selected.nodes.len(), 1);
    assert_eq!(selected.nodes[0].address, "aws_glue_trigger.trigger");
    let mut missing = fixture();
    missing["resource_changes"]
        .as_array_mut()
        .unwrap()
        .remove(12);
    for format in [crate::OutputFormat::Svg, crate::OutputFormat::Html] {
        let out = crate::generate_named(
            &[
                crate::NamedInput {
                    state: "one".into(),
                    json: FIXTURE.into(),
                },
                crate::NamedInput {
                    state: "two".into(),
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
        assert!(!out.contains("PRIVATE_"));
        assert!(!out.contains("<metadata id=\"cross-state-relationships\">"));
    }
}
