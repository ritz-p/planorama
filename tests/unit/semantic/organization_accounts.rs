use crate::{
    layout::Layout,
    model::{ReferenceKind, StateId, architecture::Architecture},
    plan, semantic, svg, view,
};
use serde_json::{Value, json};
use std::collections::BTreeMap;
const FIXTURE: &str = include_str!("../../fixtures/organization-deployments-plan.json");

fn architecture(input: &str, verified: bool) -> crate::model::ArchitectureGraph {
    let plan = if verified {
        crate::plan::evidence::tests::parse_fixture(input).unwrap()
    } else {
        plan::parse(input).unwrap()
    };
    semantic::transform(&plan)
}

#[test]
fn real_deployment_scope_attributes_survive_view_projection() {
    let input = json!({"format_version":"1.2", "resource_changes":[
        {"address":"test_resource.source", "type":"test_resource"},
        {"address":"test_resource.consumer", "type":"test_resource"}
    ], "configuration":{"root_module":{"resources":[
        {"address":"test_resource.source"},
        {"address":"test_resource.consumer", "expressions":{"deployment_scope":{"references":["test_resource.source.id"]}}}
    ]}}});
    let graph = architecture(&input.to_string(), false);
    let selected = view::single(&graph, &Default::default()).unwrap();
    let reference = selected
        .nodes
        .iter()
        .flat_map(|n| &n.references)
        .find(|r| r.attribute == "deployment_scope")
        .unwrap();
    assert_eq!(reference.kind, ReferenceKind::Configured);
    assert_eq!(reference.targets[0].id.address, "test_resource.source");
    assert!(svg::render(&selected, &Layout::new(&selected)).contains("deployment_scope"));
}

#[test]
fn deployment_witnesses_require_rendered_scope_groups_and_accept_data() {
    for case in 0..3 {
        let mut input: Value = serde_json::from_str(FIXTURE).unwrap();
        if case == 0 {
            input = serde_json::from_str(
                &input
                    .to_string()
                    .replace("aws_vpc.workload", "data.aws_vpc.workload"),
            )
            .unwrap();
            input["resource_changes"][5]["mode"] = json!("data");
            input["resource_changes"][5]["change"]["actions"] = json!(["read"]);
            input["configuration"]["root_module"]["resources"][5]["mode"] = json!("data");
        } else {
            input["configuration"]["provider_config"]["aws.other"] =
                input["configuration"]["provider_config"]["aws.member"].clone();
            input["configuration"]["provider_config"]["aws.other"]["alias"] = json!("other");
            input["configuration"]["provider_config"]["aws.other"]["expressions"]["assume_role"] =
                json!([{"role_arn":{"constant_value":"arn:aws:iam::999999999999:role/Fixture"}}]);
            input["resource_changes"].as_array_mut().unwrap().push(json!({"address":"aws_subnet.conflicting", "type":"aws_subnet", "mode":"managed", "provider_name":"registry.terraform.io/hashicorp/aws", "change":{"actions":["create"], "after":{"id":"subnet-test", "vpc_id":"vpc-fixture"}}}));
            input["configuration"]["root_module"]["resources"].as_array_mut().unwrap().push(json!({"address":"aws_subnet.conflicting", "mode":"managed", "type":"aws_subnet", "provider_config_key":"aws.other", "expressions":{"vpc_id":{"references":["aws_vpc.workload.id"]}}}));
            if case == 2 {
                let mut resource = input["resource_changes"][5].clone();
                resource["address"] = json!("aws_vpc.independent");
                input["resource_changes"]
                    .as_array_mut()
                    .unwrap()
                    .push(resource);
                let mut config = input["configuration"]["root_module"]["resources"][5].clone();
                config["address"] = json!("aws_vpc.independent");
                input["configuration"]["root_module"]["resources"]
                    .as_array_mut()
                    .unwrap()
                    .push(config);
            }
        }
        let graph = architecture(&input.to_string(), true);
        let selected = view::single(&graph, &Default::default()).unwrap();
        let layout = Layout::new(&selected);
        let links: Vec<_> = selected
            .nodes
            .iter()
            .flat_map(|n| &n.references)
            .filter(|r| r.kind == ReferenceKind::DeploymentScope)
            .collect();
        assert_eq!(links.len(), usize::from(case != 1), "case {case}");
        assert_eq!(
            layout.scopes.iter().filter(|p| p.account_panel).count(),
            usize::from(case != 1)
        );
        if case == 2 {
            assert_eq!(links[0].targets[0].id.address, "aws_vpc.independent");
        }
        let content = svg::render(&selected, &layout);
        let doc = roxmltree::Document::parse(&content).unwrap();
        for href in doc
            .descendants()
            .filter(|n| n.has_tag_name("a"))
            .filter_map(|n| n.attribute("href"))
            .filter_map(|href| href.strip_prefix('#'))
        {
            assert!(
                doc.descendants().any(|n| n.attribute("id") == Some(href)),
                "dangling {href}"
            );
        }
    }
}

