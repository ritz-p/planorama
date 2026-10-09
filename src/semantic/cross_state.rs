use crate::model::{EntityMode, MultiPlan, cross_state::*};
use std::collections::BTreeSet;

#[cfg(test)]
#[path = "../../tests/unit/semantic/cross_state.rs"]
mod tests;

pub fn resolve(plans: &MultiPlan, mappings: &[Mapping]) -> Resolution {
    let mut edges = BTreeSet::new();
    let mut diagnostics = BTreeSet::new();
    for (state, plan) in &plans.states {
        for reference in &plan.remote_references {
            let consumer = Endpoint {
                state: state.clone(),
                address: reference.consumer.clone(),
            };
            let candidates: BTreeSet<_> = mappings
                .iter()
                .filter(|m| &m.consumer == state && m.remote == reference.remote)
                .map(|m| &m.producer)
                .collect();
            let outcome = (|| {
                if candidates.is_empty() {
                    return Err(Failure::Unmapped);
                }
                if candidates.len() != 1 {
                    return Err(Failure::Ambiguous);
                }
                let producer = *candidates.first().expect("one candidate");
                let remote: Vec<_> = plan
                    .nodes
                    .iter()
                    .filter(|n| n.address == reference.remote && n.deposed_key.is_none())
                    .collect();
                if remote.len() != 1
                    || remote[0].resource_type != "terraform_remote_state"
                    || remote[0].mode != EntityMode::Data
                {
                    return Err(Failure::InvalidRemote);
                }
                let plan = plans.states.get(producer).ok_or(Failure::MissingState)?;
                let output = plan
                    .outputs
                    .iter()
                    .find(|o| o.name == reference.output)
                    .ok_or(Failure::MissingOutput)?;
                if !output.complete || output.sources.is_empty() {
                    return Err(Failure::IncompleteOutput);
                }
                Ok((producer, output))
            })();
            match outcome {
                Ok((producer, output)) => {
                    for source in &output.sources {
                        edges.insert(CrossStateEdge {
                            from: Endpoint {
                                state: producer.clone(),
                                address: source.clone(),
                            },
                            to: consumer.clone(),
                            remote: reference.remote.clone(),
                            output: reference.output.clone(),
                        });
                    }
                }
                Err(reason) => {
                    diagnostics.insert(CrossDiagnostic {
                        consumer,
                        remote: reference.remote.clone(),
                        output: reference.output.clone(),
                        reason,
                    });
                }
            }
        }
    }
    Resolution {
        edges: edges.into_iter().collect(),
        diagnostics: diagnostics.into_iter().collect(),
    }
}
