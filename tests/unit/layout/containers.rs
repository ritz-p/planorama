use super::*;
use crate::{
    model::{Action, Edge, EntityMode, Node},
    plan, semantic, svg,
};

fn fixture() -> Graph {
    let raw = plan::parse(include_str!("../../fixtures/containment-plan.json")).unwrap();
    semantic::transform(&raw).0
}

#[test]
fn relationship_names_distinguish_same_named_subnets_after_reparsing() {
    use serde_json::json;
    use std::collections::BTreeMap;
    for nested in [false, true] {
        let mut input = json!({
            "format_version":"1.2",
            "resource_changes":[
                {"address":"aws_vpc.main","type":"aws_vpc","change":{"actions":["create"]}},
                {"address":"aws_route_table.main","type":"aws_route_table","change":{"actions":["create"]}}
            ],
            "configuration":{"root_module":{
                "resources":[{"address":"aws_vpc.main"},{"address":"aws_route_table.main"}],
                "module_calls":{}
            }}
        });
        for (module, association) in [("a", "alpha"), ("b", "beta")] {
            for (kind, name) in [
                ("aws_subnet", "main"),
                ("aws_route_table_association", association),
            ] {
                input["resource_changes"]
                    .as_array_mut()
                    .unwrap()
                    .push(json!({
                        "address":format!("module.{module}.{kind}.{name}"),
                        "type":kind,"change":{"actions":["create"]}
                    }));
            }
            let subnet_expressions = match nested {
                true => json!({"vpc_id":{"references":["var.vpc"]}}),
                false => json!({}),
            };
            input["configuration"]["root_module"]["module_calls"][module] = json!({
                "expressions":{
                    "table":{"references":["aws_route_table.main.id"]},
                    "vpc":{"references":["aws_vpc.main.id"]}
                },
                "module":{"resources":[
                    {"address":"aws_subnet.main","expressions":subnet_expressions},
                    {"address":format!("aws_route_table_association.{association}"),"expressions":{
                        "subnet_id":{"references":["aws_subnet.main.id"]},
                        "route_table_id":{"references":["var.table"]}
                    }}
                ]}
            });
        }
        let original = semantic::transform(&plan::parse(&input.to_string()).unwrap()).0;
        let calls = input["configuration"]["root_module"]["module_calls"]
            .as_object_mut()
            .unwrap();
        let call = calls.remove("a").unwrap();
        calls.insert("z".into(), call);
        let renamed = semantic::transform(
            &plan::parse(&input.to_string().replace("module.a.", "module.z.")).unwrap(),
        )
        .0;
        let identity = |node: &Node| node.address.replace("module.z.", "module.a.");
        assert_ne!(
            original.nodes.iter().map(identity).collect::<Vec<_>>(),
            renamed.nodes.iter().map(identity).collect::<Vec<_>>()
        );
        assert_eq!(
            original
                .edges
                .iter()
                .filter(|edge| edge.kind == EdgeKind::Association)
                .count(),
            2
        );
        let snapshot = |graph: &Graph| {
            let layout = Layout::new(graph);
            assert_eq!(
                layout.parents.iter().flatten().count(),
                if nested { 2 } else { 0 }
            );
            let positions: BTreeMap<_, _> = graph
                .nodes
                .iter()
                .enumerate()
                .map(|(index, node)| (identity(node), layout.positions[index]))
                .collect();
            let paths: BTreeMap<_, _> = graph
                .edges
                .iter()
                .zip(layout.paths)
                .map(|(edge, path)| {
                    (
                        (
                            identity(&graph.nodes[edge.from]),
                            identity(&graph.nodes[edge.to]),
                            edge.kind,
                            edge.change
                                .as_ref()
                                .map(|change| change.local_address().to_owned()),
                        ),
                        path,
                    )
                })
                .collect();
            (positions, paths)
        };
        assert_eq!(snapshot(&original), snapshot(&renamed));
        verify(&original);
        verify(&renamed);
    }
}

