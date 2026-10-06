use super::*;

#[path = "plan/locals.rs"]
mod locals;

#[path = "plan/instances.rs"]
mod instances;

#[test]
fn parsed_edges_are_ordered_dependencies() {
    let graph = parse(include_str!("../../tests/fixtures/terraform-plan.json")).unwrap();
    assert_eq!(graph.edges.len(), 28);
    assert!(
        graph
            .edges
            .iter()
            .all(|edge| edge.kind == crate::model::EdgeKind::Dependency)
    );
    assert!(graph.edges.windows(2).all(|pair| pair[0] < pair[1]));
    assert_eq!(
        graph.edges,
        parse(include_str!("../../tests/fixtures/terraform-plan.json"))
            .unwrap()
            .edges
    );
}

#[test]
fn classification_uses_types_independently_of_entity_mode_and_input_source() {
    use crate::model::ResourceRole;
    let graph = parse(r#"{
        "format_version":"1.2",
        "planned_values":{"root_module":{"resources":[
            {"address":"aws_vpc.main","type":"aws_vpc","mode":"managed"},
            {"address":"data.aws_vpc.shared","type":"aws_vpc","mode":"data"},
            {"address":"custom_unknown.item","type":"custom_unknown"}
        ]}},
        "resource_changes":[{"address":"aws_instance.app","type":"aws_instance","change":{"actions":["create"]}}],
        "prior_state":{"values":{"root_module":{"resources":[
            {"address":"data.aws_security_group.shared","type":"aws_security_group","mode":"data"}
        ]}}},
        "configuration":{"root_module":{"resources":[{"address":"data.aws_security_group.shared"}]}}
    }"#).unwrap();
    assert_eq!(graph.nodes.len(), 5);
    for node in &graph.nodes {
        let expected = match node.address.as_str() {
            "aws_vpc.main" => (EntityMode::Managed, ResourceRole::Container),
            "data.aws_vpc.shared" => (EntityMode::Data, ResourceRole::Container),
            "aws_instance.app" => (EntityMode::Managed, ResourceRole::Node),
            "data.aws_security_group.shared" => (EntityMode::Data, ResourceRole::Policy),
            _ => (EntityMode::Managed, ResourceRole::Unknown),
        };
        assert_eq!((node.mode, node.role), expected);
    }
    assert!(
        crate::svg::render(&graph, &crate::layout::Layout::new(&graph))
            .contains("custom_unknown.item")
    );
}

