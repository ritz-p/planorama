use crate::{
    model::{Action, ReferenceKind},
    plan, semantic, view,
};
use serde_json::{Value, json};
const FIXTURE: &str = include_str!("../../fixtures/rds-global-plan.json");

#[test]
fn global_membership_requires_current_unique_explicit_aws_evidence() {
    for case in 0..12 {
        let mut input: Value = serde_json::from_str(FIXTURE).unwrap();
        match case {
            1 => input["resource_changes"][0]["provider_name"] = json!("example.com/foreign/aws"),
            2 => input["resource_changes"][0]["type"] = json!("aws_rds_cluster"),
            3 => input["resource_changes"][0]["change"]["after_unknown"] = json!({"id":true}),
            4 => {
                input["resource_changes"][1]["change"]["after"]["global_cluster_identifier"] =
                    json!("different")
            }
            5 => {
                input["configuration"]["root_module"]["resources"][1]["expressions"]["global_cluster_identifier"] =
                    json!({"constant_value":"fixture-world"})
            }
            6 => {
                input["configuration"]["root_module"]["resources"][1]["expressions"]["global_cluster_identifier"] =
                    json!({"references":["aws_rds_global_cluster.world[0].id", "var.dynamic"]})
            }
            7 => input["resource_changes"][0]["mode"] = json!("data"),
            8 => input["resource_changes"][0]["deposed"] = json!("old"),
            9 => input["resource_changes"][1]["change"]["actions"] = json!(["delete"]),
            10 => {
                let mut extra = input["resource_changes"][0].clone();
                extra["address"] = json!("aws_rds_global_cluster.world[1]");
                input["resource_changes"]
                    .as_array_mut()
                    .unwrap()
                    .push(extra);
                input["configuration"]["root_module"]["resources"][1]["expressions"]["global_cluster_identifier"] =
                    json!({"references":["aws_rds_global_cluster.world.id"]});
            }
            11 => input["resource_changes"][0]["change"]["actions"] = json!(["delete"]),
            _ => {}
        }
        let graph = semantic::transform(&plan::parse(&input.to_string()).unwrap());
        let regional = graph
            .nodes
            .iter()
            .find(|n| n.address == "aws_rds_cluster.east")
            .unwrap();
        assert_eq!(
            regional
                .references
                .iter()
                .any(|r| r.kind == ReferenceKind::Membership
                    && r.label == "Global Cluster membership"
                    && r.complete),
            case == 0,
            "case {case}"
        );
        assert_eq!(
            graph.coverage.selected.resources,
            if case == 10 { 7 } else { 6 }
        );
    }
}

#[test]
fn global_scope_keeps_regional_clusters_and_embedded_instances() {
    let graph = semantic::transform(&plan::parse(FIXTURE).unwrap());
    let global = graph
        .nodes
        .iter()
        .find(|n| n.resource_type == "aws_rds_global_cluster")
        .unwrap();
    assert!(global.entity.scope.as_ref().unwrap().global);
    assert!(global.entity.scope.as_ref().unwrap().region.is_none());
    assert_eq!(
        graph
            .nodes
            .iter()
            .filter(|n| n.member_owner.is_some())
            .count(),
        2
    );
    for (address, region) in [
        ("aws_rds_cluster.east", "us-east-1"),
        ("aws_rds_cluster.west", "eu-west-1"),
    ] {
        let node = graph.nodes.iter().find(|n| n.address == address).unwrap();
        assert_eq!(
            node.entity
                .scope
                .as_ref()
                .unwrap()
                .region
                .as_ref()
                .unwrap()
                .0,
            region
        );
    }
    let svg = crate::generate(FIXTURE, &Default::default())
        .unwrap()
        .content;
    assert!(svg.contains("Global Cluster membership"));
    assert!(!svg.contains("PRIVATE_"));
    assert!(!svg.contains("is_writer"));
    let filtered = view::single(
        &graph,
        &view::Options {
            focus: Some("aws_rds_cluster.east".into()),
            depth: Some(0),
            ..Default::default()
        },
    )
    .unwrap();
    assert!(
        !filtered
            .nodes
            .iter()
            .any(|n| n.resource_type == "aws_rds_global_cluster")
    );
    let svg = crate::svg::render(&filtered, &crate::layout::Layout::new(&filtered));
    assert!(svg.contains("Unavailable"));
    assert!(!svg.contains("data-reference-attribute=\"global_cluster_identifier\""));
    assert!(global.action == Action::Unchanged);
}

#[test]
fn contradictory_verified_accounts_do_not_claim_global_membership() {
    let mut input: Value = serde_json::from_str(FIXTURE).unwrap();
    for (provider, account) in [("aws.east", "111111111111"), ("aws.west", "222222222222")] {
        input["configuration"]["provider_config"][provider]["expressions"]["assume_role"] =
            json!([{"role_arn":{"constant_value":format!("arn:aws:iam::{account}:role/Fixture")}}]);
    }
    let graph = semantic::transform(
        &crate::plan::evidence::tests::parse_fixture(&input.to_string()).unwrap(),
    );
    let west = graph
        .nodes
        .iter()
        .find(|n| n.address == "aws_rds_cluster.west")
        .unwrap();
    assert!(
        !west
            .references
            .iter()
            .any(|r| r.label == "Global Cluster membership" && r.complete)
    );
}