#[test]
fn reparsed_parallel_associations_keep_ports_when_modules_are_renamed() {
    use serde_json::json;
    use std::collections::BTreeMap;
    for (first, second) in [("alpha", "beta"), ("main[0]", "main[1]")] {
        let mut input = json!({
            "format_version":"1.2",
            "resource_changes":[
                {"address":"aws_subnet.private","type":"aws_subnet","change":{"actions":["create"]}},
                {"address":"aws_route_table.private","type":"aws_route_table","change":{"actions":["create"]}}
            ],
            "configuration":{"root_module":{
                "resources":[{"address":"aws_subnet.private"},{"address":"aws_route_table.private"}],
                "module_calls":{}
            }}
        });
        for (module, name) in [("a", first), ("b", second)] {
            input["resource_changes"]
                .as_array_mut()
                .unwrap()
                .push(json!({
                    "address":format!("module.{module}.aws_route_table_association.{name}"),
                    "type":"aws_route_table_association","change":{"actions":["create"]}
                }));
            input["configuration"]["root_module"]["module_calls"][module] = json!({
                "expressions":{
                    "subnet":{"references":["aws_subnet.private.id"]},
                    "table":{"references":["aws_route_table.private.id"]}
                },
                "module":{"resources":[{
                    "address":format!("aws_route_table_association.{}", name.split('[').next().unwrap()),
                    "expressions":{
                        "subnet_id":{"references":["var.subnet"]},
                        "route_table_id":{"references":["var.table"]}
                    }
                }]}
            });
        }
        let original = semantic::transform(&plan::parse(&input.to_string()).unwrap()).0;
        let calls = input["configuration"]["root_module"]["module_calls"]
            .as_object_mut()
            .unwrap();
        let call = calls.remove("a").unwrap();
        calls.insert("z".into(), call);
        let renamed = semantic::transform(
            &plan::parse(&input.to_string().replace("module.a.", "module.z.")).unwrap(),
        )
        .0;
        let identity = |edge: &Edge| {
            edge.change
                .as_ref()
                .unwrap()
                .address
                .replace("module.z.", "module.a.")
        };
        assert_eq!(original.edges.len(), 2);
        assert!(
            original
                .edges
                .iter()
                .all(|edge| edge.kind == EdgeKind::Association)
        );
        assert_ne!(
            original.edges.iter().map(identity).collect::<Vec<_>>(),
            renamed.edges.iter().map(identity).collect::<Vec<_>>()
        );
        let snapshot = |graph: &Graph| {
            let layout = Layout::new(graph);
            let paths: BTreeMap<_, _> =
                graph.edges.iter().map(identity).zip(layout.paths).collect();
            (layout.positions, paths)
        };
        assert_eq!(snapshot(&original), snapshot(&renamed));
        verify(&original);
        verify(&renamed);
    }
}

