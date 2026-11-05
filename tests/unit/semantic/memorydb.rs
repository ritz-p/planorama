use crate::{
    model::{EdgeKind, Placement},
    plan, semantic, view,
};
use serde_json::{Value, json};
const FIXTURE: &str = include_str!("../../fixtures/memorydb-plan.json");
#[test]
fn memorydb_membership_and_history_do_not_change_placement_or_ownership() {
    let graph = semantic::transform(&plan::parse(FIXTURE).unwrap());
    let node = |kind: &str| {
        graph
            .nodes
            .iter()
            .find(|n| n.resource_type == kind)
            .unwrap()
    };
    let global = node("aws_memorydb_multi_region_cluster");
    assert_eq!(global.placement, Placement::Logical);
    assert!(global.entity.scope.as_ref().unwrap().global);
    assert!(global.entity.scope.as_ref().unwrap().region.is_none());
    for kind in [
        "aws_memorydb_user",
        "aws_memorydb_acl",
        "aws_memorydb_snapshot",
    ] {
        let n = node(kind);
        assert_eq!(n.placement, Placement::Logical);
        assert!(n.member_owner.is_none());
        assert!(
            !graph
                .edges
                .iter()
                .any(|e| graph.nodes[e.to].address == n.address && e.kind == EdgeKind::Containment)
        );
    }
    let cluster = graph
        .nodes
        .iter()
        .find(|n| n.address == "aws_memorydb_cluster.cache")
        .unwrap();
    assert!(
        cluster
            .references
            .iter()
            .any(|r| r.attribute == "multi_region_cluster_name"
                && r.complete
                && r.kind == crate::model::ReferenceKind::Membership)
    );
    assert!(
        node("aws_memorydb_acl")
            .references
            .iter()
            .any(|r| r.attribute == "user_names"
                && r.complete
                && r.targets.len() == 2
                && r.kind == crate::model::ReferenceKind::Membership)
    );
    let snapshot = node("aws_memorydb_snapshot");
    assert!(
        snapshot
            .references
            .iter()
            .any(|r| r.attribute == "cluster_name"
                && r.complete
                && r.kind == crate::model::ReferenceKind::Configured)
    );
    let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
    v["configuration"]["root_module"]["resources"][13]["provider_config_key"] = json!("aws.west");
    let cross_region =
        semantic::transform(&plan::evidence::tests::parse_fixture(&v.to_string()).unwrap());
    assert!(
        cross_region
            .nodes
            .iter()
            .flat_map(|n| &n.references)
            .any(|r| r.attribute == "multi_region_cluster_name"
                && r.complete
                && r.label == "MemoryDB multi-region membership")
    );
    v["resource_changes"][13]["change"]["after"]["id"] = json!("PRIVATE_GLOBAL");
    v["configuration"]["root_module"]["resources"][8]["expressions"]["multi_region_cluster_name"] =
        json!({"references":["aws_memorydb_multi_region_cluster.global.id"]});
    let graph = semantic::transform(&plan::parse(&v.to_string()).unwrap());
    assert!(
        !graph
            .nodes
            .iter()
            .flat_map(|n| &n.references)
            .any(|r| r.attribute == "multi_region_cluster_name"
                && r.complete
                && r.label == "MemoryDB multi-region membership")
    );
}

