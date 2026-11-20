use crate::model::{
    ArchitectureEntity, ArchitectureId, AwsAccountScope, ChangeMetadata, EdgeChange, EntityMode,
    Graph, ProviderConfiguration, ProviderIdentity, RegionScope, RelationshipProvenance, StateId,
    TerraformEntityId,
    architecture::{Architecture, ArchitectureEntityKind, DeploymentScope},
    cross_state::{CrossStateProvenance, Endpoint},
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Default)]
struct Projection {
    reserved: BTreeSet<String>,
    names: BTreeMap<(&'static str, String), String>,
    counts: BTreeMap<&'static str, usize>,
    entities: BTreeMap<ArchitectureId, ArchitectureId>,
    accounts: BTreeMap<String, AwsAccountScope>,
}

pub(super) fn single(graph: &mut Graph) -> Result<(), String> {
    let mut projection = Projection::default();
    projection.reserve(graph);
    projection.identities(graph);
    projection.graph(graph)
}

pub(super) fn named(architecture: &mut Architecture) -> Result<(), String> {
    let mut projection = Projection::default();
    for (state, graph) in &architecture.states {
        projection.reserve_text(state.as_str());
        projection.reserve(graph);
    }
    for edge in &architecture.relationships {
        let CrossStateProvenance::TerraformRemoteState { remote, output, .. } = &edge.provenance;
        projection.reserve_text(remote);
        projection.reserve_text(output);
    }
    for graph in architecture.states.values() {
        projection.identities(graph);
    }
    let states = std::mem::take(&mut architecture.states);
    for (id, mut graph) in states {
        let id = projection.state(&id)?;
        projection.graph(&mut graph.0)?;
        architecture.states.insert(id, graph);
    }
    for edge in &mut architecture.relationships {
        projection.endpoint(&mut edge.from)?;
        projection.endpoint(&mut edge.to)?;
        let CrossStateProvenance::TerraformRemoteState {
            remote,
            output,
            output_action: _,
        } = &mut edge.provenance;
        *remote = projection.address(remote);
        *output = projection.name("output", output);
    }
    Ok(())
}

impl Projection {
    fn reserve_text(&mut self, value: &str) {
        self.reserved.extend(
            value
                .split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
                .filter(|part| !part.is_empty())
                .map(str::to_owned),
        );
    }

    fn reserve_source(&mut self, source: &TerraformEntityId) {
        self.reserve_text(&source.address);
        if let Some(key) = &source.deposed_key {
            self.reserve_text(key);
        }
    }

    fn reserve(&mut self, graph: &Graph) {
        for entity in graph.entities() {
            for source in &entity.provenance {
                self.reserve_source(source);
            }
            if let Some(scope) = &entity.cloudflare_scope {
                for value in scope.account.iter().chain(&scope.zone) {
                    self.reserve_text(value);
                }
            }
            for scope in entity.scope.iter().chain(&entity.deployment_target) {
                self.reserve_text(&scope.provider);
                if let Some(region) = &scope.region {
                    self.reserve_text(&region.0);
                }
            }
        }
        for node in &graph.nodes {
            for value in [&node.address, &node.module, &node.resource_type]
                .into_iter()
                .chain(&node.previous_address)
                .chain(&node.deposed_key)
            {
                self.reserve_text(value);
            }
            if let Some(provider) = node.provider.source() {
                self.reserve_text(provider);
            }
            if let Some(configuration) = &node.provider_configuration {
                self.reserve_text(&configuration.key);
                if let Some(alias) = &configuration.alias {
                    self.reserve_text(alias);
                }
                if let Some(region) = &configuration.region {
                    self.reserve_text(&region.0);
                }
            }
            for reference in &node.references {
                self.reserve_text(&reference.attribute);
                for target in &reference.targets {
                    self.reserve_source(&target.id);
                    if let Some(state) = &target.state {
                        self.reserve_text(state.as_str());
                    }
                }
            }
            for entry in &node.routes.entries {
                for value in entry
                    .destination
                    .iter()
                    .chain(&entry.target_kind)
                    .chain(&entry.target_attribute)
                {
                    self.reserve_text(value);
                }
                if let Some(target) = &entry.target {
                    self.reserve_source(target);
                }
            }
        }
        for relationship in &graph.relationships {
            for proof in &relationship.provenance {
                match proof {
                    RelationshipProvenance::Reference { from, to }
                    | RelationshipProvenance::ConfiguredValue { from, to, .. } => {
                        self.reserve_source(from);
                        self.reserve_source(to);
                    }
                    RelationshipProvenance::Resource { source, change, .. } => {
                        self.reserve_source(source);
                        self.reserve_text(&change.address);
                        if let Some(previous) = &change.previous_address {
                            self.reserve_text(previous);
                        }
                    }
                }
            }
        }
        for component in &graph.components {
            for value in [&component.id, &component.kind, &component.label]
                .into_iter()
                .chain(&component.members)
            {
                self.reserve_text(value);
            }
        }
        for check in &graph.checks {
            for instance in &check.instances {
                if let Some(resource) = &instance.resource {
                    self.reserve_text(resource);
                }
            }
        }
    }

