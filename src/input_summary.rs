use crate::model::{Action, EntityMode, ProviderIdentity, TerraformPlan};
use crate::{Error, NamedInput};
use std::collections::{BTreeMap, BTreeSet};

const MAX_OBJECTS: usize = 4096;
const MAX_PROVIDERS: usize = 64;

#[derive(Debug, Default, PartialEq, Eq, serde::Serialize)]
pub struct ActionCounts {
    pub unchanged: usize,
    pub create: usize,
    pub update: usize,
    pub delete: usize,
    pub replacement: usize,
    pub read: usize,
    pub other: usize,
}

#[derive(Debug, Default, PartialEq, Eq, serde::Serialize)]
pub struct ObjectCounts {
    pub resources: usize,
    pub objects: usize,
    pub current: usize,
    pub deposed: usize,
    pub managed: usize,
    pub data: usize,
    pub actions: ActionCounts,
}

impl ObjectCounts {
    fn add(&mut self, other: &Self) {
        self.resources += other.resources;
        self.objects += other.objects;
        self.current += other.current;
        self.deposed += other.deposed;
        self.managed += other.managed;
        self.data += other.data;
        self.actions.unchanged += other.actions.unchanged;
        self.actions.create += other.actions.create;
        self.actions.update += other.actions.update;
        self.actions.delete += other.actions.delete;
        self.actions.replacement += other.actions.replacement;
        self.actions.read += other.actions.read;
        self.actions.other += other.actions.other;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderEvidence {
    Explicit,
    Inferred,
    Unknown,
}

#[derive(Debug, PartialEq, Eq, serde::Serialize)]
pub struct ProviderSummary {
    pub source: Option<String>,
    pub evidence: ProviderEvidence,
    pub source_unavailable: bool,
    pub objects: usize,
}

#[derive(Debug, PartialEq, Eq, serde::Serialize)]
pub struct InputSummary {
    pub counts: ObjectCounts,
    pub providers: Vec<ProviderSummary>,
    pub omitted_provider_objects: usize,
}

#[derive(Debug, serde::Serialize)]
pub struct NamedSummaryEntry {
    pub state: String,
    pub summary: InputSummary,
}

#[derive(Debug, serde::Serialize)]
pub struct NamedInputSummary {
    pub inputs: Vec<NamedSummaryEntry>,
    pub aggregate: InputSummary,
}

#[derive(Debug, PartialEq, Eq, serde::Serialize)]
pub struct InspectionError {
    pub code: Error,
    pub input_index: Option<usize>,
}

impl std::fmt::Display for InspectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.code.fmt(f)
    }
}

impl std::error::Error for InspectionError {}

type Providers = BTreeMap<(Option<String>, ProviderEvidence), usize>;

fn source(value: &str) -> Option<String> {
    if value.len() > 256 {
        return None;
    }
    let parts: Vec<_> = value.split('/').collect();
    (parts.len() == 3
        && parts.iter().all(|part| {
            !part.is_empty()
                && part
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"._-".contains(&b))
        }))
    .then(|| value.to_owned())
}

fn collect(plan: &TerraformPlan) -> (ObjectCounts, Providers) {
    let mut counts = ObjectCounts::default();
    let mut addresses = BTreeSet::new();
    let mut providers = Providers::new();
    for node in &plan.nodes {
        addresses.insert(&node.address);
        counts.objects += 1;
        if node.deposed_key.is_some() {
            counts.deposed += 1;
        } else {
            counts.current += 1;
        }
        match node.mode {
            EntityMode::Managed => counts.managed += 1,
            EntityMode::Data => counts.data += 1,
        }
        match node.action {
            Action::Unchanged => counts.actions.unchanged += 1,
            Action::Create => counts.actions.create += 1,
            Action::Update => counts.actions.update += 1,
            Action::Delete => counts.actions.delete += 1,
            Action::Replace => counts.actions.replacement += 1,
            Action::Read => counts.actions.read += 1,
            Action::Other => counts.actions.other += 1,
        }
        let evidence = match node.provider {
            ProviderIdentity::Explicit(_) => ProviderEvidence::Explicit,
            ProviderIdentity::InferredAws | ProviderIdentity::InferredTerraform => {
                ProviderEvidence::Inferred
            }
            ProviderIdentity::Unknown => ProviderEvidence::Unknown,
        };
        *providers
            .entry((node.provider.source().and_then(source), evidence))
            .or_default() += 1;
    }
    counts.resources = addresses.len();
    (counts, providers)
}

