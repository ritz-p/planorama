use crate::{plan, semantic, view};
use serde_json::json;

#[test]
fn unassociated_output_and_drift_count_only_toward_input_categories() {
    let raw = plan::parse(&json!({
        "format_version": "1.2",
        "resource_changes": [{"address": "aws_instance.main", "type": "aws_instance"}],
        "resource_drift": [{"address": "aws_instance.main", "type": "aws_vpc", "change": {"actions": ["delete"]}}],
        "configuration": {"root_module": {"outputs": {"missing": {"expression": {"references": ["aws_instance.missing.id"]}}}}}
    }).to_string()).unwrap();
    let graph = semantic::transform(&raw);
    assert_eq!(graph.coverage.unassociated_findings, 2);
    for category in ["unresolved", "drift"] {
        assert_eq!(graph.coverage.input_findings.get(category), Some(&1));
        assert!(!graph.coverage.selected_findings.contains_key(category));
    }
    assert!(
        !graph
            .coverage
            .findings
            .iter()
            .any(|finding| finding.code == "drift_unmatched"
                || finding.source.address.starts_with("output."))
    );
}

#[test]
fn counts_source_instances_across_lowering_routes_and_filtering() {
    for input in [
        include_str!("../fixtures/association-plan.json"),
        include_str!("../fixtures/routes-plan.json"),
        include_str!("../fixtures/containment-plan.json"),
        include_str!("../fixtures/components-plan.json"),
    ] {
        let raw = plan::parse(input).unwrap();
        let graph = semantic::transform(&raw);
        assert_eq!(graph.coverage.input.resources, raw.nodes.len());
        assert_eq!(graph.coverage.selected.resources, raw.nodes.len());
        let view = view::single(
            &graph,
            &view::Options {
                changed_only: true,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(view.coverage.input, graph.coverage.input);
        assert_eq!(
            view.coverage.selected.resources,
            crate::coverage::represented(&view).len()
        );
        assert!(
            view.coverage
                .findings
                .iter()
                .all(|finding| view.coverage.resources.contains_key(&finding.source))
        );
    }
}

#[test]
fn scope_denominators_and_current_deposed_data_identity_are_distinct() {
    let raw = plan::parse(
        &json!({"format_version":"1.2", "resource_changes":[
            {"address":"aws_instance.a", "type":"aws_instance"},
            {"address":"aws_instance.a", "type":"aws_instance", "deposed":"old"},
            {"address":"aws_iam_role.global", "type":"aws_iam_role"},
            {"address":"terraform_data.local", "type":"terraform_data"},
            {"address":"data.aws_region.hidden", "type":"aws_region", "mode":"data"}
        ]})
        .to_string(),
    )
    .unwrap();
    let graph = semantic::transform(&raw);
    let coverage = &graph.coverage;
    assert_eq!(coverage.input.resources, 5);
    assert_eq!(coverage.selected.resources, 4);
    assert_eq!(coverage.input.account_applicable, 3);
    assert_eq!(coverage.input.unknown_account, 3);
    assert_eq!(coverage.input.region_applicable, 2);
    assert_eq!(coverage.input.unknown_region, 2);
    let instances: Vec<_> = coverage
        .resources
        .keys()
        .filter(|id| id.address == "aws_instance.a")
        .collect();
    assert_eq!(instances.len(), 2);
    assert!(
        instances
            .iter()
            .any(|id| id.deposed_key.as_deref() == Some("old"))
    );
    assert_eq!(
        coverage
            .findings
            .iter()
            .filter(|f| f.code == "missing_attribute" && f.source.address == "aws_instance.a")
            .count(),
        1
    );
}

#[test]
fn named_states_keep_equal_addresses_separate_and_preserve_empty_states() {
    let raw = plan::parse(r#"{"format_version":"1.2","resource_changes":[{"address":"terraform_data.x","type":"terraform_data"}]}"#).unwrap();
    let architecture = crate::model::cross_state::Architecture {
        states: ["alpha", "beta"]
            .into_iter()
            .map(|id| {
                (
                    crate::model::StateId::new(id).unwrap(),
                    semantic::transform(&raw),
                )
            })
            .collect(),
        relationships: Vec::new(),
    };
    let selected = view::apply(
        &architecture,
        &view::Options {
            focus: Some("alpha:terraform_data.x".into()),
            depth: Some(0),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(
        selected
            .states
            .values()
            .map(|graph| graph.coverage.input.resources)
            .sum::<usize>(),
        2
    );
    assert_eq!(
        selected
            .states
            .values()
            .map(|graph| graph.coverage.selected.resources)
            .sum::<usize>(),
        1
    );
    let beta = &selected.states[&crate::model::StateId::new("beta").unwrap()].coverage;
    assert!(beta.collected);
    assert!(beta.resources.is_empty());
    assert!(beta.findings.is_empty());
    assert!(beta.selected_findings.is_empty());
    assert!(!crate::coverage::Coverage::default().collected);
}

#[test]
fn incomplete_input_and_deterministic_findings_do_not_claim_complete_analysis() {
    let mut input = json!({"format_version":"1.2", "complete": false, "resource_changes":[
        {"address":"aws_instance.z", "type":"aws_instance"},
        {"address":"aws_instance.a", "type":"aws_instance"}
    ]});
    let first = semantic::transform(&plan::parse(&input.to_string()).unwrap());
    input["resource_changes"].as_array_mut().unwrap().reverse();
    let second = semantic::transform(&plan::parse(&input.to_string()).unwrap());
    assert_eq!(first.coverage, second.coverage);
    assert_eq!(first.coverage.input_complete, Some(false));
    assert_eq!(
        crate::model::DiagnosticReason::DynamicInstanceSelection.category(),
        "dynamic"
    );
    assert_eq!(
        crate::model::DiagnosticReason::MultipleMatchingInstances.category(),
        "ambiguous"
    );
    assert_eq!(
        crate::model::DiagnosticReason::UnresolvedReference.code(),
        "unresolved_reference"
    );
}
