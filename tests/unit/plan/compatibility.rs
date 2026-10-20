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
        let mut baseline = plan::parse(&fixture("1.9.8", case)).unwrap();
        let mut candidate = plan::parse(&fixture("1.16.6", case)).unwrap();
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
        assert_eq!(first, second, "{case}");
    }
}

#[test]
fn provider_versions_preserve_semantics_for_the_common_configuration() {
    for version in ["1.9.8", "1.16.6"] {
        let old = plan::parse(&fixture(version, "aws5")).unwrap();
        let new = plan::parse(&fixture(version, "aws6")).unwrap();
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
fn real_override_contract_is_equal_but_new_core_remains_unverified() {
    let baseline = plan::parse(&fixture("1.9.8", "overrides")).unwrap();
    let candidate = plan::parse(&fixture("1.16.6", "overrides")).unwrap();
    assert_eq!(baseline.edges, candidate.edges);
    for (old, new) in baseline.nodes.iter().zip(&candidate.nodes) {
        assert_eq!(old.address, new.address);
        assert_eq!(old.provider, new.provider);
        assert_eq!(old.action, new.action);
        assert_eq!(old.has_region_override, new.has_region_override);
        assert!(new.region_override.is_none());
        if old.has_region_override {
            assert_eq!(old.region_override.as_ref().unwrap().0, "eu-west-3");
            assert!(
                crate::provider::scope(new)
                    .and_then(|scope| scope.region)
                    .is_none()
            );
        }
    }
}
