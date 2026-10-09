use super::*;
use crate::{layout::Layout, plan, semantic, svg};
use serde_json::{Value, json};

fn fixture() -> Value {
    serde_json::from_str(include_str!("../../fixtures/multi-container-plan.json")).unwrap()
}

#[test]
fn indexed_ecs_subnets_discard_terraform_traversal_prefixes() {
    for (first, second) in [("0", "1"), ("\"blue\"", "\"green\"")] {
        for blocks in [false, true] {
            let selected = format!("aws_subnet.app[{first}]");
            let other = format!("aws_subnet.app[{second}]");
            let block = json!({"subnets": {"references": [
                format!("{selected}.id"), selected, "aws_subnet.app"
            ]}});
            let raw = plan::parse(&json!({
                "format_version": "1.2",
                "resource_changes": [
                    {"address": "aws_vpc.main", "type": "aws_vpc"},
                    {"address": selected, "type": "aws_subnet"},
                    {"address": other, "type": "aws_subnet"},
                    {"address": "aws_ecs_service.app", "type": "aws_ecs_service"}
                ],
                "configuration": {"root_module": {"resources": [
                    {"address": "aws_subnet.app", "expressions": {"vpc_id": {"references": ["aws_vpc.main.id"]}}},
                    {"address": "aws_ecs_service.app", "expressions": {
                        "network_configuration": if blocks { json!([block]) } else { block }
                    }}
                ]}}
            }).to_string()).unwrap();
            let attribute = raw
                .attributes
                .iter()
                .find(|a| a.attribute == "network_configuration.subnets")
                .unwrap();
            assert!(attribute.complete);
            assert_eq!(attribute.sources.len(), 1);
            assert_eq!(raw.nodes[attribute.sources[0]].address, selected);
            let graph = semantic::transform(&raw).0;
            let ecs = index(&graph, "aws_ecs_service.app");
            assert_eq!(
                Layout::new(&graph).parents[ecs],
                Some(index(&graph, &selected))
            );
            assert!(
                !graph
                    .edges
                    .iter()
                    .any(|edge| edge.from == index(&graph, &other) && edge.to == ecs)
            );
        }
    }
}

fn index(graph: &Graph, address: &str) -> usize {
    graph
        .nodes
        .iter()
        .position(|node| node.address == address)
        .unwrap()
}

#[test]
fn alb_and_ecs_share_the_vpc_without_losing_subnet_connections() {
    let raw = plan::parse(&fixture().to_string()).unwrap();
    let original = raw.clone();
    let graph = semantic::transform(&raw).0;
    assert_eq!(raw, original);
    assert_eq!(graph.nodes, raw.nodes);
    let vpc = index(&graph, "aws_vpc.main");
    let layout = Layout::new(&graph);
    for address in ["aws_lb.app", "aws_ecs_service.app"] {
        let child = index(&graph, address);
        assert_eq!(layout.parents[child], Some(vpc));
        assert_eq!(
            graph
                .edges
                .iter()
                .filter(|edge| edge.to == child && edge.kind == EdgeKind::Containment)
                .count(),
            1
        );
        for subnet in ["aws_subnet.a", "aws_subnet.b"] {
            let source = index(&graph, subnet);
            let edge = graph
                .edges
                .iter()
                .position(|edge| {
                    edge.from == source && edge.to == child && edge.kind == EdgeKind::Connection
                })
                .unwrap();
            assert!(!layout.paths[edge].is_empty());
        }
    }
    let sg = index(&graph, "aws_security_group.app");
    assert!(
        graph
            .edges
            .iter()
            .any(|edge| edge.from == sg && edge.kind == EdgeKind::Connection)
    );
    assert_eq!(
        svg::render(&graph, &layout),
        svg::render(&graph, &Layout::new(&graph))
    );
}

#[test]
fn absent_ambiguous_or_cyclic_common_ancestry_is_not_invented() {
    for parents in [
        vec![BTreeSet::new(), BTreeSet::new()],
        vec![
            BTreeSet::from([2, 3]),
            BTreeSet::from([2]),
            BTreeSet::new(),
            BTreeSet::new(),
        ],
        vec![BTreeSet::from([1]), BTreeSet::from([0])],
    ] {
        assert_eq!(common_parent(&[0, 1], &parents), None);
    }
    let parents = vec![
        BTreeSet::from([2]),
        BTreeSet::from([2]),
        BTreeSet::from([3]),
        BTreeSet::new(),
    ];
    assert_eq!(common_parent(&[0, 1], &parents), Some(2));
    assert_eq!(common_parent(&[0], &parents), Some(0));
}