fn finish(counts: ObjectCounts, providers: Providers) -> InputSummary {
    let mut rows = Vec::new();
    let mut omitted_provider_objects = 0;
    for ((source, evidence), objects) in providers {
        if rows.len() == MAX_PROVIDERS {
            omitted_provider_objects += objects;
        } else {
            rows.push(ProviderSummary {
                source_unavailable: source.is_none(),
                source,
                evidence,
                objects,
            });
        }
    }
    InputSummary {
        counts,
        providers: rows,
        omitted_provider_objects,
    }
}

fn parse(json: &str, values: &std::cell::Cell<usize>) -> Result<TerraformPlan, Error> {
    if json.len() > crate::generation::MAX_INPUT_BYTES {
        return Err(Error::InputTooLarge);
    }
    #[cfg(target_arch = "wasm32")]
    crate::generation::memory::check_inspection(json.len() as u64 * 64 + 16 * 1024 * 1024)?;
    crate::generation::budget::validate_remaining(json, values)?;
    let plan = crate::generation::parse_input(json)?;
    if plan.nodes.len() > MAX_OBJECTS || plan.edges.len() > 65_536 {
        return Err(Error::ResourceLimit);
    }
    Ok(plan)
}

pub fn inspect_input_summary(json: &str) -> Result<InputSummary, Error> {
    let plan = parse(json, &std::cell::Cell::new(32_768))?;
    let (counts, providers) = collect(&plan);
    Ok(finish(counts, providers))
}

pub fn inspect_named_input_summaries(
    inputs: &[NamedInput],
) -> Result<NamedInputSummary, InspectionError> {
    let fail = |code, input_index| InspectionError { code, input_index };
    if inputs.is_empty() || inputs.len() > 64 {
        return Err(fail(Error::EmptyInputs, None));
    }
    let mut names = BTreeSet::new();
    let mut bytes = 0;
    for (index, input) in inputs.iter().enumerate() {
        crate::model::StateId::new(&input.state)
            .map_err(|_| fail(Error::InvalidStateName, Some(index)))?;
        if !names.insert(&input.state) {
            return Err(fail(Error::DuplicateStateName, Some(index)));
        }
        bytes += input.json.len();
        if bytes > crate::generation::MAX_INPUT_BYTES {
            return Err(fail(Error::InputTooLarge, Some(index)));
        }
    }
    let mut rows = Vec::new();
    let mut total = ObjectCounts::default();
    let mut aggregate = Providers::new();
    let values = std::cell::Cell::new(32_768);
    for (index, input) in inputs.iter().enumerate() {
        let plan = parse(&input.json, &values).map_err(|code| fail(code, Some(index)))?;
        let (counts, providers) = collect(&plan);
        total.add(&counts);
        if total.objects > MAX_OBJECTS {
            return Err(fail(Error::ResourceLimit, Some(index)));
        }
        for (key, count) in &providers {
            *aggregate.entry(key.clone()).or_default() += count;
        }
        rows.push(NamedSummaryEntry {
            state: input.state.clone(),
            summary: finish(counts, providers),
        });
    }
    Ok(NamedInputSummary {
        inputs: rows,
        aggregate: finish(total, aggregate),
    })
}

#[cfg(test)]
#[path = "../tests/unit/input_summary.rs"]
mod tests;
