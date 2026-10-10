use super::*;
use crate::model::{PlanInput, StateId};

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
