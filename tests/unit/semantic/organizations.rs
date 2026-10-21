use crate::{plan, semantic};
use serde_json::{Value, json};

fn fixture() -> Value {
    serde_json::from_str(include_str!("../../fixtures/organizations-plan.json")).unwrap()
}

#[test]
fn organization_hierarchy_requires_current_explicit_matching_parents() {
    for case in 0..9 {
        let mut input = fixture();
        match case {
            1 => input["resource_changes"][0]["provider_name"] = json!("example.com/foreign/aws"),
            2 => input["resource_changes"][1]["change"]["after"]["parent_id"] = json!("r-other"),
            3 => input["resource_changes"][0]["change"]["after_unknown"] = json!({"roots":true}),
            4 => input["resource_changes"][1]["change"]["actions"] = json!(["delete"]),
            5 => {
                input["configuration"]["root_module"]["resources"][1]["expressions"]["parent_id"] =
                    json!({"constant_value":"r-root"})
            }
            6 => input["resource_changes"][0]["deposed"] = json!("old"),
            7 => {
                input["configuration"]["root_module"]["resources"][1]["expressions"]["parent_id"] = json!({"references":["aws_organizations_organization.main.roots[0].id", "var.choice"]})
            }
            8 => input["resource_changes"][0]["mode"] = json!("data"),
            _ => {}
        }
        let graph = semantic::transform(&plan::parse(&input.to_string()).unwrap());
        let node = graph
            .nodes
            .iter()
            .find(|node| node.address == "aws_organizations_organizational_unit.engineering")
            .unwrap();
        assert_eq!(
            node.references
                .iter()
                .any(|r| r.label == "Organization Root parent" && r.complete),
            matches!(case, 0 | 8),
            "case {case}"
        );
        assert_eq!(graph.coverage.selected.resources, 5);
        assert!(
            graph
                .nodes
                .iter()
                .all(|node| node.role != crate::model::ResourceRole::Container)
        );
        assert!(graph.nodes.iter().all(|node| {
            node.entity
                .scope
                .as_ref()
                .is_none_or(|scope| scope.account.is_none())
        }));
    }
}

#[test]
fn roots_are_derived_and_private_attributes_are_not_rendered() {
    let output = crate::generate(&fixture().to_string(), &Default::default())
        .unwrap()
        .content;
    assert!(output.contains("Derived Organization Root"));
    assert!(output.contains("Organization Root parent"));
    assert!(output.contains("5 resources"));
    assert!(!output.contains("PRIVATE_EMAIL"));
    assert!(!output.contains("PRIVATE_NAME"));
}

#[test]
fn organization_cycles_and_omitted_parents_never_claim_membership() {
    let mut input = fixture();
    input["resource_changes"][1]["change"]["after"]["parent_id"] = json!("ou-root-platform");
    input["configuration"]["root_module"]["resources"][1]["expressions"]["parent_id"] =
        json!({"references":["aws_organizations_organizational_unit.platform.id"]});
    let graph = semantic::transform(&plan::parse(&input.to_string()).unwrap());
    assert!(
        graph
            .nodes
            .iter()
            .flat_map(|node| &node.references)
            .filter(|r| matches!(r.kind, crate::model::ReferenceKind::OrganizationParent))
            .all(|r| !r.complete)
    );
    let mut input = fixture();
    input["configuration"]["root_module"]["resources"][3]["expressions"] = json!({});
    let graph = semantic::transform(&plan::parse(&input.to_string()).unwrap());
    let account = graph
        .nodes
        .iter()
        .find(|node| node.address == "aws_organizations_account.production")
        .unwrap();
    assert!(
        account
            .references
            .iter()
            .any(|r| r.label == "Organization parent" && !r.complete)
    );
}

#[test]
fn filtered_root_links_are_unavailable_without_restoring_source() {
    let parsed = plan::parse(&fixture().to_string()).unwrap();
    let graph = semantic::transform(&parsed);
    let mut filtered = graph.0.clone();
    filtered
        .nodes
        .retain(|node| node.resource_type != "aws_organizations_organization");
    filtered.components.clear();
    filtered.edges.clear();
    let node = filtered
        .nodes
        .iter()
        .find(|node| node.address == "aws_organizations_organizational_unit.engineering")
        .unwrap();
    let rows = crate::view::references::rows(node);
    assert!(
        rows.iter()
            .any(|row| row.label(false).contains("Unavailable"))
    );
}

#[test]
fn indexed_roots_keep_structural_identity() {
    let original = fixture().to_string();
    let indexed = original.replace(
        "aws_organizations_organization.main",
        "aws_organizations_organization.main[0]",
    );
    let graph = semantic::transform(&plan::parse(&indexed).unwrap());
    assert!(
        graph
            .nodes
            .iter()
            .flat_map(|node| &node.references)
            .any(
                |r| matches!(r.kind, crate::model::ReferenceKind::OrganizationRoot(0))
                    && r.complete
            )
    );
}

#[test]
fn derived_roots_retain_current_source_identity_with_deposed_organizations() {
    let mut input = fixture();
    let mut old = input["resource_changes"][0].clone();
    old["deposed"] = json!("old");
    old["change"]["actions"] = json!(["delete"]);
    old["change"]["after"] = Value::Null;
    input["resource_changes"].as_array_mut().unwrap().push(old);
    let graph = semantic::transform(&plan::parse(&input.to_string()).unwrap());
    let selected = crate::view::single(&graph, &Default::default()).unwrap();
    let svg = crate::svg::render(&selected, &crate::layout::Layout::new(&selected));
    let doc = roxmltree::Document::parse(&svg).unwrap();
    let root = doc
        .descendants()
        .find(|n| n.attribute("data-component-kind") == Some("organization_root"))
        .unwrap();
    let href = root
        .descendants()
        .find_map(|n| n.attribute("href"))
        .unwrap();
    let source = doc
        .descendants()
        .find(|n| n.attribute("id") == href.strip_prefix('#'))
        .unwrap();
    assert!(source.attribute("data-deposed-key").is_none());
    assert!(!href.contains("deposed"));
    let changed = crate::view::single(
        &graph,
        &crate::view::Options {
            changed_only: true,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(changed.components.is_empty());
}
