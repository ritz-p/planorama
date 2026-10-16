use super::*;
use crate::{layout::Layout, plan, svg};

pub(crate) fn base_graph(plan: &TerraformPlan) -> crate::model::Graph {
    let mut graph = super::input::base_graph(plan);
    graph.coverage = crate::coverage::Coverage::collect(plan, &graph);
    graph
}

#[test]
fn messaging_resources_keep_scopes_data_actions_and_explicit_dependencies() {
    use crate::model::{Action, EdgeKind, EntityMode, Placement, ResourceRole};
    let raw = plan::parse(include_str!("../fixtures/messaging-plan.json")).unwrap();
    let graph = transform(&raw);
    assert_eq!(graph.nodes.len(), 3);
    for node in &graph.nodes {
        assert_eq!(node.role, ResourceRole::Node);
        assert_eq!(node.placement, Placement::Spatial);
        assert!(node.entity.scope.as_ref().unwrap().region.is_some());
        assert_eq!(node.entity.provenance.len(), 1);
    }
    assert!(graph.nodes.iter().any(|n| n.mode == EntityMode::Data));
    assert!(graph.nodes.iter().any(|n| n.action == Action::Update));
    assert_eq!(graph.edges.len(), 1);
    assert_eq!(graph.edges[0].kind, EdgeKind::Dependency);
    let output = svg::render(&graph, &Layout::new(&graph));
    assert!(output.contains("planorama-icon-aws-sqs"));
    assert!(output.contains("planorama-icon-aws-sns"));
    assert!(output.contains("us-east-1"));
    assert_eq!(graph, transform(&raw));
}

#[test]
fn messaging_singletons_keep_verified_account_and_region_panels() {
    let mut input: serde_json::Value =
        serde_json::from_str(include_str!("../fixtures/messaging-plan.json")).unwrap();
    input["configuration"]["provider_config"]["aws"]["expressions"]["assume_role"] = serde_json::json!([{"role_arn":{"constant_value":"arn:aws:iam::123456789012:role/example"}}]);
    input["configuration"]["provider_config"]["aws.archive"]["expressions"]["assume_role"] = serde_json::json!([{"role_arn":{"constant_value":"arn:aws:iam::210987654321:role/example"}}]);
    let raw = crate::plan::evidence::tests::parse_fixture(&input.to_string()).unwrap();
    let graph = transform(&raw);
    assert!(
        graph
            .nodes
            .iter()
            .all(|n| n.entity.scope.as_ref().unwrap().account.is_some())
    );
    let output = svg::render(&graph, &Layout::new(&graph));
    assert_eq!(output.matches("data-account-panel=").count(), 2);
    assert!(output.contains("210987654321"));
    assert!(output.contains("us-east-1"));
}

#[test]
fn containment_and_associations_survive_node_remapping_together() {
    use crate::model::EdgeKind;
    use serde_json::json;
    let mut input: serde_json::Value =
        serde_json::from_str(include_str!("../fixtures/association-plan.json")).unwrap();
    input["resource_changes"]
        .as_array_mut()
        .unwrap()
        .push(json!({"address":"aws_vpc.main", "type":"aws_vpc", "change":{"actions":["create"]}}));
    let resources = input["configuration"]["root_module"]["resources"]
        .as_array_mut()
        .unwrap();
    resources.push(json!({"address":"aws_vpc.main"}));
    let subnet = resources
        .iter_mut()
        .find(|resource| resource["address"] == "aws_subnet.private")
        .unwrap();
    subnet["expressions"]["vpc_id"] = json!({"references":["aws_vpc.main.id"]});
    let raw = plan::parse(&input.to_string()).unwrap();
    let original = raw.clone();
    let architecture = transform(&raw);
    assert_eq!(raw, original);
    assert_eq!(architecture.nodes.len(), 3);
    assert_eq!(architecture.edges.len(), 2);
    for edge in &architecture.edges {
        let from = architecture.nodes[edge.from].address.as_str();
        let to = architecture.nodes[edge.to].address.as_str();
        match edge.kind {
            EdgeKind::Containment => assert_eq!((from, to), ("aws_vpc.main", "aws_subnet.private")),
            EdgeKind::Association => {
                assert_eq!(
                    (from, to),
                    ("aws_subnet.private", "aws_route_table.private")
                );
                assert_eq!(
                    edge.change.as_ref().unwrap().address,
                    "aws_route_table_association.private"
                );
            }
            _ => panic!("unexpected relationship"),
        }
    }
    assert_eq!(architecture, transform(&raw));
}

#[test]
fn unmatched_graphs_preserve_structure_order_and_svg() {
    for input in [
        include_str!("../fixtures/terraform-plan.json"),
        include_str!("../../examples/bundling-plan.json"),
        r#"{"format_version":"1.0","resource_changes":[]}"#,
    ] {
        let raw = plan::parse(input).unwrap();
        let architecture = transform(&raw);
        assert_eq!(architecture.0, crate::semantic::base_graph(&raw));
        assert_eq!(architecture, transform(&raw));
        assert_eq!(
            svg::render(&base_graph(&raw), &Layout::new(&base_graph(&raw))),
            svg::render(&architecture, &Layout::new(&architecture))
        );
    }
}

#[test]
fn architecture_changes_do_not_mutate_raw_terraform_graph() {
    let raw = plan::parse(include_str!("../../examples/plan.json")).unwrap();
    let original = raw.clone();
    let mut architecture = transform(&raw);
    architecture.0.nodes[0].address = "changed".into();
    architecture.0.edges.clear();
    assert_eq!(raw, original);
    assert_ne!(architecture.0, crate::semantic::base_graph(&raw));
}
