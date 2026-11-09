use crate::{coverage::Coverage, plan, semantic};

fn fixture(version: &str, case: &str) -> String {
    std::fs::read_to_string(format!(
        "{}/tests/fixtures/compatibility/{version}/{case}/plan.json",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap()
}

#[test]
fn real_core_versions_preserve_identity_actions_references_and_coverage() {
    for case in ["aws5", "aws6", "language"] {
        let matrix: serde_json::Value =
            serde_json::from_str(include_str!("../../fixtures/compatibility/matrix.json")).unwrap();
        for version in matrix["terraform"]
            .as_object()
            .unwrap()
            .keys()
            .filter(|v| case != "aws6" || ["1.9.8", "1.16.6"].contains(&v.as_str()))
        {
            let mut baseline = plan::parse(&fixture("1.9.8", case)).unwrap();
            let mut candidate = plan::parse(&fixture(version, case)).unwrap();
            for raw in [&mut baseline, &mut candidate] {
                for node in &mut raw.nodes {
                    node.metadata.changed_paths = Default::default();
                }
            }
            if case == "language" {
                for raw in [&mut baseline, &mut candidate] {
                    for node in &mut raw.nodes {
                        if node.resource_type == "terraform_remote_state" {
                            node.metadata.relevant_attributes = None;
                        }
                    }
                }
            }
            assert_eq!(baseline.nodes, candidate.nodes, "{case}");
            assert_eq!(baseline.edges, candidate.edges, "{case}");
            assert_eq!(baseline.attributes, candidate.attributes, "{case}");
            assert_eq!(
                baseline.graph_references, candidate.graph_references,
                "{case}"
            );
            assert_eq!(baseline.checks, candidate.checks, "{case}");
            assert_eq!(baseline.status, candidate.status, "{case}");
            let baseline_graph = semantic::transform(&baseline);
            let candidate_graph = semantic::transform(&candidate);
            let first = Coverage::collect(&baseline, &baseline_graph);
            let second = Coverage::collect(&candidate, &candidate_graph);
            assert_eq!(first, second, "{version}/{case}");
        }
    }
}

#[test]
fn provider_versions_preserve_semantics_for_the_common_configuration() {
    for (version, case) in [("1.9.8", "aws6"), ("1.16.6", "aws6"), ("1.9.8", "aws670")] {
        let mut old = plan::parse(&fixture(version, "aws5")).unwrap();
        let mut new = plan::parse(&fixture(version, case)).unwrap();
        for raw in [&mut old, &mut new] {
            for node in &mut raw.nodes {
                node.metadata.changed_paths = Default::default();
            }
        }
        assert_eq!(old.nodes.len(), 10);
        assert_eq!(old.nodes, new.nodes);
        assert_eq!(old.edges, new.edges);
        assert_eq!(old.attributes, new.attributes);
        assert_eq!(
            Coverage::collect(&old, &semantic::transform(&old)),
            Coverage::collect(&new, &semantic::transform(&new))
        );
    }
}

#[test]
fn real_override_contract_preserves_scopes_and_coverage_across_verified_cores() {
    let baseline = plan::parse(&fixture("1.9.8", "overrides")).unwrap();
    let candidate = plan::parse(&fixture("1.16.6", "overrides")).unwrap();
    assert_eq!(baseline.nodes, candidate.nodes);
    assert_eq!(baseline.edges, candidate.edges);
    assert_eq!(baseline.attributes, candidate.attributes);
    let before = Coverage::collect(&baseline, &semantic::transform(&baseline));
    let after = Coverage::collect(&candidate, &semantic::transform(&candidate));
    assert_eq!(before, after);
    assert_eq!(after.input.unknown_region, 0);
    assert_eq!(after.input.unknown_account, 4);
}
