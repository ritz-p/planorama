use super::*;
#[test]
fn real_terraform_plan_preserves_module_dependencies() {
    let graph = parse(include_str!("../../tests/fixtures/terraform-plan.json")).unwrap();
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
    let json = include_str!("../../tests/fixtures/terraform-plan.json");
    let plan: Value = serde_json::from_str(json).unwrap();
    assert!(
        !plan["resource_changes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|change| { change["address"] == "data.terraform_remote_state.existing" })
    );
    let graph = parse(json).unwrap();
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
}

#[test]
fn prior_state_recovery_keeps_only_configured_data_and_preserves_actions() {
    let graph = parse(r#"{
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
    let graph = parse(include_str!("../../examples/plan.json")).unwrap();
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
}
#[test]
fn rejects_invalid_input_and_future_major_versions() {
    assert!(parse("not json").is_err());
    assert!(parse(r#"{"format_version":"2.0","resource_changes":[]}"#).is_err());
    assert!(parse(r#"{"format_version":"1.0","values":{}}"#).is_err());
    assert!(
        parse(r#"{"format_version":"1.0","resource_changes":[]}"#)
            .unwrap()
            .nodes
            .is_empty()
    );
}
#[test]
fn module_output_resolves_to_resource() {
    let graph = parse(r#"{
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
