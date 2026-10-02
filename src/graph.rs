use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Unchanged,
    Create,
    Update,
    Delete,
    Replace,
    Read,
    Other,
}

impl Action {
    pub fn label(self) -> &'static str {
        match self {
            Self::Unchanged => "unchanged",
            Self::Create => "create",
            Self::Update => "update",
            Self::Delete => "delete",
            Self::Replace => "replace",
            Self::Read => "read",
            Self::Other => "other",
        }
    }
    fn parse(actions: &Value) -> Self {
        let actions: Vec<_> = actions
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .collect();
        if actions.contains(&"create") && actions.contains(&"delete") {
            return Self::Replace;
        }
        match actions.as_slice() {
            ["no-op"] => Self::Unchanged,
            ["create"] => Self::Create,
            ["update"] => Self::Update,
            ["delete"] => Self::Delete,
            ["read"] => Self::Read,
            _ => Self::Other,
        }
    }
}

#[derive(Debug)]
pub struct Node {
    pub address: String,
    pub resource_type: String,
    pub module: String,
    pub action: Action,
}

pub struct Graph {
    pub nodes: Vec<Node>,
    /// (dependency, dependent), sorted and deduplicated.
    pub edges: Vec<(usize, usize)>,
}

// Strip instance keys without splitting dots or brackets inside quoted keys.
fn static_address(address: &str) -> String {
    let mut result = String::new();
    let (mut depth, mut quoted, mut escaped) = (0usize, false, false);
    for c in address.chars() {
        if depth > 0 {
            if escaped {
                escaped = false;
                continue;
            }
            if quoted && c == '\\' {
                escaped = true;
                continue;
            }
            if c == '"' {
                quoted = !quoted;
            }
            if !quoted {
                if c == '[' {
                    depth += 1;
                }
                if c == ']' {
                    depth -= 1;
                }
            }
        } else if c == '[' {
            depth = 1;
        } else {
            result.push(c);
        }
    }
    result
}

fn module_of(address: &str) -> String {
    let mut boundary = 0;
    let (mut depth, mut quoted, mut escaped) = (0usize, false, false);
    let mut parts = Vec::new();
    for (i, c) in address.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if quoted && c == '\\' {
            escaped = true;
            continue;
        }
        if c == '"' && depth > 0 {
            quoted = !quoted;
        }
        if !quoted {
            if c == '[' {
                depth += 1;
            }
            if c == ']' {
                depth = depth.saturating_sub(1);
            }
            if c == '.' && depth == 0 {
                parts.push((boundary, i));
                boundary = i + 1;
            }
        }
    }
    parts.push((boundary, address.len()));
    let mut count = 0;
    while count + 1 < parts.len() && &address[parts[count].0..parts[count].1] == "module" {
        count += 2;
    }
    if count == 0 {
        "root".into()
    } else {
        address[..parts[count - 1].1].into()
    }
}

fn collect_values(module: &Value, nodes: &mut BTreeMap<String, Node>, data_only: bool) {
    for resource in module["resources"].as_array().into_iter().flatten() {
        if data_only && resource["mode"] != "data" {
            continue;
        }
        if let Some(address) = resource["address"].as_str() {
            nodes.entry(address.into()).or_insert_with(|| Node {
                address: address.into(),
                module: module_of(address),
                resource_type: resource["type"].as_str().unwrap_or("resource").into(),
                action: Action::Unchanged,
            });
        }
    }
    for child in module["child_modules"].as_array().into_iter().flatten() {
        collect_values(child, nodes, data_only);
    }
}

fn references(value: &Value, found: &mut BTreeSet<String>) {
    match value {
        Value::Object(object) => {
            if let Some(refs) = object.get("references").and_then(Value::as_array) {
                found.extend(refs.iter().filter_map(Value::as_str).map(str::to_owned));
            }
            for (key, value) in object {
                if key != "constant_value" && key != "references" {
                    references(value, found);
                }
            }
        }
        Value::Array(values) => {
            for value in values {
                references(value, found);
            }
        }
        _ => {}
    }
}