    fn name(&mut self, kind: &'static str, value: &str) -> String {
        let key = (kind, value.to_owned());
        if let Some(name) = self.names.get(&key) {
            return name.clone();
        }
        let count = self.counts.entry(kind).or_default();
        *count += 1;
        let mut name = format!("{kind}_{count}");
        while name == value || self.reserved.contains(&name) {
            *count += 1;
            name = format!("{kind}_{count}");
        }
        self.names.insert(key, name.clone());
        name
    }

    fn module(&mut self, value: &str) -> String {
        if value == "root" {
            return value.into();
        }
        match crate::model::modules::Address::parse(value) {
            Ok(module) => module.anonymized(|part| self.name("group", part)),
            Err(_) => format!("module.{}", self.name("group", value)),
        }
    }

    fn resource_type(&mut self, value: &str) -> String {
        if crate::provider::aws::service_name(value).is_some()
            || [EntityMode::Managed, EntityMode::Data]
                .into_iter()
                .any(|mode| crate::provider::cloudflare::known(value, mode))
            || matches!(value, "terraform_data" | "terraform_remote_state")
        {
            value.into()
        } else {
            self.name("type", value)
        }
    }

    fn address(&mut self, value: &str) -> String {
        let module = crate::model::module_of(value);
        let local = if module == "root" {
            value
        } else {
            value
                .strip_prefix(module)
                .and_then(|v| v.strip_prefix('.'))
                .unwrap_or("")
        };
        let (mode, local) = local
            .strip_prefix("data.")
            .map_or(("", local), |v| ("data.", v));
        let kind = self.resource_type(local.split('.').next().unwrap_or(""));
        let name = self.name("resource", value);
        let prefix = if module == "root" {
            String::new()
        } else {
            format!("{}.", self.module(module))
        };
        format!("{prefix}{mode}{kind}.{name}")
    }

    fn source(&mut self, source: &mut TerraformEntityId) {
        source.address = self.address(&source.address);
        if let Some(key) = &mut source.deposed_key {
            *key = self.name("deposed", key);
        }
    }

    fn identities(&mut self, graph: &Graph) {
        for entity in graph.entities() {
            if self.entities.contains_key(&entity.id) {
                continue;
            }
            let id = if entity.kind == ArchitectureEntityKind::Terraform {
                entity.provenance.first().map(|source| {
                    let mut source = source.clone();
                    self.source(&mut source);
                    ArchitectureEntity::terraform(source).id
                })
            } else {
                None
            };
            let id = id.unwrap_or_else(|| {
                ArchitectureId::synthetic("anonymous", &self.name("entity", entity.id.as_str()))
            });
            self.entities.insert(entity.id.clone(), id);
        }
    }

    fn architecture_id(&mut self, id: &mut ArchitectureId) {
        *id = self.entities.get(id).cloned().unwrap_or_else(|| {
            ArchitectureId::synthetic("anonymous", &self.name("entity", id.as_str()))
        });
    }

    fn account(&mut self, account: &mut AwsAccountScope) {
        let next = self.accounts.len() + 1;
        *account = self
            .accounts
            .entry(account.as_str().to_owned())
            .or_insert_with(|| AwsAccountScope::anonymized(next))
            .clone();
    }