#[test]
fn account_links_require_verified_scope_unique_current_member_and_visible_endpoint() {
    for case in 0..9 {
        let mut input: Value = serde_json::from_str(FIXTURE).unwrap();
        match case {
            2 => input["resource_changes"][3]["change"]["after"]["id"] = json!("999999999999"),
            3 => input["resource_changes"][3]["change"]["after_unknown"] = json!({"id":true}),
            4 => input["resource_changes"][4]["change"]["after"]["id"] = json!("111111111111"),
            5 => input["resource_changes"][3]["provider_name"] = json!("example.com/foreign/aws"),
            6 => input["resource_changes"][3]["change"]["actions"] = json!(["delete"]),
            7 => input["resource_changes"][3]["deposed"] = json!("old"),
            _ => {}
        }
        let architecture = architecture(&input.to_string(), case != 1);
        let graph = view::single(
            &architecture,
            &view::Options {
                focus: (case == 8).then(|| "aws_organizations_account.production".into()),
                depth: (case == 8).then_some(0),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(
            graph
                .nodes
                .iter()
                .flat_map(|node| &node.references)
                .any(|r| r.kind == ReferenceKind::DeploymentScope),
            case == 0,
            "case {case}"
        );
        let member = graph
            .nodes
            .iter()
            .find(|node| node.address == "aws_organizations_account.production")
            .unwrap();
        assert!(
            member
                .entity
                .scope
                .as_ref()
                .and_then(|scope| scope.account.as_ref())
                .is_none()
        );
        if case == 4 {
            assert!(
                member
                    .references
                    .iter()
                    .any(|r| r.label.contains("ambiguous"))
            );
        }
        if case == 1 {
            assert!(graph.nodes.iter().all(|node| {
                node.entity
                    .scope
                    .as_ref()
                    .and_then(|scope| scope.account.as_ref())
                    .is_none()
            }));
        }
    }
}

#[test]
fn verified_deployment_example() {
    let architecture = architecture(FIXTURE, true);
    let graph = view::single(&architecture, &Default::default()).unwrap();
    let layout = Layout::new(&graph);
    let svg = svg::render(&graph, &layout);
    let html = crate::html::render(&graph, &layout, svg::AddressFormat::Qualified);
    assert_eq!(graph.coverage.selected.resources, 6);
    assert!(svg.contains("Verified AWS Account deployment scope"));
    assert!(svg.contains("data-deployment-member"));
    for content in [&svg, &html] {
        assert!(!content.contains("PRIVATE_"));
    }
    let document = roxmltree::Document::parse(&svg).unwrap();
    for node in document.descendants().filter(|node| node.has_tag_name("a")) {
        if let Some(href) = node
            .attribute("href")
            .and_then(|href| href.strip_prefix('#'))
        {
            assert!(
                document
                    .descendants()
                    .any(|target| target.attribute("id") == Some(href)),
                "dangling {href}"
            );
        }
    }
    if std::env::var_os("UPDATE_ORGANIZATION_DEPLOYMENTS").is_some() {
        std::fs::write("examples/organization-deployments.svg", &svg).unwrap();
        std::fs::write("examples/organization-deployments.html", &html).unwrap();
    } else {
        assert_eq!(
            std::fs::read_to_string("examples/organization-deployments.svg").unwrap(),
            svg
        );
    }
}

#[test]
fn deployment_links_keep_named_states_without_remote_state_edges() {
    let input: Value = serde_json::from_str(FIXTURE).unwrap();
    let mut org = input.clone();
    org["resource_changes"].as_array_mut().unwrap().pop();
    org["configuration"]["root_module"]["resources"]
        .as_array_mut()
        .unwrap()
        .pop();
    let mut infra = input.clone();
    infra["resource_changes"] = json!([input["resource_changes"][5]]);
    infra["configuration"]["root_module"]["resources"] =
        json!([input["configuration"]["root_module"]["resources"][5]]);
    let states = Architecture {
        states: BTreeMap::from([
            (
                StateId::new("org").unwrap(),
                architecture(&org.to_string(), true),
            ),
            (
                StateId::new("infra").unwrap(),
                architecture(&infra.to_string(), true),
            ),
            (
                StateId::new("other").unwrap(),
                architecture(&infra.to_string(), true),
            ),
        ]),
        relationships: Vec::new(),
    };
    let view = view::apply(&states, &Default::default()).unwrap();
    assert!(view.relationships().is_empty());
    let references: Vec<_> = view.states[&StateId::new("org").unwrap()]
        .nodes
        .iter()
        .flat_map(|node| &node.references)
        .filter(|r| r.kind == ReferenceKind::DeploymentScope)
        .collect();
    assert_eq!(references.len(), 1);
    assert_eq!(references[0].targets.len(), 2);
    assert!(
        references[0]
            .targets
            .iter()
            .all(|target| target.state.is_some())
    );
    let svg = svg::render_states(&view, svg::AddressFormat::Qualified);
    let doc = roxmltree::Document::parse(&svg).unwrap();
    for link in doc.descendants().filter(|node| node.has_tag_name("a")) {
        if let Some(id) = link
            .attribute("href")
            .and_then(|href| href.strip_prefix('#'))
        {
            assert!(
                doc.descendants()
                    .any(|node| node.attribute("id") == Some(id)),
                "dangling {id}"
            );
        }
    }
}
