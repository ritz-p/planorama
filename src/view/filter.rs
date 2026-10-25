use crate::model::architecture::{Action, ArchitectureGraph, ChangeMetadata, EdgeKind, StateId};
use crate::model::architecture::{Architecture, Endpoint};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Default)]
pub struct Options {
    pub spatial_types: Vec<String>,
    pub explain_selection: bool,
    pub selection_details: usize,
    pub modules: Vec<String>,
    pub html_semantic_diagnostics: bool,
    pub icon_mode: crate::icons::IconMode,
    pub changed_only: bool,
    pub focus: Option<String>,
    pub depth: Option<usize>,
}

impl Options {
    pub fn validate(&self) -> Result<(), String> {
        if self.icon_mode == crate::icons::IconMode::Icon {
            return Err("icon mode has been removed; use service-name".into());
        }
        super::purpose::validate(&self.spatial_types)?;
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

fn placement_available(
    graph: &crate::model::Graph,
    child: usize,
    selected: impl Fn(usize) -> bool,
) -> bool {
    let node = &graph.nodes[child];
    if node.resource_type != "aws_elasticache_replication_group" {
        return true;
    }
    graph
        .relationships
        .iter()
        .filter(|relationship| {
            relationship.to == node.entity.id && relationship.kind == EdgeKind::Containment
        })
        .flat_map(|relationship| &relationship.provenance)
        .all(|proof| match proof {
            crate::model::RelationshipProvenance::Reference { from, to }
                if node.entity.provenance.contains(to) =>
            {
                graph
                    .nodes
                    .iter()
                    .position(|source| source.entity.provenance.contains(from))
                    .is_some_and(&selected)
            }
            _ => true,
        })
}

pub fn apply(all: &Architecture, options: &Options) -> Result<Architecture, String> {
    options.validate()?;
    let modules = module_selections(all, &options.modules)?;
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
            if let Some(parent) = node
                .routes
                .owner
                .as_ref()
                .and_then(|_| crate::view::routes::owner(graph, node))
            {
                parents.push((
                    (state.clone(), parent),
                    (state.clone(), i),
                    "verified Route Table owner",
                ));
            }
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
                parents.push((from.clone(), to.clone(), "containment ancestor"));
            }
            if let Some(change) = &edge.change {
                if graph.nodes[edge.from].address == change.address {
                    continue;
                }
                if change.action != Action::Unchanged
                    || change.metadata != ChangeMetadata::default()
                    || change.previous_address.is_some()
                {
                    changed.extend([from.clone(), to.clone()]);
                }
                if focus == Some(change.address.as_str())
                    && !graph
                        .nodes
                        .iter()
                        .any(|n| n.address == change.address && n.deposed_key.is_none())
                {
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
        .filter_map(|e| {
            let from = endpoint(&e.from)?;
            let to = endpoint(&e.to)?;
            let crate::model::architecture::CrossStateProvenance::TerraformRemoteState {
                output_action,
                ..
            } = &e.provenance;
            if output_action.is_some_and(|action| action != Action::Unchanged) {
                changed.extend([from.clone(), to.clone()]);
            }
            Some((from, to))
        })
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
    if !options.modules.is_empty() {
        let lowered: BTreeSet<_> = all
            .states
            .iter()
            .flat_map(|(state, graph)| {
                let modules = &modules;
                graph
                    .edges
                    .iter()
                    .filter(move |edge| {
                        edge.change
                            .as_ref()
                            .is_some_and(|change| module_matches(modules, state, &change.address))
                    })
                    .flat_map(move |edge| [(state.clone(), edge.from), (state.clone(), edge.to)])
            })
            .collect();
        selected.retain(|(state, index)| {
            lowered.contains(&(state.clone(), *index))
                || all.states[state].nodes[*index]
                    .entity
                    .provenance
                    .iter()
                    .any(|id| module_matches(&modules, state, &id.address))
        });
    }
    let direct = options.explain_selection.then(|| selected.clone());
    let mut reasons: Option<BTreeMap<Key, BTreeSet<&'static str>>> =
        options.explain_selection.then(|| {
            selected
                .iter()
                .map(|key| {
                    let node = &all.states[&key.0].nodes[key.1];
                    let reason = if options.modules.is_empty()
                        || node
                            .entity
                            .provenance
                            .iter()
                            .any(|id| module_matches(&modules, &key.0, &id.address))
                    {
                        "direct selection"
                    } else {
                        "lowered relationship endpoint"
                    };
                    (key.clone(), BTreeSet::from([reason]))
                })
                .collect()
        });
    loop {
        let count = selected.len();
        for (parent, child, reason) in &parents {
            if selected.contains(child)
                && placement_available(&all.states[&child.0], child.1, |source| {
                    selected.contains(&(child.0.clone(), source))
                })
            {
                selected.insert(parent.clone());
                if let Some(reasons) = &mut reasons {
                    reasons.entry(parent.clone()).or_default().insert(reason);
                }
            }
        }
        for (from, to) in &cross {
            if selected.contains(from) || selected.contains(to) {
                selected.extend([from.clone(), to.clone()]);
                if let Some(reasons) = &mut reasons {
                    for endpoint in [from, to] {
                        reasons
                            .entry(endpoint.clone())
                            .or_default()
                            .insert("cross-state provenance");
                    }
                }
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
            if let (Some(reasons), Some(direct)) = (&reasons, &direct) {
                graph.0.coverage.selection = Some(selection_reasons(
                    &graph, state, &selected, direct, reasons, &modules, options,
                ));
            }
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
                .map(|(_, n)| {
                    let mut node = n.clone();
                    node.service_name = match options.icon_mode {
                        crate::icons::IconMode::Icon => (crate::icons::for_node(n).is_none()
                            && n.provider.is_aws()
                            && n.mode == crate::model::EntityMode::Managed)
                            .then(|| crate::provider::aws::service_name(&n.resource_type))
                            .flatten(),
                        crate::icons::IconMode::ServiceName => {
                            if n.provider.is_aws() && n.mode == crate::model::EntityMode::Managed {
                                crate::provider::aws::service_name(&n.resource_type)
                            } else {
                                crate::icons::for_node(n).map(|icon| icon.service_name())
                            }
                        }
                    };
                    node
                })
                .collect();
            graph.0.edges = graph
                .edges
                .iter()
                .filter_map(|e| {
                    if !options.modules.is_empty()
                        && e.change
                            .as_ref()
                            .is_some_and(|change| !module_matches(&modules, state, &change.address))
                    {
                        return None;
                    }
                    let mut e = e.clone();
                    e.from = *remap.get(&e.from)?;
                    e.to = *remap.get(&e.to)?;
                    Some(e)
                })
                .collect();
            let unplaced_cache: BTreeSet<_> = graph
                .nodes
                .iter()
                .enumerate()
                .filter(|(index, _)| !placement_available(&graph, *index, |_| true))
                .map(|(index, _)| index)
                .collect();
            graph.0.edges.retain(|edge| {
                edge.kind != EdgeKind::Containment || !unplaced_cache.contains(&edge.to)
            });
            let unplaced_cache: BTreeSet<_> = unplaced_cache
                .into_iter()
                .map(|index| graph.nodes[index].entity.id.clone())
                .collect();
            graph.0.relationships.retain(|relationship| {
                relationship.kind != EdgeKind::Containment
                    || !unplaced_cache.contains(&relationship.to)
            });
            let addresses: BTreeSet<_> = graph.0.nodes.iter().map(|n| n.address.as_str()).collect();
            let sources: BTreeSet<_> = graph
                .0
                .nodes
                .iter()
                .flat_map(|n| &n.entity.provenance)
                .collect();
            graph.0.components.retain(|c| {
                if c.kind == "organization_root" {
                    c.entity.provenance.iter().all(|id| sources.contains(id))
                } else {
                    c.members.iter().all(|m| addresses.contains(m.as_str()))
                }
            });
            let ids: BTreeSet<_> = graph.entities().map(|e| e.id.clone()).collect();
            graph.0.relationships.retain(|r| {
                ids.contains(&r.from)
                    && ids.contains(&r.to)
                    && (options.modules.is_empty()
                        || r.provenance.iter().all(|proof| match proof {
                            crate::model::RelationshipProvenance::Resource { source, .. } => {
                                module_matches(&modules, state, &source.address)
                            }
                            _ => true,
                        }))
            });
            crate::provider::aws::encryption::refresh(&mut graph.0);
            let represented = crate::coverage::represented(&graph);
            graph.0.coverage.retain(&represented);
            graph.0.coverage.focused = options.focus.is_some();
            graph.0.coverage.changed_only = options.changed_only;
            graph.0.coverage.embed_findings = options.html_semantic_diagnostics;
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
    let mut projected = Architecture {
        states,
        relationships,
    };
    super::purpose::apply(all, &mut projected, &options.spatial_types);
    crate::provider::aws::organization_accounts::link(all, &mut projected);
    Ok(projected)
}

fn selection_reasons(
    graph: &crate::model::Graph,
    state: &StateId,
    selected: &BTreeSet<Key>,
    direct: &BTreeSet<Key>,
    reasons: &BTreeMap<Key, BTreeSet<&'static str>>,
    modules: &BTreeMap<StateId, Vec<crate::model::modules::Address>>,
    options: &Options,
) -> crate::coverage::Selection {
    let mut result = crate::coverage::Selection::default();
    let mut node_indices = BTreeMap::new();
    for (index, node) in graph.nodes.iter().enumerate() {
        node_indices.entry(&node.entity.id).or_insert(index);
        if let Some(reasons) = reasons.get(&(state.clone(), index)) {
            for id in &node.entity.provenance {
                result
                    .reasons
                    .entry(id.clone())
                    .or_default()
                    .extend(reasons);
            }
        }
    }
    for relationship in &graph.relationships {
        let endpoints: Vec<_> = [&relationship.from, &relationship.to]
            .into_iter()
            .filter_map(|id| {
                node_indices
                    .get(id)
                    .copied()
                    .map(|index| (state.clone(), index))
            })
            .collect();
        if endpoints.len() != 2 || !endpoints.iter().all(|key| selected.contains(key)) {
            continue;
        }
        for proof in &relationship.provenance {
            let crate::model::RelationshipProvenance::Resource { source, change, .. } = proof
            else {
                continue;
            };
            if !options.modules.is_empty() && !module_matches(modules, state, &source.address) {
                continue;
            }
            let changed = change.action != Action::Unchanged
                || change.metadata != ChangeMetadata::default()
                || change.previous_address.is_some();
            let reasons = result.reasons.entry(source.clone()).or_default();
            reasons.insert("lowered relationship provenance");
            if endpoints.iter().all(|key| direct.contains(key))
                && (!options.changed_only || changed)
            {
                reasons.insert("direct selection");
            }
        }
    }
    result
}

fn module_selections(
    all: &Architecture,
    selectors: &[String],
) -> Result<BTreeMap<StateId, Vec<crate::model::modules::Address>>, String> {
    let mut result: BTreeMap<StateId, Vec<crate::model::modules::Address>> = BTreeMap::new();
    for selector in selectors {
        let (state, address) = crate::model::modules::selector(selector)?;
        let matches: Vec<_> = all
            .states
            .iter()
            .filter(|(id, graph)| {
                state.as_ref().is_none_or(|state| state == *id)
                    && graph
                        .coverage
                        .modules
                        .0
                        .keys()
                        .any(|module| address.contains(module))
            })
            .map(|(id, _)| id)
            .collect();
        match matches.as_slice() {
            [state] => result.entry((*state).clone()).or_default().push(address),
            [] => return Err(format!("unknown module selector: {selector:?}")),
            _ => {
                return Err(format!(
                    "ambiguous module selector: {selector:?}; use STATE:MODULE_ADDRESS"
                ));
            }
        }
    }
    Ok(result)
}

fn module_matches(
    modules: &BTreeMap<StateId, Vec<crate::model::modules::Address>>,
    state: &StateId,
    resource: &str,
) -> bool {
    crate::model::modules::Address::resource(resource).is_ok_and(|address| {
        modules
            .get(state)
            .into_iter()
            .flatten()
            .any(|selector| selector.contains(&address))
    })
}

#[cfg(test)]
#[path = "../../tests/unit/view/filter.rs"]
mod tests;

#[cfg(test)]
#[path = "../../tests/unit/view/modules.rs"]
mod module_tests;
