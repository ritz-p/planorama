use crate::model::cross_state::{Architecture, Endpoint};
use crate::model::{Action, ArchitectureGraph, ChangeMetadata, EdgeKind, StateId};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Default)]
pub struct Options {
    pub changed_only: bool,
    pub focus: Option<String>,
    pub depth: Option<usize>,
}

impl Options {
    pub fn validate(&self) -> Result<(), String> {
        if self.depth.is_some() && self.focus.is_none() {
            return Err("--focus-depth requires --focus".into());
        }
        Ok(())
    }
}

pub fn single(graph: &ArchitectureGraph, options: &Options) -> Result<ArchitectureGraph, String> {
    let id = StateId::new("default")?;
    let all = Architecture {
        states: BTreeMap::from([(id.clone(), graph.clone())]),
        relationships: Vec::new(),
    };
    Ok(apply(&all, options)?
        .states
        .remove(&id)
        .expect("state is retained"))
}

type Key = (StateId, usize);

#[cfg(test)]
#[test]
fn filtering_does_not_mutate_source_graph_or_plan() {
    let raw =
        crate::plan::parse(include_str!("../../tests/fixtures/containment-plan.json")).unwrap();
    let original = raw.clone();
    let graph = super::transform(&raw);
    let before = graph.clone();
    single(
        &graph,
        &Options {
            focus: Some("aws_instance.app".into()),
            depth: Some(0),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(graph, before);
    assert_eq!(raw, original);
}

pub fn apply(all: &Architecture, options: &Options) -> Result<Architecture, String> {
    options.validate()?;
    let mut keys = BTreeSet::new();
    let mut changed = BTreeSet::new();
    let mut neighbors: BTreeMap<Key, BTreeSet<Key>> = BTreeMap::new();
    let mut parents = Vec::new();
    let mut matches = Vec::new();
    for (state, graph) in &all.states {
        let focus = options
            .focus
            .as_deref()
            .map(|s| s.strip_prefix(&format!("{}:", state.as_str())).unwrap_or(s));
        for (i, node) in graph.nodes.iter().enumerate() {
            let key = (state.clone(), i);
            keys.insert(key.clone());
            if node.action != Action::Unchanged
                || node.metadata != ChangeMetadata::default()
                || node.previous_address.is_some()
            {
                changed.insert(key.clone());
            }
            if focus == Some(node.address.as_str()) {
                matches.push(BTreeSet::from([key]));
            }
        }
        for edge in &graph.edges {
            let from = (state.clone(), edge.from);
            let to = (state.clone(), edge.to);
            neighbors
                .entry(from.clone())
                .or_default()
                .insert(to.clone());
            neighbors
                .entry(to.clone())
                .or_default()
                .insert(from.clone());
            if edge.kind == EdgeKind::Containment {
                parents.push((from.clone(), to.clone()));
            }
            if let Some(change) = &edge.change {
                if change.action != Action::Unchanged
                    || change.metadata != ChangeMetadata::default()
                    || change.previous_address.is_some()
                {
                    changed.extend([from.clone(), to.clone()]);
                }
                if focus == Some(change.address.as_str()) {
                    matches.push(BTreeSet::from([from, to]));
                }
            }
        }
    }
    let endpoint = |e: &Endpoint| {
        all.states
            .get(&e.state)
            .and_then(|g| {
                g.nodes
                    .iter()
                    .position(|n| n.address == e.address && n.deposed_key.is_none())
            })
            .map(|i| (e.state.clone(), i))
    };
    let cross: Vec<_> = all
        .relationships
        .iter()
        .filter_map(|e| Some((endpoint(&e.from)?, endpoint(&e.to)?)))
        .collect();
    for (from, to) in &cross {
        neighbors
            .entry(from.clone())
            .or_default()
            .insert(to.clone());
        neighbors
            .entry(to.clone())
            .or_default()
            .insert(from.clone());
    }
    let mut selected = if let Some(focus) = &options.focus {
        if matches.len() != 1 {
            return Err(format!(
                "{} focus address {focus:?}; use an exact Terraform address, qualified as STATE:ADDRESS for named states (deposed instances may be ambiguous)",
                if matches.is_empty() {
                    "unknown"
                } else {
                    "ambiguous"
                }
            ));
        }
        let mut selected = matches.pop().unwrap();
        let mut frontier = selected.clone();
        for _ in 0..options.depth.unwrap_or(1) {
            let next: BTreeSet<_> = frontier
                .iter()
                .flat_map(|k| neighbors.get(k).into_iter().flatten())
                .filter(|k| !selected.contains(*k))
                .cloned()
                .collect();
            if next.is_empty() {
                break;
            }
            selected.extend(next.iter().cloned());
            frontier = next;
        }
        selected
    } else {
        keys
    };
    if options.changed_only {
        selected.retain(|k| changed.contains(k));
    }
    // Close over required spatial ancestry and cross-state context, not local siblings.
    loop {
        let count = selected.len();
        for (parent, child) in &parents {
            if selected.contains(child) {
                selected.insert(parent.clone());
            }
        }
        for (from, to) in &cross {
            if selected.contains(from) || selected.contains(to) {
                selected.extend([from.clone(), to.clone()]);
            }
        }
        if selected.len() == count {
            break;
        }
    }
    let states = all
        .states
        .iter()
        .map(|(state, graph)| {
            let mut graph = graph.clone();
            let remap: BTreeMap<_, _> = graph
                .nodes
                .iter()
                .enumerate()
                .filter(|(i, _)| selected.contains(&(state.clone(), *i)))
                .enumerate()
                .map(|(new, (old, _))| (old, new))
                .collect();
            graph.0.nodes = graph
                .nodes
                .iter()
                .enumerate()
                .filter(|(i, _)| remap.contains_key(i))
                .map(|(_, n)| n.clone())
                .collect();
            graph.0.edges = graph
                .edges
                .iter()
                .filter_map(|e| {
                    let mut e = e.clone();
                    e.from = *remap.get(&e.from)?;
                    e.to = *remap.get(&e.to)?;
                    Some(e)
                })
                .collect();
            let addresses: BTreeSet<_> = graph.0.nodes.iter().map(|n| n.address.as_str()).collect();
            graph
                .0
                .components
                .retain(|c| c.members.iter().all(|m| addresses.contains(m.as_str())));
            let ids: BTreeSet<_> = graph.entities().map(|e| e.id.clone()).collect();
            graph
                .0
                .relationships
                .retain(|r| ids.contains(&r.from) && ids.contains(&r.to));
            (state.clone(), graph)
        })
        .collect();
    let relationships = all
        .relationships
        .iter()
        .filter(|e| {
            endpoint(&e.from).is_some_and(|k| selected.contains(&k))
                && endpoint(&e.to).is_some_and(|k| selected.contains(&k))
        })
        .cloned()
        .collect();
    Ok(Architecture {
        states,
        relationships,
    })
}
