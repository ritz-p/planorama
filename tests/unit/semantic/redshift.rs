use crate::{model::EdgeKind, plan, semantic};
use serde_json::{Value, json};
const FIXTURE: &str = include_str!("../../fixtures/redshift-plan.json");

#[test]
fn redshift_preserves_multisubnet_vpc_placement_and_private_configuration() {
    let graph = semantic::transform(&plan::parse(FIXTURE).unwrap());
    assert_eq!(graph.coverage.selected.resources, 7);
    for kind in ["aws_redshift_cluster", "aws_redshift_subnet_group"] {
        let target = graph
            .nodes
            .iter()
            .position(|n| n.resource_type == kind)
            .unwrap();
        let parent = graph
            .edges
            .iter()
            .find(|e| e.to == target && e.kind == EdgeKind::Containment)
            .unwrap();
        assert_eq!(graph.nodes[parent.from].resource_type, "aws_vpc");
    }
    assert!(graph.edges.iter().any(|e| e.kind == EdgeKind::Connection
        && graph.nodes[e.from].resource_type == "aws_security_group"));
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
        assert_eq!(
            output.contains("planorama-icon-aws-redshift"),
            cfg!(feature = "icon-artwork")
        );
        assert!(!output.contains("PRIVATE_"));
    }
}

#[test]
fn redshift_placement_requires_complete_current_network_proof() {
    for case in [
        "unknown",
        "mismatch",
        "subnet-mismatch",
        "ambiguous",
        "wrong-provider",
        "wrong-type",
        "delete",
        "deposed",
        "data",
        "scope",
    ] {
        let mut value: Value = serde_json::from_str(FIXTURE).unwrap();
        match case {
            "unknown" => {
                value["resource_changes"][6]["change"]["after_unknown"] =
                    json!({"cluster_subnet_group_name":true})
            }
            "mismatch" => {
                value["resource_changes"][6]["change"]["after"]["cluster_subnet_group_name"] =
                    json!("other")
            }
            "subnet-mismatch" => {
                value["resource_changes"][3]["change"]["after"]["subnet_ids"] =
                    json!(["subnet-a", "other"])
            }
            "ambiguous" => {
                value["configuration"]["root_module"]["resources"][6]["expressions"]["cluster_subnet_group_name"] = json!({"references":["aws_redshift_subnet_group.main.name","aws_redshift_parameter_group.main.name"]})
            }
            "wrong-provider" => {
                value["resource_changes"][3]["provider_name"] = json!("example.com/foreign/aws")
            }
            "wrong-type" => value["resource_changes"][3]["type"] = json!("aws_db_subnet_group"),
            "delete" => value["resource_changes"][3]["change"]["actions"] = json!(["delete"]),
            "deposed" => value["resource_changes"][3]["deposed"] = json!("old"),
            "data" => value["resource_changes"][3]["mode"] = json!("data"),
            "scope" => {
                value["configuration"]["root_module"]["resources"][6]["expressions"]["region"] =
                    json!({"references":["var.region"]})
            }
            _ => unreachable!(),
        }
        let parsed = plan::parse(&value.to_string()).unwrap();
        let graph = semantic::transform(&parsed);
        let target = graph
            .nodes
            .iter()
            .position(|n| n.resource_type == "aws_redshift_cluster")
            .unwrap();
        assert!(
            !graph
                .edges
                .iter()
                .any(|e| e.to == target && e.kind == EdgeKind::Containment),
            "{case}"
        );
    }
}

#[test]
fn redshift_filtered_indexed_and_named_states_preserve_identity() {
    let indexed = FIXTURE.replace("aws_subnet.a", "aws_subnet.a[0]");
    let graph = semantic::transform(&plan::parse(&indexed).unwrap());
    let selected = crate::view::single(
        &graph,
        &crate::view::Options {
            focus: Some("aws_redshift_cluster.main".into()),
            depth: Some(0),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(selected.coverage.selected.resources, 1);
    assert!(
        crate::svg::render(&selected, &crate::layout::Layout::new(&selected))
            .contains("Unavailable")
    );
    let plans = plan::parse_inputs(
        ["one", "two"]
            .into_iter()
            .map(|state| crate::model::PlanInput {
                state_id: crate::model::StateId::new(state).unwrap(),
                plan_json: FIXTURE.into(),
            })
            .collect(),
    )
    .unwrap();
    assert!(semantic::cross_state::resolve(&plans, &[]).edges.is_empty());
}
