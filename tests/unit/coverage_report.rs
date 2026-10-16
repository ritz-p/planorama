use super::*;
use crate::{plan, semantic, view};

#[test]
fn focused_named_report_keeps_empty_states_and_unassociated_input_findings() {
    let raw = plan::parse(r#"{"format_version":"1.2","resource_changes":[{"address":"terraform_data.x","type":"terraform_data"}],"configuration":{"root_module":{"outputs":{"missing":{"expression":{"references":["terraform_data.missing.id"]}}}}}}"#).unwrap();
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
        relationships: vec![],
    };
    let options = view::Options {
        focus: Some("alpha:terraform_data.x".into()),
        depth: Some(0),
        ..Default::default()
    };
    let selected = view::apply(&architecture, &options).unwrap();
    let text = render(
        selected
            .states
            .iter()
            .map(|(id, graph)| (id.as_str(), &graph.coverage)),
        &options,
    );
    assert!(text.contains("| Resource instances | 2 | 1 |"));
    assert!(text.contains("| Resource instances | 1 | 0 |"));
    assert!(text.contains("| unresolved | 2 | 0 |"));
    assert!(text.contains("Unassociated input findings: 2"));
    assert!(!text.contains("terraform_data.missing"));
    assert!(text.contains("neighbor depth: 0"));
}

#[test]
fn report_uses_shared_counts_for_all_presentations_and_filters() {
    for input in [
        include_str!("../fixtures/association-plan.json"),
        include_str!("../fixtures/routes-plan.json"),
        include_str!("../fixtures/components-plan.json"),
    ] {
        let raw = plan::parse(input).unwrap();
        let options = view::Options {
            changed_only: true,
            ..Default::default()
        };
        let graph = view::single(&semantic::transform(&raw), &options).unwrap();
        let report = render(std::iter::once(("", &graph.coverage)), &options);
        assert!(report.contains(&format!(
            "| Resource instances | {} | {} |",
            raw.nodes.len(),
            graph.coverage.selected.resources
        )));
        assert!(report.contains("changed-only + required context"));
    }
}

#[test]
fn report_distinguishes_zero_unavailable_and_incomplete_and_escapes_focus() {
    let options = view::Options {
        focus: Some("<script>|`[x](javascript:x)\n".into()),
        ..Default::default()
    };
    let raw =
        plan::parse(r#"{"format_version":"1.2","complete":false,"resource_changes":[]}"#).unwrap();
    let graph = semantic::transform(&raw);
    let report = render(std::iter::once(("<state>", &graph.coverage)), &options);
    assert!(report.contains("| Resource instances | 0 | 0 |"));
    assert!(report.contains("Input completeness: incomplete"));
    assert!(!report.contains("<script>"));
    assert!(!report.contains("<state>"));
    assert!(!report.contains("[x]"));
    assert!(report.contains("&#124;"));
    assert!(report.contains("not applicable (0)"));
    let unavailable = render(
        std::iter::once(("", &Coverage::default())),
        &Default::default(),
    );
    assert!(unavailable.contains("analysis unavailable"));
    assert!(!unavailable.contains("| Resource instances | 0"));
}

#[test]
fn report_preserves_instance_and_state_counts_without_leaking_excluded_names() {
    let raw = plan::parse(r#"{"format_version":"1.2","resource_changes":[{"address":"aws_instance.a","type":"aws_instance"},{"address":"aws_instance.a","type":"aws_instance","deposed":"old"},{"address":"data.aws_region.hidden","type":"aws_region","mode":"data"},{"address":"terraform_data.x","type":"terraform_data"}]}"#).unwrap();
    let graph = semantic::transform(&raw);
    let report = render(
        [("beta", &graph.coverage), ("alpha", &graph.coverage)].into_iter(),
        &Default::default(),
    );
    assert!(report.contains("| Resource instances | 8 | 6 |"));
    assert!(report.contains("| Unknown account / applicable | 4 / 4 | 4 / 4 |"));
    assert!(report.find("State: alpha").unwrap() < report.find("State: beta").unwrap());
    assert!(!report.contains("data.aws_region.hidden"));
    assert!(!report.contains("aws_instance.a"));
}
