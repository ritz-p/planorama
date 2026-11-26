#[cfg(not(target_arch = "wasm32"))]
#[path = "cli/diff/diagrams.rs"]
mod diagrams;
#[cfg(not(target_arch = "wasm32"))]
#[path = "cli/diff/markdown.rs"]
mod markdown;

use crate::model::{
    EntityMode, PlanInput, StateId,
    cross_state::{Architecture, Failure, Mapping},
};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
struct Key {
    state: String,
    address: String,
    deposed: Option<String>,
    component: bool,
}

#[derive(Default)]
struct Snapshot {
    nodes: BTreeMap<Key, Record>,
    edges: BTreeSet<Link>,
    parents: BTreeMap<Key, Key>,
}

struct Record {
    provider: Option<String>,
    mode: Option<EntityMode>,
    previous: Option<String>,
    owner: Option<Key>,
    fields: Fields,
}

#[derive(Default)]
struct Fields {
    resource_type: String,
    role: Option<crate::model::ResourceRole>,
    scope: Option<crate::model::architecture::DeploymentScope>,
    cloudflare_scope: Option<crate::model::architecture::CloudflareScope>,
    deployment: Option<crate::model::architecture::DeploymentScope>,
    import: Option<crate::model::ImportMetadata>,
    members: BTreeSet<Key>,
}