    fn region(&mut self, region: &mut RegionScope) {
        if !matches!(
            region.0.as_str(),
            "us-east-1"
                | "us-east-2"
                | "us-west-1"
                | "us-west-2"
                | "eu-west-1"
                | "eu-west-2"
                | "eu-west-3"
                | "eu-central-1"
                | "ap-northeast-1"
                | "ap-northeast-2"
                | "ap-southeast-1"
                | "ap-southeast-2"
        ) {
            region.0 = self.name("region", &region.0);
        }
    }

    fn provider_name(&mut self, value: &mut String) {
        if !matches!(
            value.as_str(),
            "AWS"
                | "Cloudflare"
                | "Terraform"
                | "registry.terraform.io/hashicorp/aws"
                | "registry.terraform.io/cloudflare/cloudflare"
                | "terraform.io/builtin/terraform"
        ) {
            *value = self.name("provider", value);
        }
    }

    fn provider(&mut self, provider: &mut ProviderIdentity) {
        if let ProviderIdentity::Explicit(value) = provider {
            self.provider_name(value);
        }
    }

    fn scope(&mut self, scope: &mut DeploymentScope) {
        let DeploymentScope {
            provider,
            region,
            account,
            global: _,
        } = scope;
        self.provider_name(provider);
        if let Some(region) = region {
            self.region(region);
        }
        if let Some(account) = account {
            self.account(account);
        }
    }

    fn configuration(&mut self, configuration: &mut ProviderConfiguration) {
        let ProviderConfiguration {
            key,
            alias,
            region,
            account,
            account_conflict: _,
        } = configuration;
        *key = self.name("configuration", key);
        if let Some(alias) = alias {
            *alias = self.name("alias", alias);
        }
        if let Some(region) = region {
            self.region(region);
        }
        if let Some(account) = account {
            self.account(account);
        }
    }

    fn entity(&mut self, entity: &mut ArchitectureEntity) {
        let ArchitectureEntity {
            pool_origins: _,
            cloudflare_scope,
            deployment_target,
            represented_account,
            id,
            kind: _,
            provenance,
            scope,
        } = entity;
        self.architecture_id(id);
        for source in provenance {
            self.source(source);
        }
        if let Some(scope) = scope {
            self.scope(scope);
        }
        if let Some(scope) = deployment_target {
            self.scope(scope);
        }
        if let Some(account) = represented_account {
            self.account(account);
        }
        if let Some(scope) = cloudflare_scope {
            if let Some(account) = &mut scope.account {
                *account = self.name("cloudflare_account", account);
            }
            if let Some(zone) = &mut scope.zone {
                *zone = self.name("zone", zone);
            }
        }
    }

    fn metadata(&mut self, metadata: &mut ChangeMetadata) {
        let ChangeMetadata {
            changed_paths,
            import: _,
            state_removal: _,
            drift: _,
            action_reason,
            replace_paths,
            relevant_attributes,
        } = metadata;
        *changed_paths = Default::default();
        *action_reason = None;
        *replace_paths = None;
        *relevant_attributes = None;
    }

    fn change(&mut self, change: &mut EdgeChange) {
        let EdgeChange {
            directionality: _,
            address,
            previous_address,
            metadata,
            action: _,
        } = change;
        *address = self.address(address);
        if let Some(previous) = previous_address {
            *previous = self.address(previous);
        }
        self.metadata(metadata);
    }

    fn state(&mut self, state: &StateId) -> Result<StateId, String> {
        StateId::new(&self.name("state", state.as_str()))
    }

    fn endpoint(&mut self, endpoint: &mut Endpoint) -> Result<(), String> {
        endpoint.state = self.state(&endpoint.state)?;
        endpoint.address = self.address(&endpoint.address);
        Ok(())
    }