fn qualify(scope: &str, reference: &str) -> String {
    if scope.is_empty() {
        reference.into()
    } else {
        format!("{scope}.{reference}")
    }
}

// Module inputs and outputs become aliases, allowing references across modules.
fn collect_config(
    module: &Value,
    scope: &str,
    inherited: &BTreeSet<String>,
    symbols: &mut BTreeMap<String, BTreeSet<String>>,
) {
    for resource in module["resources"].as_array().into_iter().flatten() {
        if let Some(address) = resource["address"].as_str() {
            let mut refs = BTreeSet::new();
            references(&resource["expressions"], &mut refs);
            references(&resource["count_expression"], &mut refs);
            references(&resource["for_each_expression"], &mut refs);
            refs.extend(
                resource["depends_on"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_str)
                    .map(str::to_owned),
            );
            let mut qualified: BTreeSet<_> = refs.into_iter().map(|r| qualify(scope, &r)).collect();
            qualified.extend(inherited.iter().cloned());
            symbols.insert(qualify(scope, address), qualified);
        }
    }
    if let Some(outputs) = module["outputs"].as_object() {
        for (name, output) in outputs {
            let mut refs = BTreeSet::new();
            references(output, &mut refs);
            symbols.insert(
                qualify(scope, &format!("output.{name}")),
                refs.into_iter().map(|r| qualify(scope, &r)).collect(),
            );
        }
    }
    if let Some(calls) = module["module_calls"].as_object() {
        for (name, call) in calls {
            let child = qualify(scope, &format!("module.{name}"));
            if let Some(inputs) = call["expressions"].as_object() {
                for (name, expression) in inputs {
                    let mut refs = BTreeSet::new();
                    references(expression, &mut refs);
                    symbols.insert(
                        format!("{child}.var.{name}"),
                        refs.into_iter().map(|r| qualify(scope, &r)).collect(),
                    );
                }
            }
            let mut call_refs = BTreeSet::new();
            references(&call["count_expression"], &mut call_refs);
            references(&call["for_each_expression"], &mut call_refs);
            call_refs.extend(
                call["depends_on"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_str)
                    .map(str::to_owned),
            );
            let mut dependencies = inherited.clone();
            dependencies.extend(call_refs.into_iter().map(|r| qualify(scope, &r)));
            collect_config(&call["module"], &child, &dependencies, symbols);
            if let Some(outputs) = call["module"]["outputs"].as_object() {
                for name in outputs.keys() {
                    symbols.insert(
                        format!("{child}.{name}"),
                        BTreeSet::from([format!("{child}.output.{name}")]),
                    );
                }
            }
        }
    }
}

fn prefix_match(reference: &str, key: &str) -> bool {
    reference == key
        || reference
            .strip_prefix(key)
            .is_some_and(|suffix| suffix.starts_with('.'))
}

