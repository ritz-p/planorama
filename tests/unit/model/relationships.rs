use crate::model::*;

#[test]
fn containment_provenance_excludes_other_dependency_paths() {
    let mut input: serde_json::Value =
        serde_json::from_str(include_str!("../../fixtures/rds-subnet-group-plan.json")).unwrap();
    input["resource_changes"].as_array_mut().unwrap().push(
        serde_json::json!({"address":"aws_security_group.other","type":"aws_security_group"}),
    );
    let resources = input["configuration"]["root_module"]["resources"]
        .as_array_mut()
        .unwrap();
    resources.push(serde_json::json!({"address":"aws_security_group.other","expressions":{"vpc_id":{"references":["aws_vpc.main.id"]}}}));
    resources
        .iter_mut()
        .find(|r| r["address"] == "aws_db_instance.main")
        .unwrap()["expressions"]["vpc_security_group_ids"] =
        serde_json::json!({"references":["aws_security_group.other.id"]});
    let graph = crate::semantic::transform(&crate::plan::parse(&input.to_string()).unwrap());
    let target = &graph
        .nodes
        .iter()
        .find(|n| n.address == "aws_db_instance.main")
        .unwrap()
        .entity
        .id;
    let relationship = graph
        .relationships
        .iter()
        .find(|r| &r.to == target && r.kind == EdgeKind::Containment)
        .unwrap();
    assert_eq!(relationship.provenance.len(), 5);
    for evidence in &relationship.provenance {
        let RelationshipProvenance::Reference { from, to } = evidence else {
            panic!("expected reference evidence")
        };
        assert!(!from.address.contains("security_group") && !to.address.contains("security_group"));
    }
}

#[test]
fn rendered_relationship_provenance_preserves_redacted_change_details() {
    let mut input: serde_json::Value =
        serde_json::from_str(include_str!("../../fixtures/association-plan.json")).unwrap();
    let helper = input["resource_changes"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|r| r["type"] == "aws_route_table_association")
        .unwrap();
    helper["action_reason"] = serde_json::json!("replace_because_cannot_update");
    helper["change"] = serde_json::json!({"actions":["forget"],"importing":{"id":"PRIVATE_IMPORT"},"replace_paths":[["subnet_id"]]});
    let mut graph = crate::semantic::transform(&crate::plan::parse(&input.to_string()).unwrap());
    let RelationshipProvenance::Resource { change, .. } =
        &mut graph.0.relationships[0].provenance[0]
    else {
        panic!("expected lowered helper")
    };
    change.metadata.drift.insert(Action::Update);
    change.metadata.relevant_attributes =
        Some(vec![vec![AttributePathStep::Attribute("subnet_id".into())]]);
    let svg = crate::svg::render(&graph, &crate::layout::Layout::new(&graph));
    let document = roxmltree::Document::parse(&svg).unwrap();
    let metadata = document
        .descendants()
        .find(|n| n.attribute("id") == Some("architecture-relationships"))
        .unwrap()
        .text()
        .unwrap();
    let value: serde_json::Value = serde_json::from_str(metadata).unwrap();
    let resource = &value[0]["provenance"][0];
    assert_eq!(resource["operation"], "import / forget");
    assert_eq!(resource["metadata"]["import"]["has_id"], true);
    assert_eq!(resource["metadata"]["state_removal"], "forget");
    assert_eq!(resource["metadata"]["drift"], serde_json::json!(["update"]));
    assert_eq!(
        resource["metadata"]["replace_paths"],
        serde_json::json!([["subnet_id"]])
    );
    assert_eq!(
        resource["metadata"]["relevant_attributes"],
        serde_json::json!([["subnet_id"]])
    );
    assert_eq!(
        resource["metadata"]["action_reason"],
        "replace_because_cannot_update"
    );
    assert!(!svg.contains("PRIVATE_IMPORT"));
}

#[test]
fn native_relationships_support_synthetic_endpoints_and_multiple_sources() {
    let source = |address: &str| TerraformEntityId {
        address: address.into(),
        deposed_key: None,
    };
    let a = RelationshipProvenance::Reference {
        from: source("test.a"),
        to: source("test.b"),
    };
    let b = RelationshipProvenance::Reference {
        from: source("test.c"),
        to: source("test.b"),
    };
    let from = ArchitectureId::synthetic("component", "a");
    let to = ArchitectureId::synthetic("component", "b");
    let first = ArchitectureRelationship::new(
        from.clone(),
        to.clone(),
        EdgeKind::Connection,
        true,
        vec![b.clone(), a.clone(), a.clone()],
    );
    let second = ArchitectureRelationship::new(from, to, EdgeKind::Connection, true, vec![a, b]);
    assert_eq!(first, second);
    assert_eq!(first.provenance.len(), 2);
}

#[test]
fn lowered_and_inferred_relationships_retain_distinct_evidence() {
    for fixture in [
        include_str!("../../fixtures/association-plan.json"),
        include_str!("../../fixtures/security-groups-plan.json"),
        include_str!("../../fixtures/containment-plan.json"),
    ] {
        let plan = crate::plan::parse(fixture).unwrap();
        let graph = crate::semantic::transform(&plan);
        let ids: std::collections::BTreeSet<_> = graph.entities().map(|e| &e.id).collect();
        for relationship in &graph.relationships {
            assert!(ids.contains(&relationship.from) && ids.contains(&relationship.to));
            assert!(!relationship.provenance.is_empty());
            if relationship.kind == EdgeKind::Association {
                assert!(!relationship.inferred);
                assert!(matches!(
                    relationship.provenance[0],
                    RelationshipProvenance::Resource { .. }
                ));
            } else if relationship.kind != EdgeKind::Dependency {
                assert!(relationship.inferred);
                assert!(
                    relationship
                        .provenance
                        .iter()
                        .all(|p| matches!(p, RelationshipProvenance::Reference { .. }))
                );
            }
        }
        let svg = crate::svg::render(&graph, &crate::layout::Layout::new(&graph));
        assert!(svg.contains("architecture-relationships"));
    }
}