#[test]
fn disconnected_vpcs_keep_connections_and_single_subnet_uses_that_subnet() {
    for single in [false, true] {
        let mut input = fixture();
        if single {
            input["configuration"]["root_module"]["resources"][3]["expressions"]["subnets"] =
                json!({"references":["aws_subnet.a.id"]});
        } else {
            input["resource_changes"]
                .as_array_mut()
                .unwrap()
                .push(json!({"address":"aws_vpc.other","type":"aws_vpc"}));
            input["configuration"]["root_module"]["resources"][2]["expressions"]["vpc_id"] =
                json!({"references":["aws_vpc.other.id"]});
        }
        let graph = semantic::transform(&plan::parse(&input.to_string()).unwrap()).0;
        let alb = index(&graph, "aws_lb.app");
        let layout = Layout::new(&graph);
        assert_eq!(
            layout.parents[alb],
            single.then(|| index(&graph, "aws_subnet.a"))
        );
        if !single {
            assert_eq!(
                graph
                    .edges
                    .iter()
                    .filter(|edge| edge.to == alb && edge.kind == EdgeKind::Connection)
                    .count(),
                2
            );
        }
    }
}

#[test]
fn incomplete_non_subnet_and_data_references_do_not_infer_membership() {
    for scenario in 0..5 {
        let mut input = fixture();
        let expr = &mut input["configuration"]["root_module"]["resources"][3]["expressions"];
        match scenario {
            0 => expr["subnets"] = json!({"references":["aws_subnet.a.id","var.unknown"]}),
            1 => {
                expr["subnets"] =
                    json!({"references":["aws_subnet.a.id","aws_security_group.app.id"]})
            }
            2 => *expr = json!({"tags":{"references":["aws_subnet.a.id","aws_subnet.b.id"]}}),
            3 => expr["subnets"] = json!({"constant_value":["subnet-external"]}),
            _ => input["resource_changes"][3]["mode"] = json!("data"),
        }
        let graph = semantic::transform(&plan::parse(&input.to_string()).unwrap()).0;
        let alb = index(&graph, "aws_lb.app");
        assert!(
            graph
                .edges
                .iter()
                .filter(|edge| edge.to == alb)
                .all(|edge| edge.kind == EdgeKind::Dependency)
        );
    }
}

#[test]
fn ecs_subnet_path_excludes_security_groups_and_unresolved_sibling_fields() {
    for single_block in [false, true] {
        let mut input = fixture();
        let network = &mut input["configuration"]["root_module"]["resources"][4]["expressions"]["network_configuration"];
        network[0]["security_groups"] = json!({"references":["var.external_security_group"]});
        if single_block {
            *network = network[0].clone();
        }
        let raw = plan::parse(&input.to_string()).unwrap();
        let graph = semantic::transform(&raw).0;
        let service = index(&graph, "aws_ecs_service.app");
        assert_eq!(
            Layout::new(&graph).parents[service],
            Some(index(&graph, "aws_vpc.main"))
        );
    }
    let mut input = fixture();
    input["configuration"]["root_module"]["resources"][4]["expressions"]["network_configuration"]
        [0]["subnets"] = json!({"references":["aws_subnet.a.id","var.unknown"]});
    let graph = semantic::transform(&plan::parse(&input.to_string()).unwrap()).0;
    assert_eq!(
        Layout::new(&graph).parents[index(&graph, "aws_ecs_service.app")],
        None
    );
}

#[test]
fn missing_or_constant_nested_subnets_do_not_borrow_sibling_references() {
    for network in [
        json!({"references":["aws_subnet.a.id","aws_subnet.b.id"]}),
        json!([{"security_groups":{"references":["aws_subnet.a.id","aws_subnet.b.id"]}}]),
        json!([{"subnets":{"constant_value":["subnet-external"]},"security_groups":{"references":["aws_subnet.a.id"]}}]),
        json!([{"subnets":{"references":["aws_subnet.a.id"]}},{"subnets":{"constant_value":["subnet-external"]}}]),
        json!([{"subnets":{"references":["aws_subnet.a.id"]}},{}]),
    ] {
        let mut input = fixture();
        input["configuration"]["root_module"]["resources"][4]["expressions"]["network_configuration"] =
            network;
        let graph = semantic::transform(&plan::parse(&input.to_string()).unwrap()).0;
        let service = index(&graph, "aws_ecs_service.app");
        assert!(
            graph
                .edges
                .iter()
                .filter(|edge| edge.to == service)
                .all(|edge| edge.kind == EdgeKind::Dependency)
        );
        assert_eq!(Layout::new(&graph).parents[service], None);
    }
}

#[test]
fn common_data_vpc_can_contain_managed_spanning_resources() {
    let mut input = fixture();
    input["resource_changes"][0]["mode"] = json!("data");
    let input = input
        .to_string()
        .replace("aws_vpc.main", "data.aws_vpc.main");
    let graph = semantic::transform(&plan::parse(&input).unwrap()).0;
    let vpc = index(&graph, "data.aws_vpc.main");
    let layout = Layout::new(&graph);
    for resource in ["aws_lb.app", "aws_ecs_service.app"] {
        assert_eq!(layout.parents[index(&graph, resource)], Some(vpc));
    }
}
