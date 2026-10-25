use crate::model::{EntityMode, MultiPlan, cross_state::*};
use std::collections::BTreeSet;

#[cfg(test)]
#[path = "../../tests/unit/semantic/cross_state.rs"]
mod tests;

pub fn resolve(plans: &MultiPlan, mappings: &[Mapping]) -> Resolution {
    crate::work::complete(resolve_step(plans, mappings))
}

fn valid_remote(plan: &crate::model::TerraformPlan, address: &str) -> bool {
    let mut matches = plan
        .nodes
        .iter()
        .filter(|n| n.address == address && n.deposed_key.is_none());
    matches.next().is_some_and(|node| {
        node.resource_type == "terraform_remote_state"
            && node.mode == EntityMode::Data
            && node.provider.is_terraform()
    }) && matches.next().is_none()
}

pub(crate) fn validate_mappings(plans: &MultiPlan, mappings: &[Mapping]) -> Result<(), Failure> {
    let mut selected = std::collections::BTreeMap::new();
    for mapping in mappings {
        let key = (&mapping.consumer, &mapping.remote);
        if selected
            .insert(key, &mapping.producer)
            .is_some_and(|previous| previous != &mapping.producer)
        {
            return Err(Failure::Ambiguous);
        }
    }
    for mapping in mappings {
        let plan = plans
            .states
            .get(&mapping.consumer)
            .ok_or(Failure::MissingState)?;
        if !plans.states.contains_key(&mapping.producer) {
            return Err(Failure::MissingState);
        }
        if !valid_remote(plan, &mapping.remote) {
            return Err(Failure::InvalidRemote);
        }
    }
    Ok(())
}

pub(crate) async fn resolve_step(plans: &MultiPlan, mappings: &[Mapping]) -> Resolution {
    let _timing = crate::timing::span("cross_state_resolution", &[("states", plans.states.len())]);
    let mut edges = BTreeSet::new();
    let mut diagnostics = BTreeSet::new();
    for (state, plan) in &plans.states {
        for reference in &plan.remote_references {
            crate::work::checkpoint().await;
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
                if !valid_remote(plan, &reference.remote) {
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
                        crate::work::checkpoint().await;
                        edges.insert(CrossStateEdge {
                            from: Endpoint {
                                state: producer.clone(),
                                address: source.clone(),
                            },
                            to: consumer.clone(),
                            provenance: CrossStateProvenance::TerraformRemoteState {
                                remote: reference.remote.clone(),
                                output: reference.output.clone(),
                                output_action: output.action,
                            },
                        });
                        crate::work::check_limit(edges.len(), 1024).await;
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