#[test]
fn modes_are_preserved_from_values_changes_and_recovered_prior_state() {
    let graph = parse(r#"{
        "format_version":"1.2",
        "planned_values":{"root_module":{"resources":[
            {"address":"aws_vpc.main","type":"aws_vpc","mode":"managed"},
            {"address":"data.aws_vpc.shared","type":"aws_vpc","mode":"data"}
        ]}},
        "resource_changes":[{"address":"data.aws_ami.latest","type":"aws_ami","mode":"data","change":{"actions":["read"]}}],
        "prior_state":{"values":{"root_module":{"resources":[
            {"address":"data.custom_item.prior","type":"custom_item","mode":"data"}
        ]}}},
        "configuration":{"root_module":{"resources":[{"address":"data.custom_item.prior"}]}}
    }"#).unwrap();
    assert_eq!(graph.nodes.len(), 4);
    for node in &graph.nodes {
        let expected = match node.address.as_str() {
            "aws_vpc.main" => EntityMode::Managed,
            _ => EntityMode::Data,
        };
        assert_eq!(node.mode, expected);
    }
    let prior = graph
        .nodes
        .iter()
        .find(|node| node.address == "data.custom_item.prior")
        .unwrap();
    assert_eq!(prior.role, crate::model::ResourceRole::Unknown);
    let svg = crate::svg::render(&graph, &crate::layout::Layout::new(&graph));
    assert!(svg.contains("data.custom_item.prior"));
}
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
        .map(|edge| {
            let (a, b) = edge.endpoints();
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
    assert_eq!(edges.len(), 28);
    // Previously these references expanded to all four workers for both leaves.
    for scope in ["api", "web"] {
        let leaf = format!("module.service[\"{scope}\"].module.nested.terraform_data.leaf");
        let workers: Vec<_> = edges
            .iter()
            .filter(|(source, target)| *target == leaf && source.contains("terraform_data.worker"))
            .map(|(source, _)| *source)
            .collect();
        assert_eq!(
            workers,
            [format!(
                "module.service[\"{scope}\"].terraform_data.worker[0]"
            )]
        );
    }
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
    assert!(
        graph
            .edges
            .contains(&crate::model::Edge::from((ready, deferred)))
    );
    assert!(
        graph
            .edges
            .contains(&crate::model::Edge::from((existing, consumer)))
    );
    assert!(
        graph
            .edges
            .contains(&crate::model::Edge::from((deferred, consumer)))
    );
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
            assert!(
                graph
                    .edges
                    .contains(&crate::model::Edge::from((ami, index)))
            );
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
    assert!(
        graph
            .edges
            .contains(&crate::model::Edge::from((vpc, subnet)))
    );
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
    assert_eq!(graph.edges, vec![crate::model::Edge::from((1, 0))]);
}

#[test]
fn provider_identity_prefers_explicit_metadata_and_configuration_bindings() {
    use crate::model::ProviderIdentity;
    let raw = parse(r#"{
        "format_version":"1.2",
        "planned_values":{"root_module":{"resources":[{"address":"aws_vpc.prior","type":"aws_vpc","provider_name":"example.com/acme/custom"}]}},
        "resource_changes":[
            {"address":"aws_vpc.explicit","type":"aws_vpc","provider_name":"hashicorp/aws"},
            {"address":"aws_vpc.foreign","type":"aws_vpc","provider_name":"registry.terraform.io/acme/custom"},
            {"address":"aws_vpc.fallback","type":"aws_vpc"},
            {"address":"aws_vpc.prior","type":"aws_vpc"},
            {"address":"module.net[0].aws_vpc.bound","type":"aws_vpc"},
            {"address":"other_item.unknown","type":"other_item"}
        ],
        "configuration":{
            "provider_config":{"aws.other":{"full_name":"registry.terraform.io/hashicorp/google"}},
            "root_module":{"module_calls":{"net":{"module":{"resources":[{"address":"aws_vpc.bound","provider_config_key":"aws.other"}]}}}}
        }
    }"#).unwrap();
    let node = |address: &str| {
        raw.nodes
            .iter()
            .find(|node| node.address == address)
            .unwrap()
    };
    assert_eq!(
        node("aws_vpc.explicit").provider.source(),
        Some("registry.terraform.io/hashicorp/aws")
    );
    assert!(node("aws_vpc.explicit").provider.is_explicit());
    assert_eq!(
        node("aws_vpc.fallback").provider,
        ProviderIdentity::InferredAws
    );
    assert_eq!(
        node("other_item.unknown").provider,
        ProviderIdentity::Unknown
    );
    for address in [
        "aws_vpc.foreign",
        "aws_vpc.prior",
        "module.net[0].aws_vpc.bound",
    ] {
        assert_eq!(node(address).role, crate::model::ResourceRole::Unknown);
        assert!(!node(address).provider.is_aws());
    }
    assert_eq!(
        node("module.net[0].aws_vpc.bound").provider.source(),
        Some("registry.terraform.io/hashicorp/google")
    );
}

#[test]
fn captured_aws_entities_keep_explicit_provider_identity() {
    let raw = parse(include_str!("../fixtures/aws-captured/plan.json")).unwrap();
    assert_eq!(raw.nodes.len(), 8);
    assert!(
        raw.nodes
            .iter()
            .all(|node| node.provider.is_explicit() && node.provider.is_aws())
    );
}

#[test]
fn foreign_aws_lookalikes_do_not_gain_aws_semantics_or_icons() {
    let raw = parse(r#"{"format_version":"1.2","resource_changes":[
        {"address":"data.aws_region.custom","mode":"data","type":"aws_region","provider_name":"acme/custom"},
        {"address":"aws_vpc.custom","type":"aws_vpc","provider_name":"acme/custom"},
        {"address":"aws_subnet.custom","type":"aws_subnet","provider_name":"acme/custom"}
    ],"configuration":{"root_module":{"resources":[{"address":"aws_subnet.custom","expressions":{"vpc_id":{"references":["aws_vpc.custom.id"]}}}]}}}"#).unwrap();
    let graph = crate::semantic::transform(&raw);
    assert_eq!(graph.nodes.len(), 3);
    assert!(
        graph
            .edges
            .iter()
            .all(|edge| edge.kind == crate::model::EdgeKind::Dependency)
    );
    let svg = crate::svg::render(&graph, &crate::layout::Layout::new(&graph));
    assert!(!svg.contains("planorama-icon-aws"));
}
