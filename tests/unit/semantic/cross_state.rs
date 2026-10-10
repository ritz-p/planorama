use super::*;
use crate::model::{PlanInput, StateId};

#[test]
fn changed_only_keeps_output_only_changes_and_respects_focus() {
    use serde_json::json;
    for action in [
        None,
        Some("no-op"),
        Some("create"),
        Some("update"),
        Some("delete"),
        Some("read"),
        Some("future"),
    ] {
        let mut inputs = Vec::new();
        for (state, fixture) in [
            (
                "producer",
                include_str!("../../fixtures/multi/producer.json"),
            ),
            (
                "consumer",
                include_str!("../../fixtures/multi/consumer.json"),
            ),
        ] {
            let mut value: serde_json::Value = serde_json::from_str(fixture).unwrap();
            for resource in value["resource_changes"].as_array_mut().unwrap() {
                resource["change"] = json!({"actions":["no-op"]});
            }
            if state == "producer" {
                value["resource_changes"].as_array_mut().unwrap().push(json!({"address":"aws_subnet.unrelated","type":"aws_subnet","change":{"actions":["no-op"]}}));
                if let Some(action) = action {
                    value["output_changes"] = json!({"subnet":{"actions":[action],"before":"PRIVATE_OUTPUT","after":"PRIVATE_OUTPUT"}});
                }
            }
            inputs.push(PlanInput {
                state_id: StateId::new(state).unwrap(),
                plan_json: value.to_string(),
            });
        }
        let plans = crate::plan::parse_inputs(inputs).unwrap();
        let resolution = resolve(
            &plans,
            &[Mapping::parse("consumer:data.terraform_remote_state.network=producer").unwrap()],
        );
        let architecture = Architecture {
            states: plans
                .states
                .iter()
                .map(|(id, plan)| (id.clone(), crate::semantic::transform(plan)))
                .collect(),
            relationships: resolution.edges,
        };
        for unrelated_focus in [false, true] {
            let view = crate::view::apply(
                &architecture,
                &crate::view::Options {
                    changed_only: true,
                    focus: unrelated_focus.then(|| "producer:aws_subnet.unrelated".into()),
                    depth: unrelated_focus.then_some(0),
                },
            )
            .unwrap();
            let kept = !unrelated_focus && action.is_some_and(|action| action != "no-op");
            assert_eq!(!view.relationships().is_empty(), kept, "{action:?}");
            let svg = crate::svg::render_states(&view, Default::default());
            assert_eq!(svg.contains("data-output-action="), kept);
            assert_eq!(
                svg.contains("data-terraform-address=\"aws_subnet.app\""),
                kept
            );
            assert_eq!(
                svg.contains("data-terraform-address=\"aws_instance.app\""),
                kept
            );
            assert!(!svg.contains("PRIVATE_OUTPUT"));
        }
    }
}

#[test]
fn same_named_output_actions_stay_with_the_mapped_producer() {
    let mut plans = plans();
    let mut second = plans.states[&StateId::new("producer").unwrap()].clone();
    for output in &mut second.outputs {
        output.action = Some(crate::model::Action::Delete);
    }
    plans.states.insert(StateId::new("second").unwrap(), second);
    for (producer, expected) in [
        ("producer", None),
        ("second", Some(crate::model::Action::Delete)),
    ] {
        let result = resolve(
            &plans,
            &[Mapping::parse(&format!(
                "consumer:data.terraform_remote_state.network={producer}"
            ))
            .unwrap()],
        );
        assert_eq!(result.edges.len(), 3);
        for edge in result.edges {
            let CrossStateProvenance::TerraformRemoteState { output_action, .. } = edge.provenance;
            assert_eq!(output_action, expected);
            assert_eq!(edge.from.state.as_str(), producer);
        }
    }
}
fn plans() -> MultiPlan {
    crate::plan::parse_inputs(vec![
        PlanInput {
            state_id: StateId::new("producer").unwrap(),
            plan_json: include_str!("../../fixtures/multi/producer.json").into(),
        },
        PlanInput {
            state_id: StateId::new("consumer").unwrap(),
            plan_json: include_str!("../../fixtures/multi/consumer.json").into(),
        },
    ])
    .unwrap()
}
#[test]
fn explicit_mapping_resolves_multiple_sources_and_preserves_failures() {
    let mut plans = plans();
    let mapping = Mapping::parse("consumer:data.terraform_remote_state.network=producer").unwrap();
    let result = resolve(&plans, &[mapping.clone()]);
    assert_eq!(result.edges.len(), 3);
    assert!(result.diagnostics.is_empty());
    assert!(
        result
            .edges
            .iter()
            .all(|e| e.from.state.as_str() == "producer" && e.to.state.as_str() == "consumer")
    );
    assert_eq!(
        resolve(&plans, &[]).diagnostics[0].reason,
        Failure::Unmapped
    );
    assert_eq!(
        resolve(
            &plans,
            &[
                mapping.clone(),
                Mapping::parse("consumer:data.terraform_remote_state.network=elsewhere").unwrap()
            ]
        )
        .diagnostics[0]
            .reason,
        Failure::Ambiguous
    );
    let producer = plans
        .states
        .get_mut(&StateId::new("producer").unwrap())
        .unwrap();
    producer.outputs.retain(|o| o.name != "subnet");
    producer
        .outputs
        .iter_mut()
        .find(|o| o.name == "both")
        .unwrap()
        .complete = false;
    let result = resolve(&plans, &[mapping]);
    assert!(result.edges.is_empty());
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| d.reason == Failure::MissingOutput)
    );
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| d.reason == Failure::IncompleteOutput)
    );
    assert!(!format!("{plans:?}").contains("TOP_SECRET"));
}
