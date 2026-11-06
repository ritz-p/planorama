use crate::{plan, semantic};
use serde_json::{Value, json};

const FIXTURE: &str = include_str!("../../fixtures/prefix-list-entries-plan.json");

#[test]
fn entries_require_current_explicit_customer_managed_prefix_list_id() {
    super::super::service_references::tests::check_current_scalar_fixture(
        FIXTURE,
        "Prefix list owner",
        "prefix_list_id",
        "id",
    );
    let mut p = plan::parse(FIXTURE).unwrap();
    for node in &mut p.nodes {
        node.provider_configuration.as_mut().unwrap().account =
            crate::model::AwsAccountScope::new("123456789012");
    }
    let g = semantic::transform(&p);
    assert!(
        g.nodes
            .iter()
            .all(|n| n.entity.scope.as_ref().unwrap().account.is_some())
    );
    p.nodes[0].provider_configuration.as_mut().unwrap().account =
        crate::model::AwsAccountScope::new("210987654321");
    let g = semantic::transform(&p);
    assert!(
        !g.nodes
            .iter()
            .flat_map(|n| &n.references)
            .any(|r| r.label == "Prefix list owner" && r.complete)
    );
}

#[test]
fn owner_metadata_and_independent_coverage_do_not_establish_network_containment() {
    let p = plan::parse(FIXTURE).unwrap();
    let graph = semantic::transform(&p);
    assert_eq!(graph.coverage.selected.resources, 2);
    let entry = graph
        .nodes
        .iter()
        .find(|n| n.resource_type == "aws_ec2_managed_prefix_list_entry")
        .unwrap();
    assert!(entry.member_owner.is_some());
    assert_eq!(entry.placement, crate::model::Placement::Logical);
    assert!(
        !graph
            .edges
            .iter()
            .any(|e| e.kind == crate::model::EdgeKind::Containment)
    );
    let mut raw = semantic::Input::new(&p);
    let parent = raw
        .nodes
        .iter()
        .position(|n| n.resource_type == "aws_ec2_managed_prefix_list")
        .unwrap();
    for node in &mut raw.graph.nodes {
        node.placement = crate::model::Placement::Spatial;
    }
    let mut g = raw.graph.clone();
    super::super::service_members::present(&raw, &mut g);
    let g = crate::model::ArchitectureGraph(g);
    let entry = g
        .nodes
        .iter()
        .find(|n| n.resource_type == "aws_ec2_managed_prefix_list_entry")
        .unwrap();
    assert_eq!(crate::view::routes::owner(&g, entry), Some(parent));
    let selected = crate::view::single(
        &g,
        &crate::view::Options {
            focus: Some(entry.address.clone()),
            depth: Some(0),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(selected.nodes.len(), 1);
    assert!(crate::view::routes::owner(&selected, &selected.nodes[0]).is_none());
    let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
    v["configuration"]["root_module"]["resources"][1]["expressions"] = json!({});
    v["resource_changes"][0]["change"]["after"]["entry"] = json!([{ "cidr": "10.0.0.0/24" }]);
    let g = semantic::transform(&plan::parse(&v.to_string()).unwrap());
    assert!(g.nodes.iter().all(|n| n.member_owner.is_none()));
    for format in [crate::OutputFormat::Svg, crate::OutputFormat::Html] {
        let output = crate::generate(
            FIXTURE,
            &crate::Options {
                format,
                ..Default::default()
            },
        )
        .unwrap();
        assert!(!output.content.contains("PRIVATE_"));
        assert!(output.content.contains("Prefix list owner"));
    }
}
