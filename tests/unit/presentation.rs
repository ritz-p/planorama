use super::*;
use crate::{layout::Layout, model::PresentationOwner, plan, semantic, svg, view};

fn fixture() -> Graph {
    let graph =
        semantic::transform(&plan::parse(include_str!("../fixtures/efs-plan.json")).unwrap());
    let mut graph = (*graph).clone();
    let owner = graph
        .nodes
        .iter()
        .position(|n| n.resource_type == "aws_efs_file_system")
        .unwrap();
    let target = graph.nodes[owner].entity.provenance[0].clone();
    let child = graph
        .nodes
        .iter_mut()
        .find(|n| n.resource_type == "aws_efs_access_point")
        .unwrap();
    child.placement = Placement::Spatial;
    child.member_owner = Some(PresentationOwner {
        target,
        attribute: "file_system_id",
        label: "File system",
        section: "Access points",
    });
    graph
}

#[test]
fn effective_ownership_falls_back_without_losing_selected_resources() {
    for case in [
        "valid",
        "missing",
        "ambiguous",
        "self",
        "cycle",
        "chain",
        "scope",
        "unknown",
        "deposed",
        "deposed_owner",
        "logical_owner",
    ] {
        let mut graph = fixture();
        let child = graph
            .nodes
            .iter()
            .position(|n| n.member_owner.is_some())
            .unwrap();
        let parent = owner(&graph, &graph.nodes[child]).unwrap();
        let candidate = graph.nodes[child].member_owner.clone();
        match case {
            "missing" => {
                graph.nodes[child]
                    .member_owner
                    .as_mut()
                    .unwrap()
                    .target
                    .address = "external".into()
            }
            "ambiguous" => graph.nodes.push(graph.nodes[parent].clone()),
            "self" => {
                graph.nodes[child].member_owner.as_mut().unwrap().target =
                    graph.nodes[child].entity.provenance[0].clone()
            }
            "cycle" => {
                graph.nodes[parent].member_owner = candidate.clone();
                graph.nodes[parent].member_owner.as_mut().unwrap().target =
                    graph.nodes[child].entity.provenance[0].clone();
            }
            "chain" => graph.nodes[parent].member_owner = candidate.clone(),
            "scope" => graph.nodes[child].entity.scope.as_mut().unwrap().global = true,
            "unknown" => {
                graph.nodes[child].entity.scope = None;
                graph.nodes[parent].entity.scope = None;
            }
            "deposed" => graph.nodes[child].deposed_key = Some("old".into()),
            "deposed_owner" => graph.nodes[parent].deposed_key = Some("old".into()),
            "logical_owner" => graph.nodes[parent].placement = Placement::Logical,
            _ => {}
        }
        assert_eq!(
            owner(&graph, &graph.nodes[child]),
            (case == "valid").then_some(parent),
            "{case}"
        );
        if case != "ambiguous" {
            let layout = Layout::new(&graph);
            assert!(layout.bounds[child].width > 0, "{case}");
            let output = svg::render(&graph, &layout);
            let doc = roxmltree::Document::parse(&output).unwrap();
            assert_eq!(
                doc.descendants()
                    .filter(|n| n.attribute("data-terraform-address")
                        == Some(graph.nodes[child].address.as_str()))
                    .count(),
                1,
                "{case}"
            );
        }
    }
}

#[test]
fn common_cards_measure_new_children_and_keep_reference_provenance() {
    let mut graph = fixture();
    let child = graph
        .nodes
        .iter()
        .position(|n| n.member_owner.is_some())
        .unwrap();
    let parent = owner(&graph, &graph.nodes[child]).unwrap();
    let edges = graph.edges.clone();
    let short_height = view::routes::height(&graph, parent);
    graph.nodes[child].address = format!(
        "module.storage.aws_efs_access_point.app[\"{}<&\"]",
        "long".repeat(70)
    );
    graph.nodes[child].previous_address = Some("aws_efs_access_point.old".into());
    let layout = Layout::new(&graph);
    assert!(view::routes::height(&graph, parent) > short_height);
    let bounds = layout.bounds[child];
    let frame = layout.bounds[parent];
    assert!(bounds.origin.x > frame.origin.x && bounds.right() < frame.right());
    assert!(bounds.origin.y + bounds.height < frame.origin.y + frame.height);
    assert_eq!(graph.edges, edges);
    let output = svg::render(&graph, &layout);
    let doc = roxmltree::Document::parse(&output).unwrap();
    assert!(output.contains("Access points"));
    assert!(!output.contains("PRIVATE_PATH"));
    assert_eq!(
        doc.descendants()
            .filter(|n| n.attribute("data-terraform-address").is_some())
            .count(),
        graph.nodes.len()
    );
    let metadata = doc
        .descendants()
        .find(|n| n.attribute("id") == Some("member-presentations"))
        .unwrap();
    let value: serde_json::Value = serde_json::from_str(metadata.text().unwrap()).unwrap();
    assert_eq!(value[0]["attribute"], "file_system_id");
    assert_eq!(value[0]["kind"], "presentation");
}

#[test]
fn semantic_membership_does_not_imply_presentation_ownership() {
    let graph =
        semantic::transform(&plan::parse(include_str!("../fixtures/aurora-plan.json")).unwrap());
    let mut graph = (*graph).clone();
    let node = graph
        .nodes
        .iter_mut()
        .find(|node| node.member_owner.is_some())
        .unwrap();
    assert!(
        node.references
            .iter()
            .any(|reference| reference.kind == crate::model::ReferenceKind::Membership)
    );
    node.member_owner = None;
    let identity = node.entity.id.clone();
    let node = graph
        .nodes
        .iter()
        .find(|node| node.entity.id == identity)
        .unwrap();
    assert_eq!(effective(&graph, node), Effective::Independent);
}

#[test]
fn selected_children_and_state_local_export_context_share_effective_owner() {
    let graph = crate::model::ArchitectureGraph(fixture());
    for address in ["aws_efs_access_point.app", "aws_efs_file_system.shared"] {
        let selected = view::single(
            &graph,
            &view::Options {
                focus: Some(address.into()),
                depth: Some(0),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(selected.nodes.len(), 1);
        assert_eq!(
            effective(&selected, &selected.nodes[0]),
            Effective::Independent
        );
    }
    let architecture = crate::model::architecture::Architecture {
        states: [
            (crate::model::StateId::new("one").unwrap(), graph.clone()),
            (crate::model::StateId::new("two").unwrap(), graph),
        ]
        .into(),
        relationships: Vec::new(),
    };
    let selected = view::apply(&architecture, &Default::default()).unwrap();
    let html = crate::html::render_states(&selected, svg::Options::default());
    let start = html.find("id=\"viewer-data\"").unwrap();
    let text = &html[start..];
    let data = &text[text.find('>').unwrap() + 1..text.find("</script>").unwrap()];
    let resources: Vec<serde_json::Value> = serde_json::from_str(data).unwrap();
    for state in ["one", "two"] {
        let child = resources
            .iter()
            .find(|r| r["state"] == state && r["address"] == "aws_efs_access_point.app")
            .unwrap();
        let parent = resources
            .iter()
            .find(|r| r["state"] == state && r["address"] == "aws_efs_file_system.shared")
            .unwrap();
        assert_eq!(child["context"], serde_json::json!([parent["id"]]));
    }
}
