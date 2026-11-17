use super::target::{Kind, Target};
use super::{AttributeReference, Evidence, Failure, Result, proof};
use std::collections::BTreeSet;
pub(super) fn label(kind: &str, path: &str) -> Option<&'static str> {
    match (kind, path) {
        ("cloudflare_queue_consumer", "queue_id") => Some("Consumed Queue"),
        ("cloudflare_queue_consumer", "script_name") => Some("Consumer Worker"),
        ("cloudflare_queue_consumer", "dead_letter_queue") => Some("Configured dead letter Queue"),
        _ => None,
    }
}
pub(super) fn targets<'a>(v: Evidence<'a>, path: &str) -> Result<Vec<Target<'a>>> {
    let kind = match (v.text("type")?, path) {
        ("worker" | "http_pull" | "notification", "queue_id") => Kind::QueueId,
        ("worker", "script_name") => Kind::WorkerName,
        ("worker", "dead_letter_queue") => Kind::QueueName,
        _ => return Err(Failure::UnsupportedContract),
    };
    Ok(vec![Target::new(v.text(path)?, kind)])
}
pub(super) fn owner(
    refs: &[AttributeReference],
    r: &AttributeReference,
    index: usize,
) -> Result<()> {
    if r.attribute == "dead_letter_queue" {
        let owner = refs
            .iter()
            .find(|owner| {
                owner.target == r.target
                    && owner.attribute == "queue_id"
                    && owner.service_reference_verified
            })
            .ok_or(Failure::OwnerMismatch)?;
        if owner.sources.contains(&index) {
            return Err(Failure::OwnerMismatch);
        }
    }
    Ok(())
}
pub(super) fn reject_dlq_cycles(refs: &mut [AttributeReference]) {
    let edges: Vec<_> = refs
        .iter()
        .enumerate()
        .filter_map(|(index, r)| {
            if r.attribute != "dead_letter_queue" || !r.service_reference_verified {
                return None;
            }
            let owner = refs.iter().find(|owner| {
                owner.target == r.target
                    && owner.attribute == "queue_id"
                    && owner.service_reference_verified
            })?;
            Some((index, *owner.sources.first()?, *r.sources.first()?))
        })
        .collect();
    for &(index, owner, target) in &edges {
        let mut pending = vec![target];
        let mut visited = BTreeSet::new();
        while let Some(node) = pending.pop() {
            if node == owner {
                proof::record(&mut refs[index], Err(Failure::OwnerMismatch));
                break;
            }
            if visited.insert(node) {
                pending.extend(
                    edges
                        .iter()
                        .filter(|(_, from, _)| *from == node)
                        .map(|(_, _, to)| *to),
                );
            }
        }
    }
}
