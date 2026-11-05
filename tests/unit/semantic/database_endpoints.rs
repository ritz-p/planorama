use crate::{plan, semantic};
use serde_json::{Value, json};

fn fixture() -> Value {
    serde_json::from_str(include_str!("../../fixtures/database-endpoints-plan.json")).unwrap()
}

#[test]
fn proxy_endpoint_preserves_verified_account_panel() {
    let mut v = fixture();
    v["configuration"]["provider_config"]["aws"]["expressions"]["assume_role"] =
        json!([{"role_arn":{"constant_value":"arn:aws:iam::123456789012:role/Test"}}]);
    let parsed = crate::plan::evidence::tests::parse_fixture(&v.to_string()).unwrap();
    let graph = semantic::transform(&parsed);
    let endpoint = graph
        .nodes
        .iter()
        .position(|n| n.address == "aws_db_proxy_endpoint.custom")
        .unwrap();
    let vpc = graph
        .nodes
        .iter()
        .position(|n| n.address == "aws_vpc.main")
        .unwrap();
    assert!(graph.edges.iter().any(|e| e.from == vpc
        && e.to == endpoint
        && e.kind == crate::model::EdgeKind::Containment));
    assert_eq!(
        graph.nodes[endpoint].entity.scope,
        graph.nodes[vpc].entity.scope
    );
    let groups = crate::view::scope::groups(&graph, &crate::layout::ContainmentTree::new(&graph));
    let (scope, _) = groups
        .iter()
        .find(|(_, roots)| roots.contains(&vpc))
        .unwrap();
    assert_eq!(
        scope.as_ref().unwrap().account.as_ref().unwrap().as_str(),
        "123456789012"
    );
    let output = crate::svg::render(&graph, &crate::layout::Layout::new(&graph));
    let doc = roxmltree::Document::parse(&output).unwrap();
    assert!(
        doc.descendants()
            .any(|n| n.attribute("data-account") == Some("123456789012")
                && n.attribute("data-region") == Some("us-east-1"))
    );
}

#[test]
fn endpoint_named_states_preserve_coverage_and_privacy() {
    let v = fixture();
    let g = semantic::transform(&plan::parse(&v.to_string()).unwrap());
    assert_eq!(g.coverage.selected.resources, 15);
    let mut missing = v.clone();
    missing["resource_changes"]
        .as_array_mut()
        .unwrap()
        .remove(6);
    for format in [crate::OutputFormat::Svg, crate::OutputFormat::Html] {
        let result = crate::generate_named(
            &[
                crate::NamedInput {
                    state: "one".into(),
                    json: v.to_string(),
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
        assert!(!result.content.contains("PRIVATE_"));
        assert!(result.content.contains("data-state-id=\"one\""));
        assert!(result.content.contains("data-state-id=\"two\""));
    }
}

#[test]
fn custom_endpoint_requires_same_cluster_proof() {
    for invalid in [false, true] {
        let mut v = fixture();
        if invalid {
            v["resource_changes"][7]["change"]["after"]["cluster_identifier"] = json!("other");
        }
        let graph = semantic::transform(&plan::parse(&v.to_string()).unwrap());
        let endpoint = graph
            .nodes
            .iter()
            .find(|n| n.address == "aws_neptune_cluster_endpoint.custom")
            .unwrap();
        let members = endpoint
            .references
            .iter()
            .find(|r| r.attribute == "static_members" && r.label == "Static endpoint members")
            .unwrap();
        assert_eq!(members.complete, !invalid);
        assert_eq!(endpoint.placement, crate::model::Placement::Logical);
    }
}

#[test]
fn database_endpoint_rejects_unproven_references() {
    for (target, field, source) in [
        (9, "static_members", 7),
        (12, "excluded_members", 11),
        (14, "db_proxy_name", 13),
    ] {
        for case in [
            "valid", "unknown", "deleted", "deposed", "value", "output", "filtered",
        ] {
            let mut v = fixture();
            match case {
                "unknown" => {
                    v["resource_changes"][target]["change"]["after_unknown"] = json!({field:true})
                }
                "deleted" => v["resource_changes"][source]["change"]["actions"] = json!(["delete"]),
                "deposed" => v["resource_changes"][source]["deposed"] = json!("old"),
                "value" => {
                    v["resource_changes"][target]["change"]["after"][field] = json!("mismatch")
                }
                "output" => {
                    v["configuration"]["root_module"]["resources"][target]["expressions"][field] = json!({"references":[format!("{}.id", v["resource_changes"][source]["address"].as_str().unwrap())]})
                }
                "filtered" => {
                    v["resource_changes"].as_array_mut().unwrap().remove(source);
                }
                _ => {}
            }
            let g = semantic::transform(&plan::parse(&v.to_string()).unwrap());
            let address = fixture()["resource_changes"][target]["address"]
                .as_str()
                .unwrap()
                .to_owned();
            let node = g.nodes.iter().find(|n| n.address == address).unwrap();
            assert_eq!(
                node.references.iter().any(|r| r.attribute == field
                    && matches!(
                        r.label,
                        "Static endpoint members" | "Excluded endpoint members" | "Proxy owner"
                    )
                    && r.complete
                    && !r.targets.is_empty()),
                case == "valid",
                "{target} {case}"
            );
        }
    }
}

#[test]
fn proxy_endpoint_network_is_independent_of_proxy_owner() {
    for invalid_network in [false, true] {
        let mut v = fixture();
        if invalid_network {
            v["resource_changes"][14]["change"]["after"]["vpc_security_group_ids"] =
                json!(["missing"]);
        }
        let g = semantic::transform(&plan::parse(&v.to_string()).unwrap());
        let endpoint = g
            .nodes
            .iter()
            .position(|n| n.address == "aws_db_proxy_endpoint.custom")
            .unwrap();
        assert_eq!(
            g.nodes[endpoint].placement,
            crate::model::Placement::Spatial
        );
        assert_eq!(
            g.edges
                .iter()
                .any(|e| e.to == endpoint && e.kind == crate::model::EdgeKind::Containment),
            !invalid_network
        );
        assert!(
            g.nodes[endpoint]
                .references
                .iter()
                .any(|r| r.attribute == "db_proxy_name" && r.complete)
        );
    }
}
