use crate::{
    model::{EdgeKind, Placement, ReferenceKind},
    plan, semantic,
};
use serde_json::{Value, json};

const FIXTURE: &str = include_str!("../../fixtures/eks-plan.json");

#[test]
fn eks_configurations_validate_cluster_references_independently() {
    let original: Value = serde_json::from_str(FIXTURE).unwrap();
    for (index, label) in [
        (1, "Cluster membership"),
        (2, "Cluster membership"),
        (3, "Access configuration cluster"),
        (4, "Cluster membership"),
    ] {
        let mut input = original.clone();
        input["resource_changes"]
            .as_array_mut()
            .unwrap()
            .swap(1, index);
        input["configuration"]["root_module"]["resources"]
            .as_array_mut()
            .unwrap()
            .swap(1, index);
        super::service_references::tests::check_service_fixture(
            &input.to_string(),
            label,
            "cluster_name",
        );
    }
    let graph = semantic::transform(&plan::parse(FIXTURE).unwrap());
    assert_eq!(graph.coverage.selected.resources, 10);
    assert!(graph.edges.iter().any(|e| e.kind == EdgeKind::Containment
        && graph.nodes[e.from].resource_type == "aws_vpc"
        && graph.nodes[e.to].resource_type == "aws_eks_cluster"));
    assert!(graph.edges.iter().any(|e| e.kind == EdgeKind::Connection
        && graph.nodes[e.from].resource_type == "aws_security_group"
        && graph.nodes[e.to].resource_type == "aws_eks_cluster"));
    assert!(!graph.edges.iter().any(|e| e.kind == EdgeKind::Connection
        && graph.nodes[e.to].resource_type == "aws_eks_fargate_profile"));
    for node in &graph.nodes {
        if matches!(
            node.resource_type.as_str(),
            "aws_eks_addon" | "aws_eks_access_entry"
        ) {
            assert_eq!(node.placement, Placement::Logical);
        }
        for reference in &node.references {
            if reference.label == "Execution role reference"
                || reference.label == "Configured principal"
            {
                assert!(reference.complete);
                assert_eq!(reference.kind, ReferenceKind::Configured);
            }
        }
    }
    assert!(!graph.edges.iter().any(|e| e.kind == EdgeKind::Containment
        && graph.nodes[e.from].resource_type == "aws_eks_cluster"));
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
        assert!(!output.contains("arn:aws:iam::123456789012:role/fixture"));
        assert!(output.contains("Cluster membership"));
        assert!(output.contains("Access configuration cluster"));
    }
}

#[test]
fn eks_network_placement_requires_current_complete_known_subnets() {
    for target in [2, 4] {
        for case in 0..13 {
            let mut input: Value = serde_json::from_str(FIXTURE).unwrap();
            match case {
                1 => {
                    input["resource_changes"][6]["provider_name"] = json!("example.com/foreign/aws")
                }
                2 => input["resource_changes"][6]["change"]["after_unknown"] = json!({"id":true}),
                3 => {
                    input["resource_changes"][target]["change"]["after"]["subnet_ids"] =
                        json!(["subnet-a", "literal"])
                }
                4 => input["resource_changes"][target]["change"]["actions"] = json!(["delete"]),
                5 => input["resource_changes"][6]["deposed"] = json!("old"),
                6 => input["resource_changes"][6]["mode"] = json!("data"),
                7 => {
                    input["configuration"]["root_module"]["resources"][target]["expressions"]["subnet_ids"] =
                        json!({"references":["aws_subnet.a.id","var.selector"]})
                }
                8 => {
                    input["configuration"]["root_module"]["resources"][6]["expressions"]["region"] =
                        json!({"constant_value":"us-west-2"})
                }
                9 => input["resource_changes"][6]["type"] = json!("aws_instance"),
                10 => {
                    input["resource_changes"][target]["change"]["after_unknown"]["subnet_ids"] =
                        json!([false, true])
                }
                11 => input["resource_changes"][5]["change"]["actions"] = json!(["delete"]),
                12 => {
                    input["resource_changes"][6]["change"]["after"]["vpc_id"] = json!("other-vpc")
                }
                _ => {}
            }
            let graph = semantic::transform(&plan::parse(&input.to_string()).unwrap());
            let address = input["resource_changes"][target]["address"]
                .as_str()
                .unwrap();
            assert_eq!(
                graph.edges.iter().any(
                    |e| e.kind == EdgeKind::Containment && graph.nodes[e.to].address == address
                ),
                case == 0,
                "{target}/{case}"
            );
            assert_eq!(graph.coverage.selected.resources, 10);
        }
    }
}
