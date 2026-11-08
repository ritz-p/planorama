use crate::{plan, semantic};
use serde_json::{Value, json};

const FIXTURE: &str = include_str!("../../fixtures/secondary-network-plan.json");

#[test]
fn secondary_network_requires_current_explicit_matching_dedicated_identity() {
    for case in [
        "valid",
        "foreign",
        "sensitive",
        "endpoint-sensitive",
        "deleted",
        "deposed",
        "data",
        "region",
        "ambiguous",
        "indexed",
    ] {
        let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
        match case {
            "foreign" => {
                v["resource_changes"][0]["provider_name"] = json!("example.com/foreign/aws")
            }
            "sensitive" => {
                v["resource_changes"][1]["change"]["after_sensitive"] =
                    json!({"secondary_network_id":true})
            }
            "endpoint-sensitive" => {
                v["resource_changes"][0]["change"]["after_sensitive"] =
                    json!({"secondary_network_id":true})
            }
            "deleted" => v["resource_changes"][0]["change"]["actions"] = json!(["delete"]),
            "deposed" => v["resource_changes"][0]["deposed"] = json!("old"),
            "data" => v["resource_changes"][0]["mode"] = json!("data"),
            "region" => {
                v["configuration"]["root_module"]["resources"][0]["expressions"]["region"] =
                    json!({"constant_value":"us-west-2"})
            }
            "ambiguous" => {
                v["configuration"]["root_module"]["resources"][1]["expressions"]["secondary_network_id"] = json!({"references":["aws_ec2_secondary_network.main.secondary_network_id", "aws_ec2_secondary_network.missing.secondary_network_id"]})
            }
            "indexed" => {
                v = serde_json::from_str(&v.to_string().replace(
                    "aws_ec2_secondary_network.main",
                    "aws_ec2_secondary_network.main[0]",
                ))
                .unwrap();
                v["configuration"]["root_module"]["resources"][0]["address"] =
                    json!("aws_ec2_secondary_network.main");
            }
            _ => {}
        }
        let g = semantic::transform(&plan::parse(&v.to_string()).unwrap());
        assert_eq!(
            g.nodes
                .iter()
                .flat_map(|n| &n.references)
                .any(|r| r.label == "Secondary network" && r.complete),
            matches!(case, "valid" | "indexed"),
            "{case}"
        );
        assert_eq!(g.coverage.selected.resources, 2);
    }
}

#[test]
fn independent_containers_preserve_coverage_and_reject_invalid_networks() {
    for case in [
        "valid", "type", "unknown", "literal", "id", "value", "az-id",
    ] {
        let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
        match case {
            "type" => v["resource_changes"][0]["change"]["after"]["network_type"] = json!("other"),
            "unknown" => {
                v["resource_changes"][0]["change"]["after_unknown"] = json!({"network_type":true})
            }
            "literal" => {
                v["configuration"]["root_module"]["resources"][1]["expressions"] = json!({})
            }
            "id" => {
                v["configuration"]["root_module"]["resources"][1]["expressions"]["secondary_network_id"] =
                    json!({"references":["aws_ec2_secondary_network.main.id"]})
            }
            "value" => {
                v["resource_changes"][1]["change"]["after"]["secondary_network_id"] = json!("other")
            }
            "az-id" => {
                v["resource_changes"][1]["change"]["after"]
                    .as_object_mut()
                    .unwrap()
                    .remove("availability_zone");
            }
            _ => {}
        }
        let g = semantic::transform(&plan::parse(&v.to_string()).unwrap());
        assert_eq!(g.nodes.len(), 2);
        assert_eq!(g.coverage.selected.resources, 2);
        assert!(
            g.nodes
                .iter()
                .all(|n| n.role == crate::model::ResourceRole::Container
                    && n.placement == crate::model::Placement::Spatial)
        );
        let parent = g
            .nodes
            .iter()
            .position(|n| n.resource_type == "aws_ec2_secondary_network")
            .unwrap();
        let child = g
            .nodes
            .iter()
            .position(|n| n.resource_type == "aws_ec2_secondary_subnet")
            .unwrap();
        assert_eq!(
            g.edges.iter().any(|e| e.from == parent
                && e.to == child
                && e.kind == crate::model::EdgeKind::Containment),
            matches!(case, "valid" | "az-id"),
            "{case}"
        );
    }
}

#[test]
fn accounts_named_states_filtering_and_outputs_preserve_independent_identity() {
    let mut p = plan::parse(FIXTURE).unwrap();
    for n in &mut p.nodes {
        n.provider_configuration.as_mut().unwrap().account =
            crate::model::AwsAccountScope::new("123456789012");
    }
    let g = semantic::transform(&p);
    assert!(
        g.nodes
            .iter()
            .all(|n| n.entity.scope.as_ref().unwrap().account.is_some())
    );
    let selected = crate::view::single(
        &g,
        &crate::view::Options {
            focus: Some("aws_ec2_secondary_subnet.main".into()),
            depth: Some(0),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(selected.nodes.len(), 2);
    p.nodes[0].provider_configuration.as_mut().unwrap().account =
        crate::model::AwsAccountScope::new("210987654321");
    let incompatible = semantic::transform(&p);
    assert!(
        !incompatible
            .edges
            .iter()
            .any(|e| e.kind == crate::model::EdgeKind::Containment)
    );
    let plans = plan::parse_inputs(
        ["one", "two"]
            .into_iter()
            .map(|s| crate::model::PlanInput {
                state_id: crate::model::StateId::new(s).unwrap(),
                plan_json: FIXTURE.into(),
            })
            .collect(),
    )
    .unwrap();
    assert!(semantic::cross_state::resolve(&plans, &[]).edges.is_empty());
    for format in [crate::OutputFormat::Svg, crate::OutputFormat::Html] {
        let output = crate::generate(
            FIXTURE,
            &crate::Options {
                format,
                ..Default::default()
            },
        )
        .unwrap();
        assert!(output.content.contains("Secondary network"));
        assert!(!output.content.contains("10.0.0.0/16"));
    }
}
