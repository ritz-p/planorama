use super::*;
use crate::{plan, semantic, view};

#[test]
fn selection_reasons_capture_direct_ancestors_routes_and_projection_once() {
    for (input, focus, direct_count, context_count, reason) in [
        (
            include_str!("../fixtures/containment-plan.json"),
            "aws_instance.app",
            1,
            2,
            "containment ancestor",
        ),
        (
            include_str!("../fixtures/routes-plan.json"),
            "aws_route.default",
            1,
            1,
            "verified Route Table owner",
        ),
        (
            include_str!("../fixtures/association-plan.json"),
            "aws_route_table_association.private",
            3,
            0,
            "lowered relationship provenance",
        ),
    ] {
        let source = semantic::transform(&plan::parse(input).unwrap());
        let options = view::Options {
            explain_selection: true,
            selection_details: 100,
            focus: Some(focus.into()),
            depth: Some(0),
            ..Default::default()
        };
        let graph = view::single(&source, &options).unwrap();
        let selection = graph.coverage.selection.as_ref().unwrap();
        assert_eq!(selection.reasons.len(), graph.coverage.selected.resources);
        assert_eq!(selection.direct(), direct_count, "{focus}");
        assert_eq!(
            graph.coverage.selected.resources - selection.direct(),
            context_count
        );
        assert!(
            selection
                .reasons
                .values()
                .any(|reasons| reasons.contains(reason))
        );
        let report = render(std::iter::once(("", &graph.coverage)), &options);
        assert!(report.contains(&format!(
            "Direct selection: {direct_count}. Additional context: {context_count}."
        )));
        assert!(!report.contains("TOP_SECRET"));
        let plain = view::single(
            &source,
            &view::Options {
                explain_selection: false,
                ..options
            },
        )
        .unwrap();
        assert!(plain.coverage.selection.is_none());
        assert_eq!(graph.nodes, plain.nodes);
        assert_eq!(graph.edges, plain.edges);
        assert_eq!(graph.relationships, plain.relationships);
    }
}

#[test]
fn selection_details_escape_bound_and_keep_current_deposed_and_data() {
    let mut input: serde_json::Value =
        serde_json::from_str(r#"{"format_version":"1.2","resource_changes":[]}"#).unwrap();
    for (address, mode, deposed) in [
        (
            "module.app.terraform_data.x[\"<script>|`&\"]",
            "managed",
            None,
        ),
        (
            "module.app.terraform_data.x[\"<script>|`&\"]",
            "managed",
            Some("old|key"),
        ),
        ("module.app.data.aws_ami.image", "data", None),
        ("module.other.terraform_data.hidden", "managed", None),
    ] {
        input["resource_changes"].as_array_mut().unwrap().push(serde_json::json!({"address":address,"mode":mode,"deposed":deposed,"type":"terraform_data","change":{"actions":["no-op"]}}));
    }
    let source = semantic::transform(&plan::parse(&input.to_string()).unwrap());
    let mut options = view::Options {
        explain_selection: true,
        selection_details: 2,
        modules: vec!["module.app".into()],
        ..Default::default()
    };
    let graph = view::single(&source, &options).unwrap();
    assert_eq!(graph.coverage.selection.as_ref().unwrap().direct(), 3);
    let text = render(std::iter::once(("state", &graph.coverage)), &options);
    assert!(text.contains("Shown: 2. Omitted selected identities: 1."));
    assert!(!text.contains("<script>"));
    assert!(!text.contains("terraform_data.hidden"));
    options.selection_details = 10;
    let text = render(std::iter::once(("state", &graph.coverage)), &options);
    assert!(text.contains("deposed&#58; old&#124;key"));
    assert!(text.contains("| data | current | direct |"));
    options.changed_only = true;
    let empty = view::single(&source, &options).unwrap();
    let text = render(std::iter::once(("state", &empty.coverage)), &options);
    assert!(text.contains("Direct selection: 0. Additional context: 0. Effective selected: 0."));
}

#[test]
fn cross_state_closure_records_overlapping_reasons() {
    use crate::model::{
        PlanInput, StateId,
        cross_state::{Architecture, Mapping},
    };
    let inputs = [
        ("producer", include_str!("../fixtures/multi/producer.json")),
        ("consumer", include_str!("../fixtures/multi/consumer.json")),
    ]
    .into_iter()
    .map(|(state, json)| PlanInput {
        state_id: StateId::new(state).unwrap(),
        plan_json: json.into(),
    })
    .collect();
    let plans = plan::parse_inputs(inputs).unwrap();
    let resolved = semantic::cross_state::resolve(
        &plans,
        &[Mapping::parse("consumer:data.terraform_remote_state.network=producer").unwrap()],
    );
    let architecture = Architecture {
        states: plans
            .states
            .iter()
            .map(|(id, raw)| (id.clone(), semantic::transform(raw)))
            .collect(),
        relationships: resolved.edges,
    };
    let options = view::Options {
        explain_selection: true,
        focus: Some("consumer:aws_instance.app".into()),
        depth: Some(0),
        ..Default::default()
    };
    let graph = view::apply(&architecture, &options).unwrap();
    for state in graph.states.values() {
        let selection = state.coverage.selection.as_ref().unwrap();
        assert_eq!(selection.reasons.len(), state.coverage.selected.resources);
        assert!(
            selection
                .reasons
                .values()
                .any(|reasons| reasons.contains("cross-state provenance"))
        );
    }
    assert!(
        graph
            .states
            .values()
            .flat_map(|g| g.coverage.selection.as_ref().unwrap().reasons.values())
            .any(|reasons| reasons.contains("direct selection")
                && reasons.contains("cross-state provenance"))
    );
}

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
            explain_selection: true,
            ..Default::default()
        };
        let graph = view::single(&semantic::transform(&raw), &options).unwrap();
        assert_eq!(
            graph.coverage.selection.as_ref().unwrap().reasons.len(),
            graph.coverage.selected.resources
        );
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
