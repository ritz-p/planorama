use crate::{
    model::{EdgeKind, Placement, ReferenceKind},
    plan, semantic, view,
};
use serde_json::{Value, json};

const FIXTURE: &str = include_str!("../../fixtures/dynamodb-plan.json");

#[test]
fn dynamodb_configuration_links_preserve_scope_identity_and_safe_output() {
    let plain = semantic::transform(&plan::parse(FIXTURE).unwrap());
    assert!(
        !plain
            .nodes
            .iter()
            .flat_map(|n| &n.references)
            .any(|r| r.label == "V2 table membership" && r.complete)
    );
    let graph = semantic::transform(&plan::evidence::tests::parse_fixture(FIXTURE).unwrap());
    assert_eq!(graph.coverage.selected.resources, 9);
    for node in &graph.nodes {
        assert!(node.member_owner.is_none());
        if node.resource_type == "aws_dynamodb_table" || node.resource_type == "aws_kinesis_stream"
        {
            continue;
        }
        assert_eq!(node.placement, Placement::Logical);
        let refs: Vec<_> = node
            .references
            .iter()
            .filter(|r| super::label(&node.resource_type, &r.attribute) == Some(r.label))
            .collect();
        assert_eq!(
            refs.len(),
            if node.resource_type == "aws_dynamodb_kinesis_streaming_destination" {
                2
            } else {
                1
            },
            "{}",
            node.address
        );
        assert!(
            refs.iter().all(|r| r.complete && r.targets.len() == 1),
            "{}: {refs:?}",
            node.address
        );
        if node.resource_type == "aws_dynamodb_global_table" {
            assert!(node.entity.scope.as_ref().unwrap().global);
            assert_eq!(refs[0].kind, ReferenceKind::Membership);
        } else {
            let scope = node.entity.scope.as_ref().unwrap();
            assert!(!scope.global);
            assert_eq!(
                scope.region.as_ref().unwrap().0,
                if node.resource_type == "aws_dynamodb_table_replica" {
                    "us-west-2"
                } else {
                    "us-east-1"
                }
            );
        }
    }
    assert!(graph.edges.iter().all(|e| e.kind == EdgeKind::Dependency));
    for format in [crate::OutputFormat::Svg, crate::OutputFormat::Html] {
        let output = crate::generate(
            FIXTURE,
            &crate::Options {
                format,
                ..Default::default()
            },
        )
        .unwrap()
        .content;
        assert!(!output.contains("PRIVATE_"));
        assert!(output.contains("V2 table membership"));
        assert!(output.contains("Legacy V1 table membership"));
        assert!(output.contains("Configured Kinesis stream"));
    }
    let filtered = view::single(
        &graph,
        &view::Options {
            focus: Some("aws_dynamodb_resource_policy.policy".into()),
            depth: Some(0),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(filtered.coverage.selected.resources, 1);
    let output = crate::svg::render(&filtered, &crate::layout::Layout::new(&filtered));
    assert!(output.contains("Unavailable"));
}

#[test]
fn dynamodb_requires_unique_current_explicit_compatible_evidence() {
    for index in 2..9 {
        let field = match index {
            2 => "global_table_arn",
            3 => "name",
            7 | 8 => "resource_arn",
            _ => "table_name",
        };
        for case in [
            "literal",
            "unknown",
            "unknown-source",
            "ambiguous",
            "wrong-type",
            "wrong-provider",
            "deleted",
            "deposed",
            "data",
            "account",
            "region",
            "unknown-region",
            "value-mismatch",
            "wrong-field",
        ] {
            let mut value: Value = serde_json::from_str(FIXTURE).unwrap();
            match case {
                "literal" => {
                    value["configuration"]["root_module"]["resources"][index]["expressions"]
                        [field] = json!({"constant_value":"PRIVATE_TABLE"})
                }
                "unknown" => {
                    value["resource_changes"][index]["change"]["after_unknown"][field] = json!(true)
                }
                "unknown-source" => {
                    value["resource_changes"][0]["change"]["after_unknown"] = json!(true)
                }
                "ambiguous" => {
                    value["configuration"]["root_module"]["resources"][index]["expressions"][field]
                        ["references"]
                        .as_array_mut()
                        .unwrap()
                        .push(json!("var.unresolved"))
                }
                "wrong-type" => value["resource_changes"][0]["type"] = json!("aws_s3_bucket"),
                "wrong-provider" => {
                    value["resource_changes"][0]["provider_name"] = json!("example.com/foreign/aws")
                }
                "deleted" => {
                    value["resource_changes"][index]["change"]["actions"] = json!(["delete"])
                }
                "deposed" => value["resource_changes"][index]["deposed"] = json!("old"),
                "data" => value["resource_changes"][index]["mode"] = json!("data"),
                "account" => {
                    let mut config = value["configuration"]["provider_config"]["aws"].clone();
                    config["expressions"]["assume_role"][0]["role_arn"]["constant_value"] =
                        json!("arn:aws:iam::999988887777:role/example");
                    value["configuration"]["provider_config"]["aws.other"] = config;
                    value["configuration"]["root_module"]["resources"][index]["provider_config_key"] =
                        json!("aws.other");
                }
                "region" => {
                    if index == 3 {
                        value["resource_changes"][index]["change"]["after"]["replica"] =
                            json!([{ "region_name": "eu-west-1" }]);
                    } else {
                        value["configuration"]["root_module"]["resources"][index]["provider_config_key"] =
                            json!(if index == 2 { "aws" } else { "aws.west" });
                    }
                }
                "unknown-region" => {
                    value["configuration"]["provider_config"]["aws"]["expressions"]["region"] =
                        json!({"references":["var.region"]})
                }
                "value-mismatch" => {
                    value["resource_changes"][index]["change"]["after"][field] =
                        json!("PRIVATE_OTHER")
                }
                "wrong-field" => {
                    value["configuration"]["root_module"]["resources"][index]["expressions"]
                        [field] = json!({"references":["aws_dynamodb_table.main.hash_key"]})
                }
                _ => unreachable!(),
            }
            let parsed = plan::evidence::tests::parse_fixture(&value.to_string()).unwrap();
            let graph = semantic::transform(&parsed);
            let node = graph
                .nodes
                .iter()
                .find(|n| {
                    n.address
                        == value["resource_changes"][index]["address"]
                            .as_str()
                            .unwrap()
                })
                .unwrap();
            assert!(
                !node.references.iter().any(|r| r.attribute == field
                    && r.complete
                    && super::label(&node.resource_type, field) == Some(r.label)),
                "{index} {case}"
            );
            if !matches!(case, "literal" | "data" | "deposed" | "deleted") {
                assert!(
                    semantic::diagnostics::collect(&parsed)
                        .iter()
                        .any(|d| d.address == node.address && d.attribute == field),
                    "missing diagnostic: {index} {case}"
                );
            }
        }
    }
}

#[test]
fn dynamodb_preserves_lifecycle_and_named_state_boundaries() {
    let mut value: Value = serde_json::from_str(FIXTURE).unwrap();
    value["resource_changes"][7]["previous_address"] = json!("aws_dynamodb_resource_policy.old");
    value["resource_changes"][7]["change"]["actions"] = json!(["update"]);
    value["resource_changes"][7]["change"]["importing"] = json!({"id":"PRIVATE_IMPORT"});
    let mut old = value["resource_changes"][7].clone();
    old["deposed"] = json!("old");
    old["change"]["actions"] = json!(["delete"]);
    value["resource_changes"].as_array_mut().unwrap().push(old);
    let parsed = plan::parse(&value.to_string()).unwrap();
    let graph = semantic::transform(&parsed);
    assert_eq!(graph.coverage.selected.resources, 10);
    for source in &parsed.nodes {
        let node = graph
            .nodes
            .iter()
            .find(|n| n.address == source.address && n.deposed_key == source.deposed_key)
            .unwrap();
        assert_eq!(node.metadata, source.metadata);
        assert_eq!(node.previous_address, source.previous_address);
        assert_eq!(node.action, source.action);
        if source.deposed_key.is_some() {
            assert!(
                !node
                    .references
                    .iter()
                    .any(|r| r.complete && r.label == "Configured table")
            );
        }
    }
    let mut missing = value.clone();
    missing["resource_changes"]
        .as_array_mut()
        .unwrap()
        .remove(0);
    for format in [crate::OutputFormat::Svg, crate::OutputFormat::Html] {
        let generated = crate::generate_named(
            &[
                crate::NamedInput {
                    state: "one".into(),
                    json: value.to_string(),
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
        .unwrap();
        assert!(generated.content.contains("data-state-id=\"one\""));
        assert!(generated.content.contains("data-state-id=\"two\""));
        assert!(
            !generated
                .content
                .contains("<metadata id=\"cross-state-relationships\">")
        );
        assert!(!generated.content.contains("PRIVATE_"));
    }
    let graph = semantic::transform(&plan::parse(&missing.to_string()).unwrap());
    assert_eq!(graph.coverage.selected.resources, 9);
    assert!(
        !graph
            .nodes
            .iter()
            .flat_map(|n| &n.references)
            .any(|r| r.label == "Configured table" && r.complete)
    );
}

#[test]
fn dynamodb_stream_and_replica_endpoints_do_not_accept_substituted_resources() {
    for field in ["arn", "name"] {
        for case in ["unknown", "foreign", "wrong-type", "ambiguous", "deleted"] {
            let mut value: Value = serde_json::from_str(FIXTURE).unwrap();
            match case {
                "unknown" => {
                    value["resource_changes"][1]["change"]["after_unknown"][field] = json!(true)
                }
                "foreign" => {
                    value["resource_changes"][1]["provider_name"] = json!("example.com/foreign/aws")
                }
                "wrong-type" => value["resource_changes"][1]["type"] = json!("aws_dynamodb_table"),
                "ambiguous" => {
                    value["configuration"]["root_module"]["resources"][4]["expressions"]["stream_arn"]
                        ["references"] = json!([
                        "aws_kinesis_stream.changes.arn",
                        "aws_dynamodb_table.main.arn"
                    ])
                }
                _ => value["resource_changes"][1]["change"]["actions"] = json!(["delete"]),
            }
            let graph = semantic::transform(&plan::parse(&value.to_string()).unwrap());
            assert_eq!(
                graph
                    .nodes
                    .iter()
                    .flat_map(|n| &n.references)
                    .any(|r| r.label == "Configured Kinesis stream" && r.complete),
                case == "unknown" && field == "name"
            );
        }
    }
    let mut value: Value = serde_json::from_str(FIXTURE).unwrap();
    for index in [7, 8] {
        value["resource_changes"][index]["change"]["after"]["resource_arn"] =
            value["resource_changes"][2]["change"]["after"]["arn"].clone();
        value["configuration"]["root_module"]["resources"][index]["provider_config_key"] =
            json!("aws.west");
        value["configuration"]["root_module"]["resources"][index]["expressions"]["resource_arn"] =
            json!({"references":["aws_dynamodb_table_replica.west.arn"]});
    }
    let graph = semantic::transform(&plan::parse(&value.to_string()).unwrap());
    for node in graph.nodes.iter().filter(|n| {
        matches!(
            n.resource_type.as_str(),
            "aws_dynamodb_resource_policy" | "aws_dynamodb_tag"
        )
    }) {
        assert!(node.references.iter().any(|r| r.label == "Configured table"
            && r.complete
            && r.targets[0].id.address == "aws_dynamodb_table_replica.west"));
    }
}

#[test]
fn dynamodb_legacy_membership_requires_known_unique_replica_regions() {
    for mask in [
        json!(true),
        json!({"replica":true}),
        json!({"replica":[{"region_name":true},{}]}),
        json!({"replica":[{}]}),
    ] {
        let mut value: Value = serde_json::from_str(FIXTURE).unwrap();
        value["resource_changes"][3]["change"]["after_unknown"] = mask;
        let graph = semantic::transform(&plan::parse(&value.to_string()).unwrap());
        assert!(
            !graph
                .nodes
                .iter()
                .find(|n| n.resource_type == "aws_dynamodb_global_table")
                .unwrap()
                .references
                .iter()
                .any(|r| r.complete && r.label == "Legacy V1 table membership")
        );
    }
}
