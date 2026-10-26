use crate::{
    model::{EdgeKind, Placement},
    plan, semantic, view,
};
use serde_json::{Value, json};

const FIXTURE: &str = include_str!("../../fixtures/elasticache-configuration-plan.json");

#[test]
fn cache_associations_preserve_lifecycle_and_named_state_boundaries() {
    let mut value: Value = serde_json::from_str(FIXTURE).unwrap();
    value["resource_changes"][4]["previous_address"] =
        json!("aws_elasticache_user_group_association.old");
    value["resource_changes"][4]["change"]["actions"] = json!(["update"]);
    value["resource_changes"][4]["change"]["importing"] = json!({"id":"PRIVATE_IMPORT"});
    let mut old = value["resource_changes"][4].clone();
    old["deposed"] = json!("old");
    old["change"]["actions"] = json!(["delete"]);
    value["resource_changes"].as_array_mut().unwrap().push(old);
    let parsed = plan::parse(&value.to_string()).unwrap();
    let graph = semantic::transform(&parsed);
    assert_eq!(graph.coverage.selected.resources, 12);
    for source in &parsed.nodes {
        let nodes: Vec<_> = graph
            .nodes
            .iter()
            .filter(|n| n.address == source.address && n.deposed_key == source.deposed_key)
            .collect();
        assert_eq!(nodes.len(), 1);
        let node = nodes[0];
        assert_eq!(node.metadata, source.metadata);
        assert_eq!(node.previous_address, source.previous_address);
        assert_eq!(node.action, source.action);
        if source.deposed_key.is_some() {
            assert!(
                !node.references.iter().any(|r| r.complete
                    && super::label(&node.resource_type, &r.attribute) == Some(r.label))
            );
        }
    }
    let mut missing = value.clone();
    missing["resource_changes"]
        .as_array_mut()
        .unwrap()
        .remove(1);
    for format in [crate::OutputFormat::Svg, crate::OutputFormat::Html] {
        let output = crate::generate_named(
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
        .unwrap()
        .content;
        assert!(output.contains("data-state-id=\"one\""));
        assert!(output.contains("data-state-id=\"two\""));
        assert!(!output.contains("<metadata id=\"cross-state-relationships\">"));
        assert!(!output.contains("PRIVATE_"));
    }
    let graph = semantic::transform(&plan::parse(&missing.to_string()).unwrap());
    assert!(
        !graph
            .nodes
            .iter()
            .flat_map(|n| &n.references)
            .any(|r| r.label == "Cache user membership" && r.complete)
    );
}

#[test]
fn cache_references_preserve_shared_identity_scope_and_safe_output() {
    let graph = semantic::transform(&plan::evidence::tests::parse_fixture(FIXTURE).unwrap());
    assert_eq!(graph.coverage.selected.resources, 11);
    let counts = [0, 0, 0, 1, 2, 2, 1, 1, 1, 1, 1];
    let fixture: Value = serde_json::from_str(FIXTURE).unwrap();
    for node in &graph.nodes {
        let index = fixture["resource_changes"]
            .as_array()
            .unwrap()
            .iter()
            .position(|n| n["address"] == node.address)
            .unwrap();
        let refs: Vec<_> = node
            .references
            .iter()
            .filter(|r| super::label(&node.resource_type, &r.attribute) == Some(r.label))
            .collect();
        assert_eq!(refs.len(), counts[index], "{}", node.address);
        assert!(
            refs.iter().all(|r| r.complete),
            "{}: {refs:?}",
            node.address
        );
        assert!(node.member_owner.is_none());
        if index <= 4 || index == 6 {
            assert_eq!(node.placement, Placement::Logical);
        }
        if index == 3 {
            assert_eq!(refs[0].targets.len(), 2);
        }
        if index == 6 {
            assert!(node.entity.scope.as_ref().unwrap().global);
            assert!(node.entity.scope.as_ref().unwrap().region.is_none());
        }
        if index == 7 {
            assert_eq!(
                node.entity
                    .scope
                    .as_ref()
                    .unwrap()
                    .region
                    .as_ref()
                    .unwrap()
                    .0,
                "us-west-2"
            );
        }
    }
    assert!(graph.edges.iter().all(|e| e.kind == EdgeKind::Dependency));
    let plain = semantic::transform(&plan::parse(FIXTURE).unwrap());
    assert!(
        !plain
            .nodes
            .iter()
            .find(|n| n.address == "aws_elasticache_replication_group.secondary")
            .unwrap()
            .references
            .iter()
            .any(|r| r.label == "Global datastore membership" && r.complete)
    );
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
        assert!(output.contains("Cache user membership"));
        assert!(output.contains("Configured primary replication group"));
    }
    let filtered = view::single(
        &graph,
        &view::Options {
            focus: Some("aws_elasticache_user_group_association.member".into()),
            depth: Some(0),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(filtered.coverage.selected.resources, 1);
    assert!(
        crate::svg::render(&filtered, &crate::layout::Layout::new(&filtered))
            .contains("Unavailable")
    );
}

#[test]
fn cache_links_require_exact_complete_current_endpoint_evidence() {
    let fixture: Value = serde_json::from_str(FIXTURE).unwrap();
    for index in 3..11 {
        for (field, expression) in
            fixture["configuration"]["root_module"]["resources"][index]["expressions"]
                .as_object()
                .unwrap()
        {
            let address = expression["references"][0]
                .as_str()
                .unwrap()
                .rsplit_once('.')
                .unwrap()
                .0;
            let endpoint = fixture["resource_changes"]
                .as_array()
                .unwrap()
                .iter()
                .position(|n| n["address"] == address)
                .unwrap();
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
                "mismatch",
                "wrong-field",
                "missing",
            ] {
                let mut v = fixture.clone();
                match case {
                    "literal" => {
                        v["configuration"]["root_module"]["resources"][index]["expressions"]
                            [field] = json!({"constant_value":"PRIVATE_LITERAL"})
                    }
                    "unknown" => {
                        v["resource_changes"][index]["change"]["after_unknown"][field] = json!(true)
                    }
                    "unknown-source" => {
                        v["resource_changes"][endpoint]["change"]["after_unknown"] = json!(true)
                    }
                    "ambiguous" => {
                        v["configuration"]["root_module"]["resources"][index]["expressions"][field]
                            ["references"]
                            .as_array_mut()
                            .unwrap()
                            .push(json!("var.unknown"))
                    }
                    "wrong-type" => {
                        v["resource_changes"][endpoint]["type"] = json!("aws_s3_bucket")
                    }
                    "wrong-provider" => {
                        v["resource_changes"][endpoint]["provider_name"] =
                            json!("example.com/foreign/aws")
                    }
                    "deleted" => {
                        v["resource_changes"][endpoint]["change"]["actions"] = json!(["delete"])
                    }
                    "deposed" => v["resource_changes"][endpoint]["deposed"] = json!("old"),
                    "data" => v["resource_changes"][endpoint]["mode"] = json!("data"),
                    "account" => {
                        let mut config = v["configuration"]["provider_config"]["aws"].clone();
                        config["expressions"]["assume_role"][0]["role_arn"]["constant_value"] =
                            json!("arn:aws:iam::999988887777:role/example");
                        v["configuration"]["provider_config"]["aws.other"] = config;
                        v["configuration"]["root_module"]["resources"][endpoint]["provider_config_key"] =
                            json!("aws.other");
                    }
                    "region" => {
                        if field == "primary_replication_group_id"
                            || field == "global_replication_group_id"
                        {
                            continue;
                        }
                        v["configuration"]["root_module"]["resources"][endpoint]["provider_config_key"] =
                            json!("aws.west");
                    }
                    "unknown-region" => {
                        for binding in ["aws", "aws.west"] {
                            v["configuration"]["provider_config"][binding]["expressions"]["region"] =
                                json!({"references":["var.region"]});
                        }
                    }
                    "mismatch" => {
                        v["resource_changes"][index]["change"]["after"][field] =
                            json!("PRIVATE_OTHER")
                    }
                    "wrong-field" => {
                        v["configuration"]["root_module"]["resources"][index]["expressions"]
                            [field] = json!({"references":[format!("{address}.arn")]})
                    }
                    "missing" => {
                        v["resource_changes"]
                            .as_array_mut()
                            .unwrap()
                            .remove(endpoint);
                    }
                    _ => unreachable!(),
                }
                let parsed = plan::evidence::tests::parse_fixture(&v.to_string()).unwrap();
                let graph = semantic::transform(&parsed);
                let node = graph
                    .nodes
                    .iter()
                    .find(|n| n.address == fixture["resource_changes"][index]["address"])
                    .unwrap();
                assert!(
                    !node.references.iter().any(|r| r.attribute == *field
                        && r.complete
                        && super::label(&node.resource_type, field) == Some(r.label)),
                    "{index} {field} {case}"
                );
            }
        }
    }
}

#[test]
fn cache_sets_reject_partial_duplicate_and_unknown_members() {
    for index in [3, 5] {
        let field = if index == 3 {
            "user_ids"
        } else {
            "user_group_ids"
        };
        for case in [
            "partial",
            "duplicate",
            "unknown-item",
            "mask-shape",
            "empty",
            "nonstring",
        ] {
            let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
            match case {
                "partial" => v["resource_changes"][index]["change"]["after"][field]
                    .as_array_mut()
                    .unwrap()
                    .push(json!("PRIVATE_UNREFERENCED")),
                "duplicate" => {
                    let first = v["resource_changes"][index]["change"]["after"][field][0].clone();
                    v["resource_changes"][index]["change"]["after"][field]
                        .as_array_mut()
                        .unwrap()
                        .push(first);
                }
                "unknown-item" => {
                    v["resource_changes"][index]["change"]["after_unknown"][field] = if index == 3 {
                        json!([false, true])
                    } else {
                        json!([true])
                    }
                }
                "mask-shape" => {
                    v["resource_changes"][index]["change"]["after_unknown"][field] =
                        json!({"bad":false})
                }
                "empty" => v["resource_changes"][index]["change"]["after"][field] = json!([]),
                _ => v["resource_changes"][index]["change"]["after"][field][0] = json!(4),
            }
            let graph = semantic::transform(&plan::parse(&v.to_string()).unwrap());
            let node = graph
                .nodes
                .iter()
                .find(|n| n.address == v["resource_changes"][index]["address"])
                .unwrap();
            assert!(
                !node.references.iter().any(|r| r.attribute == field
                    && r.complete
                    && super::label(&node.resource_type, field) == Some(r.label)),
                "{index} {case}"
            );
        }
    }
}