#[test]
fn memorydb_preserves_shared_configuration_and_requires_common_vpc() {
    let fixture: Value = serde_json::from_str(FIXTURE).unwrap();
    let graph = semantic::transform(&plan::parse(FIXTURE).unwrap());
    assert_eq!(graph.coverage.selected.resources, 15);
    let cluster = graph
        .nodes
        .iter()
        .position(|n| n.address == "aws_memorydb_cluster.cache")
        .unwrap();
    assert_eq!(graph.nodes[cluster].placement, Placement::Spatial);
    assert_eq!(
        graph.nodes[cluster]
            .references
            .iter()
            .filter(
                |r| super::label("aws_memorydb_cluster", &r.attribute) == Some(r.label)
                    && r.complete
            )
            .count(),
        8
    );
    assert!(graph.edges.iter().any(|e| e.to == cluster
        && e.kind == EdgeKind::Containment
        && graph.nodes[e.from].resource_type == "aws_vpc"));
    for node in graph.nodes.iter().filter(|n| {
        matches!(
            n.resource_type.as_str(),
            "aws_memorydb_subnet_group" | "aws_memorydb_parameter_group"
        )
    }) {
        assert_eq!(node.placement, Placement::Logical);
        assert!(node.member_owner.is_none());
        assert!(!graph.edges.iter().any(|e| {
            e.to == graph
                .nodes
                .iter()
                .position(|n| n.address == node.address)
                .unwrap()
                && e.kind == EdgeKind::Containment
        }));
    }
    for (index, field) in [
        (1, "vpc_id"),
        (3, "vpc_id"),
        (4, "subnet_ids"),
        (8, "security_group_ids"),
    ] {
        let mut v = fixture.clone();
        v["resource_changes"][index]["change"]["after_unknown"][field] = json!(true);
        let g = semantic::transform(&plan::parse(&v.to_string()).unwrap());
        assert!(
            !g.edges
                .iter()
                .any(|e| g.nodes[e.to].resource_type == "aws_memorydb_cluster"
                    && e.kind == EdgeKind::Containment),
            "{field}"
        );
        assert!(
            crate::provider::aws::diagnostics::collect(&plan::parse(&v.to_string()).unwrap())
                .iter()
                .any(|d| d.address == "aws_memorydb_cluster.cache")
        );
    }
    let filtered = view::single(
        &graph,
        &view::Options {
            focus: Some("aws_memorydb_cluster.cache".into()),
            depth: Some(0),
            ..Default::default()
        },
    )
    .unwrap();
    assert!(
        crate::svg::render(&filtered, &crate::layout::Layout::new(&filtered))
            .contains("Unavailable")
    );
    assert!(
        !filtered
            .edges
            .iter()
            .any(|e| e.kind == EdgeKind::Containment
                && filtered.nodes[e.to].resource_type == "aws_memorydb_cluster")
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
        assert!(output.contains("MemoryDB parameter group"));
    }
}

#[test]
fn memorydb_preserves_lifecycle_and_separates_named_states() {
    let mut value: Value = serde_json::from_str(FIXTURE).unwrap();
    value["resource_changes"][8]["previous_address"] = json!("aws_memorydb_cluster.old");
    value["resource_changes"][8]["change"]["actions"] = json!(["update"]);
    value["resource_changes"][8]["change"]["importing"] = json!({"id":"PRIVATE_IMPORT"});
    let parsed = plan::parse(&value.to_string()).unwrap();
    let graph = semantic::transform(&parsed);
    for original in &parsed.nodes {
        let matches: Vec<_> = graph
            .nodes
            .iter()
            .filter(|n| n.address == original.address)
            .collect();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].metadata, original.metadata);
        assert_eq!(matches[0].previous_address, original.previous_address);
        assert_eq!(matches[0].action, original.action);
    }
    let mut missing = value.clone();
    missing["resource_changes"]
        .as_array_mut()
        .unwrap()
        .remove(4);
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
}

#[test]
fn memorydb_sets_reject_partial_duplicate_and_unknown_members() {
    for (index, field) in [
        (4, "subnet_ids"),
        (8, "security_group_ids"),
        (12, "user_names"),
    ] {
        for case in ["partial", "duplicate", "unknown-member", "empty"] {
            let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
            match case {
                "partial" => v["resource_changes"][index]["change"]["after"][field]
                    .as_array_mut()
                    .unwrap()
                    .push(json!("unreferenced")),
                "duplicate" => {
                    let member = v["resource_changes"][index]["change"]["after"][field][0].clone();
                    v["resource_changes"][index]["change"]["after"][field]
                        .as_array_mut()
                        .unwrap()
                        .push(member);
                }
                "unknown-member" => {
                    v["resource_changes"][index]["change"]["after_unknown"][field] = json!([true])
                }
                "empty" => v["resource_changes"][index]["change"]["after"][field] = json!([]),
                _ => unreachable!(),
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
                "{field}: {case}"
            );
        }
    }
}
#[test]
fn memorydb_links_require_exact_complete_current_endpoint_evidence() {
    let fixture: Value = serde_json::from_str(FIXTURE).unwrap();
    for index in 4..15 {
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
                        if field == "multi_region_cluster_name"
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
                            [field] = json!({"references":[format!("{address}.unsupported")]})
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