#[test]
fn reparsed_module_renames_preserve_fan_in_and_fan_out_ports_and_paths() {
    use serde_json::json;
    use std::collections::BTreeMap;
    let mut input = json!({
        "format_version":"1.2",
        "resource_changes":[
            {"address":"aws_vpc.main","type":"aws_vpc","change":{"actions":["create"]}},
            {"address":"aws_s3_bucket.source","type":"aws_s3_bucket","change":{"actions":["create"]}},
            {"address":"aws_s3_bucket.sink","type":"aws_s3_bucket","change":{"actions":["create"]}}
        ],
        "configuration":{"root_module":{
            "resources":[
                {"address":"aws_vpc.main"},
                {"address":"aws_s3_bucket.source"},
                {"address":"aws_s3_bucket.sink","expressions":{"tags":{"references":["module.a.id","module.b.id"]}}}
            ],
            "module_calls":{}
        }}
    });
    for (module, name) in [("a", "alpha"), ("b", "beta")] {
        input["resource_changes"]
            .as_array_mut()
            .unwrap()
            .push(json!({
                "address":format!("module.{module}.aws_instance.{name}"),
                "type":"aws_instance","change":{"actions":["create"]}
            }));
        input["configuration"]["root_module"]["module_calls"][module] = json!({
            "expressions":{"source":{"references":["aws_s3_bucket.source.id"]}},
            "module":{
                "resources":[{"address":format!("aws_instance.{name}"),"expressions":{"tags":{"references":["var.source"]}}}],
                "outputs":{"id":{"expression":{"references":[format!("aws_instance.{name}.id")]}}}
            }
        });
    }
    let original = semantic::transform(&plan::parse(&input.to_string()).unwrap()).0;
    let calls = input["configuration"]["root_module"]["module_calls"]
        .as_object_mut()
        .unwrap();
    let renamed_call = calls.remove("a").unwrap();
    calls.insert("z".into(), renamed_call);
    let renamed = semantic::transform(
        &plan::parse(&input.to_string().replace("module.a.", "module.z.")).unwrap(),
    )
    .0;
    let identity = |node: &Node| node.address.replace("module.z.", "module.a.");
    let edge_ids = |graph: &Graph| {
        graph
            .edges
            .iter()
            .map(|edge| {
                (
                    identity(&graph.nodes[edge.from]),
                    identity(&graph.nodes[edge.to]),
                    edge.kind,
                )
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(original.edges.len(), 4);
    assert_ne!(edge_ids(&original), edge_ids(&renamed));
    let snapshot = |graph: &Graph| {
        let layout = Layout::new(graph);
        let positions: BTreeMap<_, _> = graph
            .nodes
            .iter()
            .zip(&layout.positions)
            .map(|(node, point)| (identity(node), *point))
            .collect();
        let paths: BTreeMap<_, _> = edge_ids(graph).into_iter().zip(layout.paths).collect();
        (positions, paths)
    };
    assert_eq!(snapshot(&original), snapshot(&renamed));
    verify(&original);
    verify(&renamed);
}

#[test]
fn large_aws_example_keeps_cross_module_containment_and_multi_subnet_relationships() {
    let raw = plan::parse(include_str!("../../../examples/terraform-large/plan.json")).unwrap();
    assert!(raw.nodes.len() >= 45);
    let ranks = crate::layout::rank::compute(&raw.graph);
    assert!(ranks.iter().copied().max().unwrap() >= 4);
    assert!(
        raw.edges
            .iter()
            .any(|edge| ranks[edge.to] > ranks[edge.from] + 1)
    );
    let workers: Vec<_> = raw
        .nodes
        .iter()
        .enumerate()
        .filter(|(_, node)| node.resource_type == "aws_instance")
        .map(|(index, _)| ranks[index])
        .collect();
    assert_eq!(workers.len(), 4);
    assert!(workers.iter().all(|rank| *rank == workers[0]));
    let original = raw.clone();
    let graph = semantic::transform(&raw).0;
    assert_eq!(raw, original);
    assert!(graph.edges.len() >= 70);
    let layout = Layout::new(&graph);
    assert!(layout.bands.is_empty());
    for zone in ["a", "b"] {
        let subnet = graph
            .nodes
            .iter()
            .position(|node| node.address == format!("module.network.aws_subnet.private_{zone}"))
            .unwrap();
        for index in 0..2 {
            let worker = graph
                .nodes
                .iter()
                .position(|node| {
                    node.address
                        == format!("module.application.aws_instance.workers_{zone}[{index}]")
                })
                .unwrap();
            assert_eq!(layout.parents[worker], Some(subnet));
            assert_ne!(graph.nodes[worker].module, graph.nodes[subnet].module);
        }
    }
    for resource_type in [
        "aws_lb",
        "aws_lb_target_group",
        "aws_ecs_service",
        "aws_db_instance",
    ] {
        let node = graph
            .nodes
            .iter()
            .position(|node| node.resource_type == resource_type)
            .unwrap();
        assert!(layout.parents[node].is_none());
    }
    let service = graph
        .nodes
        .iter()
        .position(|node| node.resource_type == "aws_ecs_service")
        .unwrap();
    assert_eq!(
        graph
            .edges
            .iter()
            .filter(
                |edge| edge.to == service && graph.nodes[edge.from].resource_type == "aws_subnet"
            )
            .count(),
        2
    );
    assert_eq!(
        graph
            .edges
            .iter()
            .filter(|edge| edge.kind == EdgeKind::Association)
            .count(),
        4
    );
    for role in [
        ResourceRole::Container,
        ResourceRole::Node,
        ResourceRole::Connector,
        ResourceRole::Policy,
        ResourceRole::Controller,
    ] {
        assert!(graph.nodes.iter().any(|node| node.role == role));
    }
    verify(&graph);
}

#[test]
fn duplicate_local_names_use_topology_after_module_renames_and_node_reordering() {
    for nested in [false, true] {
        let mut graph = fixture();
        let subnet = graph
            .nodes
            .iter()
            .position(|node| node.resource_type == "aws_subnet")
            .unwrap();
        let template = graph
            .nodes
            .iter()
            .find(|node| node.resource_type == "aws_instance")
            .unwrap()
            .clone();
        let first = graph.nodes.len();
        for (module, target) in [("a", "a"), ("b", "b")] {
            let worker = graph.nodes.len();
            graph.nodes.push(Node {
                address: format!("module.{module}.aws_instance.main"),
                module: format!("module.{module}"),
                ..template.clone()
            });
            graph.nodes.push(Node {
                address: format!("aws_s3_bucket.{target}"),
                resource_type: "aws_s3_bucket".into(),
                module: "root".into(),
                ..template.clone()
            });
            graph.edges.push(Edge::from((worker, worker + 1)));
            if nested {
                graph.edges.push(Edge {
                    kind: EdgeKind::Containment,
                    ..Edge::from((subnet, worker))
                });
            }
        }
        let before = Layout::new(&graph);
        let mut renamed = graph.clone();
        for (index, module) in [(first, "z"), (first + 2, "a")] {
            renamed.nodes[index].module = format!("module.{module}");
            renamed.nodes[index].address = format!("module.{module}.aws_instance.main");
        }
        let count = renamed.nodes.len();
        renamed.nodes.reverse();
        for edge in &mut renamed.edges {
            edge.from = count - 1 - edge.from;
            edge.to = count - 1 - edge.to;
        }
        let after = Layout::new(&renamed);
        for node in 0..count {
            assert_eq!(before.positions[node], after.positions[count - 1 - node]);
        }
        assert_eq!(before.paths, after.paths);
        assert_eq!(
            ordering::structural_keys(&graph),
            ordering::structural_keys(&renamed)
                .into_iter()
                .rev()
                .collect::<Vec<_>>()
        );
    }
}

#[test]
fn architecture_parentage_and_geometry_do_not_depend_on_module_membership() {
    let raw = plan::parse(include_str!(
        "../../fixtures/cross-module-containment-plan.json"
    ))
    .unwrap();
    let graph = semantic::transform(&raw).0;
    let layout = Layout::new(&graph);
    let subnet = graph
        .nodes
        .iter()
        .position(|node| node.resource_type == "aws_subnet")
        .unwrap();
    let workload = graph
        .nodes
        .iter()
        .position(|node| node.resource_type == "aws_instance")
        .unwrap();
    assert_eq!(layout.parents[workload], Some(subnet));
    assert!(layout.bands.is_empty());
    let output = svg::render(&graph, &layout);
    assert!(output.contains("module.network"));
    assert!(output.contains("module.application"));
    let mut relocated = graph.clone();
    for node in &mut relocated.nodes {
        node.address = node.resource_address().local().to_owned();
        node.module = "root".into();
    }
    let other = Layout::new(&relocated);
    assert_eq!(layout.positions, other.positions);
    assert_eq!(layout.parents, other.parents);
    assert_eq!(layout.containers, other.containers);
    assert_eq!(layout.paths, other.paths);
    verify(&graph);
}

#[test]
fn high_degree_ports_expand_headers_and_remain_distinct_inside_parent_bounds() {
    for role in [ResourceRole::Container, ResourceRole::Node] {
        for count in [56, 57, 100] {
            let mut graph = fixture();
            for (index, node) in graph.nodes.iter_mut().enumerate() {
                node.role = match index {
                    0 => ResourceRole::Container,
                    _ => role,
                };
            }
            graph.edges = [1, 2]
                .map(|child| Edge {
                    kind: EdgeKind::Containment,
                    ..Edge::from((0, child))
                })
                .into();
            graph.edges.extend((0..count).map(|index| Edge {
                kind: EdgeKind::Association,
                change: Some(crate::model::EdgeChange {
                    address: format!("test.relationship{index}"),
                    action: Action::Create,
                }),
                ..Edge::from((1, 2))
            }));
            let layout = Layout::new(&graph);
            for node in [1, 2] {
                assert!(layout.header_heights[node] >= count + 41);
            }
            let paths = &layout.paths[2..];
            let sources: BTreeSet<_> = paths.iter().map(|path| path[0].y).collect();
            let targets: BTreeSet<_> = paths.iter().map(|path| path.last().unwrap().y).collect();
            assert_eq!(sources.len(), count);
            assert_eq!(targets.len(), count);
            let output = svg::render(&graph, &layout);
            if role == ResourceRole::Node {
                assert!(output.contains(&format!(
                    "width=\"320\" height=\"{}\"",
                    layout.header_heights[1]
                )));
            }
            verify(&graph);
            for index in (2..graph.edges.len()).step_by(2) {
                let edge = &mut graph.edges[index];
                std::mem::swap(&mut edge.from, &mut edge.to);
            }
            let layout = Layout::new(&graph);
            let mut ports = [BTreeSet::new(), BTreeSet::new()];
            for (edge, path) in graph.edges[2..].iter().zip(&layout.paths[2..]) {
                assert!(ports[edge.from - 1].insert(path[0].y));
                assert!(ports[edge.to - 1].insert(path.last().unwrap().y));
            }
            verify(&graph);
        }
    }
}

#[test]
fn reciprocal_relationships_and_self_loops_use_distinct_incoming_and_outgoing_ports() {
    for role in [ResourceRole::Container, ResourceRole::Node] {
        let mut node = fixture()
            .nodes
            .into_iter()
            .find(|node| node.role == ResourceRole::Container)
            .unwrap();
        let mut nodes = Vec::new();
        for index in 0..3 {
            node.address = format!("test.peer{index}");
            node.role = match index {
                2 => ResourceRole::Container,
                _ => role,
            };
            nodes.push(node.clone());
        }
        let edges = [
            (0, 1, Action::Create),
            (1, 0, Action::Update),
            (0, 0, Action::Delete),
        ]
        .into_iter()
        .enumerate()
        .map(|(index, (from, to, action))| Edge {
            from,
            to,
            kind: EdgeKind::Association,
            change: Some(crate::model::EdgeChange {
                address: format!("test.relationship{index}"),
                action,
            }),
        })
        .collect();
        let graph = Graph { nodes, edges };
        let layout = Layout::new(&graph);
        assert_ne!(layout.paths[0].first(), layout.paths[1].last());
        assert_ne!(layout.paths[0].last(), layout.paths[1].first());
        let reversed: Vec<_> = layout.paths[1].iter().rev().copied().collect();
        assert_ne!(layout.paths[0], reversed);
        assert_ne!(layout.paths[2].first(), layout.paths[2].last());
        let output = svg::render(&graph, &layout);
        for (index, action) in ["create", "update", "delete"].iter().enumerate() {
            assert!(output.contains(&format!("association; {action}: test.relationship{index}")));
            assert!(output.contains(&format!("marker-end=\"url(#arrow-{action})\"")));
        }
        verify(&graph);
    }
}

#[test]
fn nesting_hides_ancestor_references_but_keeps_sibling_and_external_connections() {
    let mut graph = fixture();
    let parent = graph
        .nodes
        .iter()
        .position(|node| node.resource_type == "aws_subnet")
        .unwrap();
    let child = graph
        .nodes
        .iter()
        .position(|node| node.resource_type == "aws_instance")
        .unwrap();
    let root = graph
        .nodes
        .iter()
        .position(|node| node.resource_type == "aws_vpc")
        .unwrap();
    let sibling = graph.nodes.len();
    let mut node = graph.nodes[child].clone();
    node.address = "aws_instance.sibling".into();
    graph.nodes.push(node.clone());
    node.address = "aws_instance.external".into();
    graph.nodes.push(node);
    graph.edges.extend([
        Edge {
            kind: EdgeKind::Containment,
            ..Edge::from((parent, sibling))
        },
        Edge::from((child, root)),
        Edge::from((child, sibling)),
        Edge::from((child, sibling + 1)),
    ]);
    let original = graph.clone();
    let layout = Layout::new(&graph);
    let paths = &layout.paths[layout.paths.len() - 4..];
    assert!(paths[0].is_empty());
    assert!(paths[1].is_empty());
    assert!(!paths[2].is_empty());
    assert!(!paths[3].is_empty());
    assert_eq!(graph, original);
    verify(&graph);
}

#[test]
fn parallel_association_changes_have_distinct_ports_and_paths() {
    let raw = plan::parse(include_str!("../../fixtures/association-plan.json")).unwrap();
    let mut graph = semantic::transform(&raw).0;
    let mut other = graph.edges[0].clone();
    let change = other.change.as_mut().unwrap();
    change.address = "aws_route_table_association.other".into();
    change.action = Action::Update;
    graph.edges.push(other);
    let layout = Layout::new(&graph);
    assert_ne!(layout.paths[0], layout.paths[1]);
    assert_ne!(layout.paths[0].first(), layout.paths[1].first());
    assert_ne!(layout.paths[0].last(), layout.paths[1].last());
    let output = svg::render(&graph, &layout);
    for expected in [
        "association; create: aws_route_table_association.private",
        "association; update: aws_route_table_association.other",
        "marker-end=\"url(#arrow-create)\"",
        "marker-end=\"url(#arrow-update)\"",
    ] {
        assert!(output.contains(expected));
    }
    verify(&graph);
}

fn overlaps(a: Bounds, b: Bounds) -> bool {
    a.origin.x < b.origin.x + b.width
        && b.origin.x < a.origin.x + a.width
        && a.origin.y < b.origin.y + b.height
        && b.origin.y < a.origin.y + a.height
}

fn verify(graph: &Graph) {
    let layout = Layout::new(graph);
    for (edge, path) in graph
        .edges
        .iter()
        .zip(&layout.paths)
        .filter(|(_, path)| !path.is_empty())
    {
        for (node, point) in [
            (edge.from, path.first().unwrap()),
            (edge.to, path.last().unwrap()),
        ] {
            let bounds = layout
                .containers
                .iter()
                .find(|(index, _)| *index == node)
                .map(|(_, bounds)| *bounds)
                .unwrap_or(Bounds {
                    origin: layout.positions[node],
                    width: NODE_WIDTH,
                    height: layout.header_heights[node],
                });
            assert_eq!(point.x, bounds.origin.x + bounds.width);
            assert!(point.y > bounds.origin.y && point.y < bounds.origin.y + bounds.height);
        }
    }
    for path in layout.paths.iter().filter(|path| !path.is_empty()) {
        assert_eq!(path[0].y, path[1].y);
        assert!(path[0].x < path[1].x);
        let last = path.len() - 1;
        assert_eq!(path[last].y, path[last - 1].y);
        assert!(path[last].x < path[last - 1].x);
    }
    for &(parent, bounds) in &layout.containers {
        assert!(bounds.origin.x + bounds.width <= layout.width);
        assert!(bounds.origin.y + bounds.height <= layout.height);
        for (child, parent_index) in layout.parents.iter().enumerate() {
            if *parent_index != Some(parent) {
                continue;
            }
            let child_bounds = layout
                .containers
                .iter()
                .find(|(node, _)| *node == child)
                .map(|(_, bounds)| *bounds)
                .unwrap_or(Bounds {
                    origin: layout.positions[child],
                    width: NODE_WIDTH,
                    height: layout.header_heights[child],
                });
            assert!(child_bounds.origin.x >= bounds.origin.x + PADDING);
            assert!(
                child_bounds.origin.y >= bounds.origin.y + layout.header_heights[parent] + PADDING
            );
            assert!(
                child_bounds.origin.x + child_bounds.width + PADDING
                    <= bounds.origin.x + bounds.width
            );
            assert!(
                child_bounds.origin.y + child_bounds.height + PADDING
                    <= bounds.origin.y + bounds.height
            );
        }
    }
    for (i, &origin) in layout.positions.iter().enumerate() {
        let a = Bounds {
            origin,
            width: NODE_WIDTH,
            height: layout.header_heights[i],
        };
        for (j, &other) in layout.positions.iter().enumerate().skip(i + 1) {
            assert!(!overlaps(
                a,
                Bounds {
                    origin: other,
                    height: layout.header_heights[j],
                    ..a
                }
            ));
        }
        for path in &layout.paths {
            for pair in path.windows(2) {
                assert!(pair[0].x == pair[1].x || pair[0].y == pair[1].y);
                assert!(!routing::crosses(pair[0], pair[1], a));
            }
        }
    }
    assert_eq!(
        svg::render(graph, &layout),
        svg::render(graph, &Layout::new(graph))
    );
}

#[test]
fn relationships_anchor_to_outer_frames_for_nested_peer_and_external_containers() {
    for mode in [EntityMode::Managed, EntityMode::Data] {
        let mut graph = fixture();
        let root = graph
            .nodes
            .iter()
            .position(|node| node.resource_type == "aws_vpc")
            .unwrap();
        let subnet = graph
            .nodes
            .iter()
            .position(|node| node.resource_type == "aws_subnet")
            .unwrap();
        let peer = graph.nodes.len();
        let mut node = graph.nodes[root].clone();
        node.address = "aws_vpc.peer".into();
        node.mode = mode;
        graph.nodes.push(node);
        let card = graph.nodes.len();
        let mut node = graph
            .nodes
            .iter()
            .find(|node| node.resource_type == "aws_instance")
            .unwrap()
            .clone();
        node.address = "aws_instance.external".into();
        graph.nodes.push(node);
        graph
            .edges
            .extend([(root, peer), (peer, subnet), (subnet, card), (card, root)].map(Edge::from));
        let layout = Layout::new(&graph);
        assert!(
            layout.paths[layout.paths.len() - 4..]
                .iter()
                .all(|path| !path.is_empty())
        );
        verify(&graph);
    }
}

#[test]
fn nested_vpc_subnet_ec2_reserve_space_and_route_extra_dependencies() {
    let graph = fixture();
    verify(&graph);
    let layout = Layout::new(&graph);
    assert_eq!(layout.containers.len(), 2);
    assert_eq!(layout.parents.iter().flatten().count(), 2);
    assert_eq!(
        layout.paths.iter().filter(|path| path.is_empty()).count(),
        3
    );
}

#[test]
fn peer_containers_and_cross_module_children_remain_separate_and_identifiable() {
    let mut graph = fixture();
    for node in &mut graph.nodes {
        if node.resource_type == "aws_instance" {
            node.module = "module.compute".into();
        }
    }
    graph.nodes.push(Node {
        address: "aws_vpc.peer".into(),
        resource_type: "aws_vpc".into(),
        module: "module.network".into(),
        action: Action::Create,
        mode: EntityMode::Managed,
        role: ResourceRole::Container,
    });
    graph.edges.push(Edge::from((graph.nodes.len() - 1, 0)));
    verify(&graph);
    let layout = Layout::new(&graph);
    let peer = layout
        .containers
        .iter()
        .find(|(node, _)| *node == graph.nodes.len() - 1)
        .unwrap()
        .1;
    for (edge, path) in graph.edges.iter().zip(&layout.paths) {
        if edge.from == graph.nodes.len() - 1 || edge.to == graph.nodes.len() - 1 {
            continue;
        }
        for pair in path.windows(2) {
            assert!(!routing::crosses(pair[0], pair[1], peer));
        }
    }
    for &(node, bounds) in &layout.containers {
        if node != graph.nodes.len() - 1 {
            assert!(!overlaps(peer, bounds));
        }
    }
    assert!(layout.bands.is_empty());
    assert!(svg::render(&graph, &layout).contains("module.compute"));
}

#[test]
fn ambiguous_parents_and_cycles_keep_visible_edges_instead_of_nesting() {
    let mut graph = fixture();
    for node in &mut graph.nodes {
        node.role = ResourceRole::Container;
    }
    graph.edges = vec![
        Edge::from((0, 1)),
        Edge::from((1, 0)),
        Edge::from((0, 2)),
        Edge::from((1, 2)),
    ];
    for edge in &mut graph.edges {
        edge.kind = EdgeKind::Containment;
    }
    let layout = Layout::new(&graph);
    assert!(layout.parents.iter().all(Option::is_none));
    assert!(layout.paths.iter().all(|path| !path.is_empty()));
    verify(&graph);
}