    fn graph(&mut self, graph: &mut Graph) -> Result<(), String> {
        let Graph {
            coverage,
            relationships,
            components,
            checks,
            status: _,
            nodes,
            edges,
        } = graph;
        *coverage = Default::default();
        for node in nodes {
            let crate::model::Node {
                member_owner,
                service_name: _,
                routes,
                placement: _,
                references,
                entity,
                address,
                deposed_key,
                previous_address,
                metadata,
                resource_type,
                provider,
                provider_configuration,
                module,
                action: _,
                mode: _,
                role: _,
            } = node;
            *address = self.address(address);
            *module = self.module(module);
            *resource_type = self.resource_type(resource_type);
            self.provider(provider);
            if let Some(configuration) = provider_configuration {
                self.configuration(configuration);
            }
            self.entity(entity);
            if let Some(key) = deposed_key {
                *key = self.name("deposed", key);
            }
            if let Some(previous) = previous_address {
                *previous = self.address(previous);
            }
            self.metadata(metadata);
            if let Some(owner) = member_owner {
                self.source(&mut owner.target);
            }
            if let Some(owner) = &mut routes.owner {
                self.source(owner);
            }
            for entry in &mut routes.entries {
                if let Some(value) = &mut entry.destination {
                    *value = self.name("destination", value);
                }
                if let Some(target) = &mut entry.target {
                    self.source(target);
                }
                if let Some(kind) = &mut entry.target_kind {
                    *kind = self.resource_type(kind);
                }
                if let Some(attribute) = &mut entry.target_attribute {
                    *attribute = self.name("attribute", attribute);
                }
            }
            references.retain(|reference| {
                !matches!(
                    reference.kind,
                    crate::model::ReferenceKind::ConfiguredScope(_)
                )
            });
            for reference in references {
                if member_owner
                    .as_ref()
                    .is_none_or(|owner| owner.attribute != reference.attribute)
                {
                    reference.attribute = self.name("attribute", &reference.attribute);
                }
                for target in &mut reference.targets {
                    self.source(&mut target.id);
                    if let Some(state) = &mut target.state {
                        *state = self.state(state)?;
                    }
                    if let Some(account) = &mut target.account_scope {
                        self.account(account);
                    }
                }
            }
        }
        for edge in edges {
            if let Some(change) = &mut edge.change {
                self.change(change);
            }
        }
        for relationship in relationships {
            self.architecture_id(&mut relationship.from);
            self.architecture_id(&mut relationship.to);
            for proof in &mut relationship.provenance {
                match proof {
                    RelationshipProvenance::Reference { from, to }
                    | RelationshipProvenance::ConfiguredValue { from, to, .. } => {
                        self.source(from);
                        self.source(to);
                    }
                    RelationshipProvenance::Resource {
                        source,
                        change,
                        deployment,
                    } => {
                        self.source(source);
                        self.change(change);
                        if let Some(deployment) = deployment {
                            if let Some(scope) = &mut deployment.scope {
                                self.scope(scope);
                            }
                            self.provider(&mut deployment.provider);
                            if let Some(configuration) = &mut deployment.configuration {
                                self.configuration(configuration);
                            }
                        }
                    }
                }
            }
        }
        for component in components {
            self.entity(&mut component.entity);
            component.id = self.name("component", &component.id);
            if !matches!(
                component.kind.as_str(),
                "organization_root" | "load_balancer"
            ) {
                component.kind = self.name("category", &component.kind);
            }
            component.label = "Anonymized component".into();
            if let Some(reference) = &mut component.label_reference {
                self.source(&mut reference.source);
                if !reference.suffix.is_empty()
                    && reference
                        .suffix
                        .strip_prefix(".roots[")
                        .and_then(|s| s.strip_suffix(']'))
                        .is_none_or(|s| s.parse::<usize>().is_err())
                {
                    reference.suffix.clear();
                }
            }
            for member in &mut component.members {
                *member = self.address(member);
            }
        }
        for check in checks {
            sanitize_status(&mut check.status);
            for instance in &mut check.instances {
                sanitize_status(&mut instance.status);
                if let Some(resource) = &mut instance.resource {
                    *resource = self.address(resource);
                }
            }
        }
        Ok(())
    }
}

fn sanitize_status(status: &mut Option<String>) {
    if status
        .as_deref()
        .is_some_and(|s| !matches!(s, "pass" | "fail" | "error" | "unknown"))
    {
        *status = Some("withheld".into());
    }
}

#[cfg(test)]
#[path = "../../tests/unit/anonymize.rs"]
mod tests;