impl Graph {
    pub fn parse(json: &str) -> Result<Self, String> {
        let plan: Value = serde_json::from_str(json.trim_start_matches('\u{feff}'))
            .map_err(|e| format!("invalid JSON: {e}"))?;
        let version = plan["format_version"]
            .as_str()
            .ok_or("missing format_version; expected terraform show -json output")?;
        if version.split('.').next() != Some("1") {
            return Err(format!("unsupported plan format_version: {version}"));
        }
        if !plan["resource_changes"].is_array() && !plan["planned_values"].is_object() {
            return Err("expected plan JSON containing resource_changes or planned_values".into());
        }
        let mut nodes = BTreeMap::new();
        collect_values(&plan["planned_values"]["root_module"], &mut nodes, false);
        // Include deleted resources absent from planned_values.
        for change in plan["resource_changes"].as_array().into_iter().flatten() {
            let address = change["address"]
                .as_str()
                .ok_or("resource change is missing address")?;
            nodes.insert(
                address.into(),
                Node {
                    address: address.into(),
                    module: module_of(address),
                    resource_type: change["type"].as_str().unwrap_or("resource").into(),
                    action: Action::parse(&change["change"]["actions"]),
                },
            );
        }
        let mut symbols = BTreeMap::new();
        collect_config(
            &plan["configuration"]["root_module"],
            "",
            &BTreeSet::new(),
            &mut symbols,
        );
        // Terraform can omit data sources already read during plan from both
        // planned_values and resource_changes. Recover only data sources still
        // in configuration, without reintroducing removed or managed resources.
        let mut prior_data = BTreeMap::new();
        collect_values(
            &plan["prior_state"]["values"]["root_module"],
            &mut prior_data,
            true,
        );
        for (address, node) in prior_data {
            if symbols.contains_key(&static_address(&address)) {
                nodes.entry(address).or_insert(node);
            }
        }
        let nodes: Vec<_> = nodes.into_values().collect();
        let mut instances: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        for (i, node) in nodes.iter().enumerate() {
            instances
                .entry(static_address(&node.address))
                .or_default()
                .push(i);
        }
        let mut edges = BTreeSet::new();
        for (target, node) in nodes.iter().enumerate() {
            let key = static_address(&node.address);
            let mut pending: Vec<_> = symbols.get(&key).into_iter().flatten().cloned().collect();
            let mut visited = BTreeSet::new();
            while let Some(reference) = pending.pop() {
                let reference = static_address(&reference);
                if !visited.insert(reference.clone()) {
                    continue;
                }
                if let Some((_, sources)) = instances
                    .iter()
                    .filter(|(key, _)| prefix_match(&reference, key))
                    .max_by_key(|(key, _)| key.len())
                {
                    for &source in sources {
                        if source != target {
                            edges.insert((source, target));
                        }
                    }
                } else if let Some((_, aliases)) = symbols
                    .iter()
                    .filter(|(key, _)| prefix_match(&reference, key))
                    .max_by_key(|(key, _)| key.len())
                {
                    pending.extend(aliases.iter().cloned());
                } else if reference.starts_with("module.") {
                    for (key, sources) in &instances {
                        if prefix_match(key, &reference) {
                            for &source in sources {
                                if source != target {
                                    edges.insert((source, target));
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(Self {
            nodes,
            edges: edges.into_iter().collect(),
        })
    }

    /// Condense strongly connected components, then rank the resulting DAG.
    /// Iterative DFS avoids a call-stack limit for long dependency chains.
    pub fn ranks(&self) -> Vec<usize> {
        let n = self.nodes.len();
        let mut next = vec![Vec::new(); n];
        let mut previous = vec![Vec::new(); n];
        for &(a, b) in &self.edges {
            next[a].push(b);
            previous[b].push(a);
        }
        let mut visited = vec![false; n];
        let mut order = Vec::new();
        for root in 0..n {
            if visited[root] {
                continue;
            }
            let mut stack = vec![(root, false)];
            while let Some((node, done)) = stack.pop() {
                if done {
                    order.push(node);
                    continue;
                }
                if visited[node] {
                    continue;
                }
                visited[node] = true;
                stack.push((node, true));
                for &child in next[node].iter().rev() {
                    if !visited[child] {
                        stack.push((child, false));
                    }
                }
            }
        }
        let mut component = vec![usize::MAX; n];
        let mut count = 0;
        for &root in order.iter().rev() {
            if component[root] != usize::MAX {
                continue;
            }
            let mut stack = vec![root];
            while let Some(node) = stack.pop() {
                if component[node] != usize::MAX {
                    continue;
                }
                component[node] = count;
                stack.extend(previous[node].iter().copied());
            }
            count += 1;
        }
        let mut links = vec![BTreeSet::new(); count];
        let mut degree = vec![0; count];
        for &(a, b) in &self.edges {
            let (a, b) = (component[a], component[b]);
            if a != b && links[a].insert(b) {
                degree[b] += 1;
            }
        }
        let mut ready: BTreeSet<_> = (0..count).filter(|&i| degree[i] == 0).collect();
        let mut rank = vec![0; count];
        while let Some(a) = ready.pop_first() {
            for &b in &links[a] {
                rank[b] = rank[b].max(rank[a] + 1);
                degree[b] -= 1;
                if degree[b] == 0 {
                    ready.insert(b);
                }
            }
        }
        component.into_iter().map(|c| rank[c]).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn real_terraform_plan_preserves_module_dependencies() {
        // Generated by examples/terraform/plan.sh using Terraform 1.9.8.
        let graph = Graph::parse(include_str!("../tests/fixtures/terraform-plan.json")).unwrap();
        assert_eq!(graph.nodes.len(), 16);
        assert_eq!(
            graph
                .nodes
                .iter()
                .filter(|node| node.action == Action::Create)
                .count(),
            14
        );
        let edges: BTreeSet<_> = graph
            .edges
            .iter()
            .map(|&(a, b)| {
                (
                    graph.nodes[a].address.as_str(),
                    graph.nodes[b].address.as_str(),
                )
            })
            .collect();
        assert!(edges.contains(&("terraform_data.independent", "terraform_data.explicit")));
        for node in &graph.nodes {
            if node.address.starts_with("module.") {
                // Both module expansion expressions and explicit module depends_on
                // propagate through nested modules, alongside input references.
                assert!(edges.contains(&("terraform_data.network", node.address.as_str())));
                assert!(edges.contains(&("terraform_data.ready", node.address.as_str())));
            }
            if node.address.starts_with("module.service[") {
                assert!(edges.contains(&(node.address.as_str(), "terraform_data.consumer")));
            }
        }
        assert!(edges.contains(&(
            "module.service[\"api\"].terraform_data.worker[0]",
            "module.service[\"api\"].module.nested.terraform_data.leaf",
        )));
        assert_eq!(edges.len(), 34);
    }

    #[test]
    fn real_data_sources_preserve_read_status_and_both_edge_directions() {
        let json = include_str!("../tests/fixtures/terraform-plan.json");
        let plan: Value = serde_json::from_str(json).unwrap();
        // This completed read appears only in prior_state, so it must be
        // recovered instead of silently disappearing from the graph.
        assert!(
            !plan["resource_changes"]
                .as_array()
                .unwrap()
                .iter()
                .any(|change| { change["address"] == "data.terraform_remote_state.existing" })
        );
        let graph = Graph::parse(json).unwrap();
        let index = |address: &str| {
            graph
                .nodes
                .iter()
                .position(|node| node.address == address)
                .unwrap()
        };
        let existing = index("data.terraform_remote_state.existing");
        let deferred = index("data.terraform_remote_state.after_ready");
        let ready = index("terraform_data.ready");
        let consumer = index("terraform_data.from_state");
        assert_eq!(graph.nodes[existing].action, Action::Unchanged);
        assert_eq!(graph.nodes[deferred].action, Action::Read);
        assert!(graph.edges.contains(&(ready, deferred)));
        assert!(graph.edges.contains(&(existing, consumer)));
        assert!(graph.edges.contains(&(deferred, consumer)));
        let ranks = graph.ranks();
        assert!(ranks[ready] < ranks[deferred] && ranks[deferred] < ranks[consumer]);
    }

    #[test]
    fn prior_state_recovery_keeps_only_configured_data_and_preserves_actions() {
        let graph = Graph::parse(r#"{
          "format_version":"1.2",
          "resource_changes":[{"address":"data.test.deferred","type":"test","change":{"actions":["read"]}}],
          "prior_state":{"values":{"root_module":{
            "resources":[
              {"address":"data.test.removed","type":"test","mode":"data"},
              {"address":"data.test.deferred","type":"test","mode":"data"},
              {"address":"test.managed","type":"test","mode":"managed"}
            ],
            "child_modules":[{"address":"module.child","resources":[
              {"address":"module.child.data.test.active[0]","type":"test","mode":"data"}
            ]}]
          }}},
          "configuration":{"root_module":{
            "resources":[{"address":"data.test.deferred"},{"address":"test.managed"}],
            "module_calls":{"child":{"module":{"resources":[{"address":"data.test.active"}]}}}
          }}
        }"#).unwrap();
        assert_eq!(graph.nodes.len(), 2);
        assert_eq!(graph.nodes[0].address, "data.test.deferred");
        assert_eq!(graph.nodes[0].action, Action::Read);
        assert_eq!(graph.nodes[1].address, "module.child.data.test.active[0]");
        assert_eq!(graph.nodes[1].action, Action::Unchanged);
    }

    #[test]
    fn example_covers_changes_and_module_input_edges() {
        let graph = Graph::parse(include_str!("../examples/plan.json")).unwrap();
        assert_eq!(graph.nodes.len(), 6);
        assert_eq!(graph.edges.len(), 5);
        let ami = graph
            .nodes
            .iter()
            .position(|node| node.address == "data.aws_ami.latest")
            .unwrap();
        assert_eq!(graph.nodes[ami].action, Action::Unchanged);
        for (index, node) in graph.nodes.iter().enumerate() {
            if node.resource_type == "aws_instance" {
                assert!(graph.edges.contains(&(ami, index)));
            }
        }
        let subnet = graph
            .nodes
            .iter()
            .position(|n| n.resource_type == "aws_subnet")
            .unwrap();
        let vpc = graph
            .nodes
            .iter()
            .position(|n| n.resource_type == "aws_vpc")
            .unwrap();
        assert!(graph.edges.contains(&(vpc, subnet)));
        assert_eq!(
            graph
                .nodes
                .iter()
                .filter(|n| n.action == Action::Replace)
                .count(),
            1
        );
        assert_eq!(
            graph
                .nodes
                .iter()
                .filter(|n| n.action == Action::Delete)
                .count(),
            1
        );
        assert_eq!(graph.ranks()[subnet], 1);
    }
    #[test]
    fn indexed_addresses_preserve_module_keys() {
        let address = "module.app[\"a.b]c\"].module.inner[0].aws_instance.web[\"x.y\"]";
        assert_eq!(
            static_address(address),
            "module.app.module.inner.aws_instance.web"
        );
        assert_eq!(module_of(address), "module.app[\"a.b]c\"].module.inner[0]");
    }
    #[test]
    fn cycle_is_condensed_and_dependents_follow() {
        let nodes = (0..4)
            .map(|i| Node {
                address: i.to_string(),
                resource_type: "test".into(),
                module: "root".into(),
                action: Action::Create,
            })
            .collect();
        let graph = Graph {
            nodes,
            edges: vec![(0, 1), (1, 0), (1, 2), (2, 3)],
        };
        assert_eq!(graph.ranks(), vec![0, 0, 1, 2]);
    }
    #[test]
    fn rejects_invalid_input_and_future_major_versions() {
        assert!(Graph::parse("not json").is_err());
        assert!(Graph::parse(r#"{"format_version":"2.0","resource_changes":[]}"#).is_err());
        assert!(Graph::parse(r#"{"format_version":"1.0","values":{}}"#).is_err());
        assert!(
            Graph::parse(r#"{"format_version":"1.0","resource_changes":[]}"#)
                .unwrap()
                .nodes
                .is_empty()
        );
    }
    #[test]
    fn module_output_resolves_to_resource() {
        let graph = Graph::parse(r#"{
          "format_version":"1.0",
          "resource_changes":[
            {"address":"module.network.aws_vpc.main","change":{"actions":["create"]}},
            {"address":"aws_subnet.main","change":{"actions":["create"]}}
          ],
          "configuration":{"root_module":{
            "resources":[{"address":"aws_subnet.main","expressions":{"vpc_id":{"references":["module.network.id"]}}}],
            "module_calls":{"network":{"module":{
              "resources":[{"address":"aws_vpc.main"}],
              "outputs":{"id":{"expression":{"references":["aws_vpc.main.id"]}}}
            }}}
          }}
        }"#).unwrap();
        assert_eq!(graph.edges, vec![(1, 0)]);
    }
}