impl Fields {
    fn changes(&self, next: &Self, mapping: &BTreeMap<Key, Key>) -> Vec<&'static str> {
        [
            ("type", self.resource_type != next.resource_type),
            ("role", self.role != next.role),
            (
                "scope",
                self.scope != next.scope
                    || self.cloudflare_scope != next.cloudflare_scope
                    || self.deployment != next.deployment,
            ),
            ("import", self.import != next.import),
            (
                "members",
                self.members
                    .iter()
                    .map(|k| mapping.get(k).unwrap_or(k))
                    .collect::<BTreeSet<_>>()
                    != next.members.iter().collect(),
            ),
        ]
        .into_iter()
        .filter_map(|(field, changed)| changed.then_some(field))
        .collect()
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
struct Link {
    from: Key,
    to: Key,
    kind: &'static str,
    direction: &'static str,
}

impl Link {
    fn canonical(mut self) -> Self {
        if self.direction == "undirected" && self.to < self.from {
            std::mem::swap(&mut self.from, &mut self.to);
        }
        self
    }
}

enum SnapshotError {
    Input(String),
    Mapping(Failure),
    Unresolved(Failure),
}

impl From<String> for SnapshotError {
    fn from(message: String) -> Self {
        Self::Input(message)
    }
}

impl SnapshotError {
    fn message(self) -> String {
        match self {
            Self::Input(message) => message,
            Self::Mapping(reason) => reason.description().into(),
            Self::Unresolved(reason) => format!(
                "cannot compare remote-state relationships: {}",
                reason.description()
            ),
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn snapshot(
    files: &[(StateId, String)],
    mappings: &[Mapping],
    diagrams: bool,
) -> Result<Snapshot, SnapshotError> {
    let inputs = files
        .iter()
        .map(|(state, path)| {
            Ok(PlanInput {
                state_id: state.clone(),
                plan_json: std::fs::read_to_string(path)
                    .map_err(|e| format!("cannot read {path}: {e}"))?,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let plans = crate::plan::parse_inputs_with(inputs, |_, json| crate::plan::parse(json))?;
    snapshot_plans(&plans, mappings, diagrams)
}

fn snapshot_plans(
    plans: &crate::model::MultiPlan,
    mappings: &[Mapping],
    diagrams: bool,
) -> Result<Snapshot, SnapshotError> {
    crate::semantic::cross_state::validate_mappings(plans, mappings)
        .map_err(SnapshotError::Mapping)?;
    let resolved = crate::semantic::cross_state::resolve(plans, mappings);
    if let Some(diagnostic) = resolved.diagnostics.first() {
        return Err(SnapshotError::Unresolved(diagnostic.reason));
    }
    let arch = Architecture {
        relationships: resolved.edges,
        states: plans
            .states
            .iter()
            .map(|(id, plan)| (id.clone(), crate::semantic::transform(plan)))
            .collect(),
    };
    let view = crate::view::apply(&arch, &Default::default())?;
    let mut snapshot = Snapshot::default();
    for (state, graph) in &view.states {
        let mut ids = BTreeMap::new();
        for node in &graph.nodes {
            let key = Key {
                state: state.as_str().into(),
                address: node.address.clone(),
                deposed: node.deposed_key.clone(),
                component: false,
            };
            ids.insert(&node.entity.id, key.clone());
            let fields = Fields {
                resource_type: node.resource_type.clone(),
                role: Some(node.role),
                scope: node.entity.scope.clone(),
                cloudflare_scope: node.entity.cloudflare_scope.clone(),
                deployment: node.entity.deployment_target.clone(),
                import: node.metadata.import,
                ..Default::default()
            };
            snapshot.nodes.insert(
                key,
                Record {
                    provider: node.provider.source().map(str::to_owned),
                    mode: Some(node.mode),
                    previous: node.previous_address.clone(),
                    owner: None,
                    fields,
                },
            );
        }
        if diagrams {
            let tree = crate::layout::ContainmentTree::new(graph);
            for (child, parent) in tree.parents.iter().enumerate() {
                if let Some(parent) = parent {
                    snapshot.parents.insert(
                        ids[&graph.nodes[child].entity.id].clone(),
                        ids[&graph.nodes[*parent].entity.id].clone(),
                    );
                }
            }
        }
        for component in &graph.components {
            let member_key = |address: &str, deposed: Option<String>| Key {
                state: state.as_str().into(),
                address: address.into(),
                deposed,
                component: false,
            };
            let owner = component
                .label_reference
                .as_ref()
                .map(|r| member_key(&r.source.address, r.source.deposed_key.clone()));
            let previous = owner.as_ref().and_then(|owner| {
                let previous = snapshot.nodes.get(owner)?.previous.as_ref()?;
                let suffix = &component.label_reference.as_ref()?.suffix;
                let id = |address: &str| {
                    crate::model::ArchitectureId::synthetic(
                        &component.kind,
                        &format!("{address}{suffix}"),
                    )
                };
                (id(&owner.address) == component.entity.id)
                    .then(|| id(previous).as_str().to_owned())
            });
            let key = Key {
                state: state.as_str().into(),
                address: component.entity.id.as_str().into(),
                deposed: None,
                component: true,
            };
            ids.insert(&component.entity.id, key.clone());
            snapshot.nodes.insert(
                key,
                Record {
                    provider: None,
                    mode: None,
                    previous,
                    owner,
                    fields: Fields {
                        resource_type: component.kind.clone(),
                        members: component
                            .members
                            .iter()
                            .map(|a| member_key(a, None))
                            .collect(),
                        ..Default::default()
                    },
                },
            );
        }
        for edge in &graph.relationships {
            let kind = relationship_kind(edge.kind);
            snapshot.edges.insert(
                Link {
                    from: ids
                        .get(&edge.from)
                        .ok_or_else(|| "missing source entity".to_owned())?
                        .clone(),
                    to: ids
                        .get(&edge.to)
                        .ok_or_else(|| "missing target entity".to_owned())?
                        .clone(),
                    kind,
                    direction: edge.directionality().as_str(),
                }
                .canonical(),
            );
        }
    }
    for edge in view.relationships() {
        let key = |e: &crate::model::cross_state::Endpoint| Key {
            state: e.state.as_str().into(),
            address: e.address.clone(),
            deposed: None,
            component: false,
        };
        snapshot.edges.insert(Link {
            from: key(&edge.from),
            to: key(&edge.to),
            kind: "dependency",
            direction: "directed",
        });
    }
    Ok(snapshot)
}

struct Change {
    before: Key,
    after: Key,
    fields: Vec<&'static str>,
}

struct Comparison {
    mapping: BTreeMap<Key, Key>,
    added: Vec<Key>,
    removed: Vec<Key>,
    changed: Vec<Change>,
    unknown: BTreeSet<Key>,
    added_links: BTreeSet<Link>,
    removed_links: BTreeSet<Link>,
}

type Names<'a> = Option<BTreeMap<&'a Key, usize>>;

impl Comparison {
    fn pseudonyms<'a>(
        &self,
        before: &'a Snapshot,
        after: &'a Snapshot,
    ) -> BTreeMap<&'a Key, usize> {
        let mut ids: BTreeMap<_, _> = before
            .nodes
            .keys()
            .enumerate()
            .map(|(i, key)| (key, i + 1))
            .collect();
        for (old, new) in &self.mapping {
            if let Some((key, _)) = after.nodes.get_key_value(new) {
                ids.insert(key, ids[old]);
            }
        }
        let mut next = before.nodes.len() + 1;
        for key in after.nodes.keys() {
            ids.entry(key).or_insert_with(|| {
                let id = next;
                next += 1;
                id
            });
        }
        ids
    }

    fn json(&self, names: &Names<'_>) -> serde_json::Value {
        use serde_json::json;
        let show = |k: &Key| match names {
            Some(ids) => json!(ids[k]),
            None => json!(k),
        };
        let link = |e: &Link| json!({"from":show(&e.from),"to":show(&e.to),"kind":e.kind,"direction":e.direction});
        json!({"schema_version":1,"resources":{
            "added": self.added.iter().map(show).collect::<Vec<_>>(),
            "removed": self.removed.iter().map(show).collect::<Vec<_>>(),
            "changed": self.changed.iter().map(|c| json!({"before":show(&c.before),"after":show(&c.after),"fields":c.fields})).collect::<Vec<_>>(),
            "unknown": self.unknown.iter().map(show).collect::<Vec<_>>()},
            "relationships":{"added":self.added_links.iter().map(link).collect::<Vec<_>>(),
                             "removed":self.removed_links.iter().map(link).collect::<Vec<_>>()}})
    }
}

fn compare(before: &Snapshot, after: &Snapshot) -> Comparison {
    let mut claims = BTreeMap::<Key, Vec<&Key>>::new();
    let mut added = Vec::new();
    for (key, record) in &after.nodes {
        let predecessor = record
            .previous
            .as_ref()
            .filter(|addr| *addr != &key.address)
            .map(|addr| Key {
                address: addr.clone(),
                ..key.clone()
            });
        let mut candidates = Vec::new();
        if before.nodes.contains_key(key) {
            candidates.push(key.clone());
        }
        if let Some(old) = predecessor.filter(|k| before.nodes.contains_key(k)) {
            candidates.push(old);
        }
        if candidates.is_empty() {
            added.push(key.clone());
        }
        for old in candidates {
            claims.entry(old).or_default().push(key);
        }
    }
    let mut counts = BTreeMap::new();
    for key in claims.values().flatten() {
        *counts.entry(*key).or_insert(0) += 1;
    }
    let mut uncertain = BTreeSet::new();
    let mut changed = Vec::new();
    let mut mapping = BTreeMap::new();
    let resources = claims.iter().filter(|(key, _)| !key.component);
    let components = claims.iter().filter(|(key, _)| key.component);
    for (old, targets) in resources.chain(components) {
        if let [new] = targets.as_slice() {
            let a = &before.nodes[old];
            let b = &after.nodes[*new];
            if counts[new] == 1
                && a.provider == b.provider
                && a.mode == b.mode
                && (old.component || a.provider.is_some())
                && match (&a.owner, &b.owner) {
                    (Some(old), Some(new)) => mapping.get(old) == Some(new),
                    (None, None) => true,
                    _ => false,
                }
            {
                mapping.insert(old.clone(), (*new).clone());
                let fields = a.fields.changes(&b.fields, &mapping);
                if (!old.component && old != *new) || !fields.is_empty() {
                    changed.push(Change {
                        before: old.clone(),
                        after: (*new).clone(),
                        fields,
                    });
                }
                continue;
            }
        }
        uncertain.insert(old.clone());
        uncertain.extend(targets.iter().map(|k| (*k).clone()));
    }
    let removed: Vec<_> = before
        .nodes
        .keys()
        .filter(|k| !claims.contains_key(*k))
        .cloned()
        .collect();
    let links = |edges: &BTreeSet<Link>, remap: bool| {
        edges
            .iter()
            .filter(|e| !uncertain.contains(&e.from) && !uncertain.contains(&e.to))
            .map(|e| {
                let key = |k: &Key| {
                    if remap {
                        mapping.get(k).unwrap_or(k).clone()
                    } else {
                        k.clone()
                    }
                };
                Link {
                    from: key(&e.from),
                    to: key(&e.to),
                    ..e.clone()
                }
                .canonical()
            })
            .collect::<BTreeSet<_>>()
    };
    let old = links(&before.edges, true);
    let new = links(&after.edges, false);
    Comparison {
        mapping,
        added,
        removed,
        changed,
        unknown: uncertain,
        added_links: new.difference(&old).cloned().collect(),
        removed_links: old.difference(&new).cloned().collect(),
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(PartialEq)]
enum ReportFormat {
    Json,
    Markdown,
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn run(mut args: impl Iterator<Item = String>) -> Result<(), String> {
    let mut files = Vec::new();
    let (mut before, mut after) = (Vec::new(), Vec::new());
    let mut mappings = Vec::new();
    let mut output = None;
    let mut svg_dir = None;
    let mut format = None;
    let mut anonymous = false;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--before-state" | "--after-state" => {
                let field = args.next().ok_or("missing ID=PATH")?;
                let (id, path) = field.split_once('=').ok_or("expected ID=PATH")?;
                let side = if arg == "--before-state" {
                    &mut before
                } else {
                    &mut after
                };
                side.push((StateId::new(id)?, path.to_owned()));
            }
            "--remote-state" => mappings.push(Mapping::parse(
                &args.next().ok_or("missing remote-state mapping")?,
            )?),
            "--anonymize" => anonymous = true,
            "--format" => {
                format = Some(match args.next().as_deref() {
                    Some("json") => ReportFormat::Json,
                    Some("markdown") => ReportFormat::Markdown,
                    _ => return Err("diff --format requires json or markdown".into()),
                });
            }
            "--svg-dir" => {
                svg_dir = Some(PathBuf::from(args.next().ok_or("missing SVG directory")?))
            }
            "-o" | "--output" => {
                output = Some(PathBuf::from(args.next().ok_or("missing output path")?))
            }
            flag if flag.starts_with('-') => {
                return Err("invalid diff option; use --help for usage".into());
            }
            _ => files.push(arg),
        }
    }
    if !files.is_empty() {
        if files.len() != 2 || !before.is_empty() || !after.is_empty() || !mappings.is_empty() {
            return Err("diff requires two files or two named-state snapshots".into());
        }
        before.push((StateId::new("default")?, files.remove(0)));
        after.push((StateId::new("default")?, files.remove(0)));
    }
    if svg_dir.is_some() && output.is_some() {
        return Err("--svg-dir and --output select different diff outputs".into());
    }
    if svg_dir.is_some() && format.is_some() {
        return Err("--svg-dir and --format select different diff outputs".into());
    }
    let output = output.unwrap_or_else(|| PathBuf::from("-"));
    for (_, path) in before.iter().chain(&after) {
        crate::cli::states::protect_input(path, &output)?;
    }
    let before = snapshot(&before, &mappings, svg_dir.is_some());
    let after = snapshot(&after, &mappings, svg_dir.is_some());
    let markdown = format == Some(ReportFormat::Markdown);
    let bytes = match (before, after) {
        (Ok(before), Ok(after)) => {
            let diff = compare(&before, &after);
            if let Some(dir) = svg_dir {
                return diagrams::write(&dir, &before, &after, &diff.mapping, anonymous)
                    .map_err(|e| e.to_string());
            }
            let names = anonymous.then(|| diff.pseudonyms(&before, &after));
            if markdown {
                markdown::render(&diff, &names, &before, &after).into_bytes()
            } else {
                serde_json::to_vec_pretty(&diff.json(&names)).map_err(|e| e.to_string())?
            }
        }
        (before, after) if markdown => {
            let mut message = String::from("# Architecture diff\n\n## Comparison unavailable\n\n");
            for (side, result) in [("Before", before), ("After", after)] {
                match result {
                    Err(SnapshotError::Input(error)) => return Err(error),
                    Err(reason) => {
                        message.push_str(&format!("- {side} snapshot — {}\n", reason.message()))
                    }
                    Ok(_) => {}
                }
            }
            message.into_bytes()
        }
        (Err(error), _) | (_, Err(error)) => return Err(error.message()),
    };
    if output == Path::new("-") {
        std::io::stdout().lock().write_all(&bytes)
    } else {
        std::fs::write(output, bytes)
    }
    .map_err(|e| e.to_string())
}

pub(crate) fn relationship_kind(kind: crate::model::EdgeKind) -> &'static str {
    use crate::model::EdgeKind;
    match kind {
        EdgeKind::Dependency => "dependency",
        EdgeKind::Connection => "connection",
        EdgeKind::Containment => "containment",
        EdgeKind::Association => "association",
        EdgeKind::EventSource => "event-source",
        EdgeKind::EventTarget => "event-target",
        EdgeKind::Subscription => "subscription",
    }
}

pub fn compare_plans(
    before: &str,
    after: &str,
    anonymize: bool,
) -> Result<serde_json::Value, crate::Error> {
    use crate::generation::{MAX_BROWSER_EDGES, MAX_BROWSER_INPUT_BYTES, MAX_BROWSER_NODES};
    if before.len().max(after.len()) > MAX_BROWSER_INPUT_BYTES {
        return Err(crate::Error::ResourceLimit);
    }
    #[cfg(target_arch = "wasm32")]
    crate::generation::memory::check_inspection(
        (before.len() as u64 + after.len() as u64) * 64 + 16 * 1024 * 1024,
    )?;
    let remaining = std::cell::Cell::new(32_768);
    for json in [before, after] {
        crate::generation::budget::validate_remaining(json, &remaining)?;
    }
    let snapshot = |json: &str| {
        let plan = crate::generation::parse_input(json)?;
        if plan.nodes.len() > MAX_BROWSER_NODES || plan.edges.len() > MAX_BROWSER_EDGES {
            return Err(crate::Error::ResourceLimit);
        }
        let plans = crate::model::MultiPlan {
            states: [(
                StateId::new("default").map_err(|_| crate::Error::GenerationFailed)?,
                plan,
            )]
            .into(),
        };
        let snapshot =
            snapshot_plans(&plans, &[], false).map_err(|_| crate::Error::GenerationFailed)?;
        if snapshot.nodes.len() > MAX_BROWSER_NODES || snapshot.edges.len() > MAX_BROWSER_EDGES {
            return Err(crate::Error::ResourceLimit);
        }
        Ok(snapshot)
    };
    let before = snapshot(before)?;
    let after = snapshot(after)?;
    let diff = compare(&before, &after);
    let names = anonymize.then(|| diff.pseudonyms(&before, &after));
    let result = diff.json(&names);
    if result.to_string().len() > crate::generation::MAX_BROWSER_OUTPUT_BYTES {
        return Err(crate::Error::ResourceLimit);
    }
    Ok(result)
}
