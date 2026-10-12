use crate::{layout::Layout, model::Action, plan, semantic, svg, view};
use serde_json::{Value, json};
use std::collections::BTreeSet;

fn input() -> Value {
    serde_json::from_str(include_str!("../../fixtures/routes-plan.json")).unwrap()
}

fn output(value: &Value) -> String {
    let graph = semantic::transform(&plan::parse(&value.to_string()).unwrap());
    let layout = Layout::new(&graph);
    verify_route_paths(&graph, &layout);
    svg::render(&graph, &layout)
}

fn verify_route_paths(graph: &crate::model::Graph, layout: &Layout<'_>) {
    let mut ports = std::collections::BTreeMap::<_, Vec<_>>::new();
    for (edge, path) in graph.edges.iter().zip(&layout.paths) {
        let Some(parent) = view::routes::owner(graph, &graph.nodes[edge.from]) else {
            continue;
        };
        if edge.kind != crate::model::EdgeKind::Association || path.is_empty() {
            continue;
        }
        let child = layout.bounds[edge.from];
        let target = layout.bounds[edge.to];
        let boundary = |b: crate::layout::Bounds, p: crate::layout::Point| {
            ((p.x == b.origin.x || p.x == b.right())
                && p.y > b.origin.y
                && p.y < b.origin.y + b.height)
                || ((p.y == b.origin.y || p.y == b.origin.y + b.height)
                    && p.x > b.origin.x
                    && p.x < b.right())
        };
        assert!(boundary(child, path[0]));
        assert!(
            boundary(target, *path.last().unwrap()),
            "target={target:?}, path={path:?}"
        );
        ports
            .entry(edge.to)
            .or_default()
            .push(*path.last().unwrap());
        let obstacles = layout.containment.children[parent]
            .iter()
            .map(|&i| layout.bounds[i])
            .chain([layout.bounds[parent].header(layout.header_heights[parent])]);
        for bounds in obstacles {
            for p in path.windows(2) {
                let horizontal = p[0].y == p[1].y;
                let intersects = if horizontal {
                    p[0].y > bounds.origin.y
                        && p[0].y < bounds.origin.y + bounds.height
                        && p[0].x.min(p[1].x) < bounds.right()
                        && p[0].x.max(p[1].x) > bounds.origin.x
                } else {
                    p[0].x > bounds.origin.x
                        && p[0].x < bounds.right()
                        && p[0].y.min(p[1].y) < bounds.origin.y + bounds.height
                        && p[0].y.max(p[1].y) > bounds.origin.y
                };
                assert!(
                    !intersects,
                    "route intersects card/header: {p:?}, {bounds:?}"
                );
            }
        }
        assert!(
            path.windows(2)
                .all(|p| p[0].x == p[1].x || p[0].y == p[1].y)
        );
    }
    for points in ports.values() {
        for (i, a) in points.iter().enumerate() {
            for b in &points[i + 1..] {
                assert!(a.x.abs_diff(b.x) + a.y.abs_diff(b.y) >= 24);
            }
        }
    }
    let index = svg::relationship_index::Index::new(graph, layout);
    let markers: Vec<_> = index.marker_bounds().collect();
    let overlaps = |a: crate::layout::Bounds, b: crate::layout::Bounds| {
        a.origin.x < b.right()
            && b.origin.x < a.right()
            && a.origin.y < b.origin.y + b.height
            && b.origin.y < a.origin.y + a.height
    };
    for (i, &marker) in markers.iter().enumerate() {
        assert!(
            markers[i + 1..]
                .iter()
                .all(|&other| !overlaps(marker, other))
        );
        for (node, &bounds) in layout.bounds.iter().enumerate() {
            let bounds = if layout.containers.contains(&node) {
                bounds.header(layout.header_heights[node])
            } else {
                bounds
            };
            assert!(
                !overlaps(marker, bounds),
                "marker covers card/header: {marker:?}, {bounds:?}"
            );
        }
    }
}

#[test]
fn nonspatial_next_hop_keeps_status_without_a_spatial_path() {
    let mut graph = semantic::transform(&plan::parse(&input().to_string()).unwrap());
    let target = graph
        .0
        .nodes
        .iter_mut()
        .find(|n| n.resource_type == "aws_internet_gateway")
        .unwrap();
    target.placement = crate::model::Placement::Logical;
    let layout = Layout::new(&graph);
    for (edge, path) in graph.edges.iter().zip(&layout.paths) {
        if edge.kind == crate::model::EdgeKind::Association {
            assert!(path.is_empty());
        }
    }
    let svg = svg::render(&graph, &layout);
    verify_cards(&svg);
    assert!(svg.contains("(nonspatial)"));
    assert!(!svg.contains("Next hop ["));
    assert!(
        svg.contains(
            "2 spatial cards, 1 logical resources in 1 group cards, 2 embedded route cards"
        )
    );
}

fn verify_cards(svg: &str) {
    let doc = roxmltree::Document::parse(svg).unwrap();
    let mut ids = BTreeSet::new();
    for id in doc.descendants().filter_map(|n| n.attribute("id")) {
        assert!(ids.insert(id), "duplicate {id}");
    }
    for href in doc.descendants().filter_map(|n| n.attribute("href")) {
        assert!(
            ids.contains(href.strip_prefix('#').unwrap()),
            "dangling {href}"
        );
    }
    let number =
        |n: roxmltree::Node<'_, '_>, key: &str| n.attribute(key).unwrap().parse::<usize>().unwrap();
    for card in doc.descendants().filter(|n| {
        n.attribute("data-embedded-route").is_some() || n.attribute("data-route-entry").is_some()
    }) {
        let rect = card.children().find(|n| n.has_tag_name("rect")).unwrap();
        let (x, y, width, height) = (
            number(rect, "x"),
            number(rect, "y"),
            number(rect, "width"),
            number(rect, "height"),
        );
        let parent = card
            .ancestors()
            .skip(1)
            .find(|n| n.attribute("data-terraform-address").is_some())
            .unwrap();
        let outer = parent.children().find(|n| n.has_tag_name("rect")).unwrap();
        assert!(x > number(outer, "x") && x + width < number(outer, "x") + number(outer, "width"));
        assert!(
            y > number(outer, "y") && y + height < number(outer, "y") + number(outer, "height")
        );
        for text in card.descendants().filter(|n| n.has_tag_name("text")) {
            assert!(number(text, "y") > y && number(text, "y") < y + height);
        }
    }
}

#[test]
fn route_children_keep_identity_palette_metadata_and_reference_edges() {
    for (actions, action, visible_operation) in [
        (json!(["create"]), Action::Create, "create"),
        (json!(["no-op"]), Action::Unchanged, "unchanged"),
        (json!(["update"]), Action::Update, "update"),
        (json!(["delete", "create"]), Action::Replace, "replace"),
        (json!(["delete"]), Action::Delete, "delete"),
        (json!(["read"]), Action::Read, "read"),
        (json!(["forget"]), Action::Other, "forget"),
        (
            json!(["create", "forget"]),
            Action::Other,
            "create then forget",
        ),
        (
            json!(["forget", "create"]),
            Action::Other,
            "forget then create",
        ),
    ] {
        let mut value = input();
        value["resource_changes"][2]["change"]["actions"] = actions;
        value["resource_changes"][2]["previous_address"] = "aws_route.old".into();
        value["resource_changes"][2]["change"]["importing"] = json!({"id":"TOP_SECRET_IMPORT"});
        let raw = plan::parse(&value.to_string()).unwrap();
        let graph = semantic::transform(&raw);
        assert_eq!(graph.nodes.len(), raw.nodes.len());
        let layout = Layout::new(&graph);
        let svg = svg::render(&graph, &layout);
        assert!(svg.contains(&format!("{} resources (", raw.nodes.len())));
        verify_cards(&svg);
        let doc = roxmltree::Document::parse(&svg).unwrap();
        let child = doc
            .descendants()
            .find(|n| n.attribute("data-terraform-address") == Some("aws_route.default"))
            .unwrap();
        assert_eq!(child.attribute("data-embedded-route"), Some("true"));
        let badge = child
            .children()
            .find(|n| n.attribute("data-import-badge") == Some("true"))
            .unwrap();
        let badge_rect = badge.children().find(|n| n.has_tag_name("rect")).unwrap();
        let badge_bottom = badge_rect.attribute("y").unwrap().parse::<usize>().unwrap()
            + badge_rect
                .attribute("height")
                .unwrap()
                .parse::<usize>()
                .unwrap();
        let heading = child.children().find(|n| n.has_tag_name("text")).unwrap();
        assert!(badge_bottom + 10 < heading.attribute("y").unwrap().parse::<usize>().unwrap());
        let visible: String = child
            .children()
            .filter(|n| n.has_tag_name("text"))
            .filter_map(|n| n.text())
            .collect();
        assert!(visible.starts_with(&format!("aws_route.default [{visible_operation}]")));
        assert_eq!(child.attribute("data-import-id-present"), Some("true"));
        assert_eq!(
            child.attribute("data-import-identity-present"),
            Some("false")
        );
        assert_eq!(child.attribute("data-import-unknown"), Some("false"));
        assert_eq!(
            child.attribute("data-previous-address"),
            Some("aws_route.old")
        );
        let moved = child
            .children()
            .filter(|n| n.attribute("data-move-annotation") == Some("true"))
            .filter_map(|n| n.text())
            .collect::<String>();
        assert_eq!(moved, "Moved from: aws_route.old");
        let title = child
            .children()
            .find(|n| n.has_tag_name("title"))
            .and_then(|n| n.text())
            .unwrap();
        assert!(title.contains("moved from: aws_route.old"));
        assert_eq!(title.matches("aws_route.old").count(), 1);
        let rect = child.children().find(|n| n.has_tag_name("rect")).unwrap();
        assert_eq!(rect.attribute("fill"), Some(svg::color(action).0));
        assert_eq!(rect.attribute("stroke"), Some(svg::color(action).1));
        assert!(
            child
                .ancestors()
                .any(|n| n.attribute("data-terraform-address") == Some("aws_route_table.public"))
        );
        for (edge, path) in graph.edges.iter().zip(&layout.paths) {
            if graph.nodes[edge.from].routes.owner.is_some()
                || graph.nodes[edge.to].routes.owner.is_some()
            {
                if edge.kind == crate::model::EdgeKind::Association {
                    assert!(path.len() >= 2);
                    let child = layout.bounds[edge.from];
                    assert!(path[0].x == child.origin.x || path[0].x == child.right());
                    assert!(
                        path[0].y > child.origin.y && path[0].y < child.origin.y + child.height
                    );
                    assert_eq!(
                        edge.change.as_ref().unwrap().action,
                        graph.nodes[edge.from].action
                    );
                } else {
                    assert!(path.is_empty());
                }
            }
        }
        assert!(!svg.contains("TOP_SECRET"));
        assert_eq!(
            svg.matches("data-source-address=\"aws_route.default\"")
                .count(),
            1
        );
        assert!(
            !child
                .descendants()
                .filter_map(|n| n.text())
                .any(|t| t.starts_with("Target"))
        );
        assert!(
            child
                .descendants()
                .filter_map(|n| n.text())
                .any(|t| t.starts_with("Next hop ["))
        );
    }
}

#[test]
fn unknown_owner_stays_spatial_but_unknown_target_can_still_embed() {
    for (field, embedded) in [("route_table_id", false), ("gateway_id", true)] {
        let mut value = input();
        value["resource_changes"][2]["change"]["after_unknown"] = json!({field:true});
        let graph = semantic::transform(&plan::parse(&value.to_string()).unwrap());
        let route = graph
            .nodes
            .iter()
            .find(|n| n.address == "aws_route.default")
            .unwrap();
        assert_eq!(route.routes.owner.is_some(), embedded);
        assert_eq!(
            route.routes.entries[0].target.is_none(),
            field == "gateway_id"
        );
        let svg = output(&value);
        verify_cards(&svg);
        if field == "gateway_id" {
            assert!(svg.contains("Target: unresolved / unsupported"));
        }
    }
}

#[test]
fn inline_entries_are_distinct_sorted_and_do_not_add_resource_identities() {
    let mut value = input();
    value["resource_changes"][0]["change"]["importing"] = json!({"unknown":true});
    let entries = json!([
        {"cidr_block":"0.0.0.0/0", "gateway_id":"TOP_SECRET_TARGET"},
        {"cidr_block":"0.0.0.0/0", "nat_gateway_id":"TOP_SECRET_OTHER"},
        {"ipv6_cidr_block":"::/0"}, {"destination_prefix_list_id":"pl-012abc"}
    ]);
    value["configuration"]["root_module"]["resources"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "address":"aws_route_table.public", "expressions":{"route":{"constant_value":entries}}
        }));
    let svg = output(&value);
    verify_cards(&svg);
    assert_eq!(svg.matches("data-route-entry=\"inline\"").count(), 4);
    assert_eq!(svg.matches("data-terraform-address=").count(), 5);
    let doc = roxmltree::Document::parse(&svg).unwrap();
    let parent = doc
        .descendants()
        .find(|n| n.attribute("data-terraform-address") == Some("aws_route_table.public"))
        .unwrap();
    assert_eq!(
        parent
            .children()
            .filter(|n| n.attribute("data-import-badge") == Some("true"))
            .count(),
        1
    );
    for inline in parent
        .descendants()
        .filter(|n| n.attribute("data-route-entry").is_some())
    {
        assert!(
            !inline
                .descendants()
                .any(|n| n.attribute("data-import-badge").is_some())
        );
    }
    assert!(
        svg.contains("IPv4: 0.0.0.0/0")
            && svg.contains("IPv6: ::/0")
            && svg.contains("Prefix list: pl-012abc")
    );
    assert!(!svg.contains("TOP_SECRET"));
    let graph = semantic::transform(&plan::parse(&value.to_string()).unwrap());
    let table = graph
        .nodes
        .iter()
        .find(|n| n.address == "aws_route_table.public")
        .unwrap();
    assert_eq!(
        table
            .routes
            .entries
            .iter()
            .filter(|e| e.target.is_some())
            .count(),
        1
    );
    assert!(
        table
            .routes
            .entries
            .iter()
            .any(|e| e.target_kind.as_deref() == Some("aws_internet_gateway"))
    );
    value["configuration"]["root_module"]["resources"][3]["expressions"]["route"]["constant_value"]
        .as_array_mut()
        .unwrap()
        .reverse();
    assert_eq!(output(&value), svg);
    value["configuration"]["root_module"]["resources"][3]["expressions"]["route"] =
        json!({"references":["var.routes"]});
    assert_eq!(
        output(&value)
            .matches("data-route-entry=\"inline\"")
            .count(),
        1
    );
    value["configuration"]["root_module"]["resources"][3]["expressions"]["route"] =
        json!({"constant_value":[]});
    assert_eq!(
        output(&value)
            .matches("data-route-entry=\"inline\"")
            .count(),
        0
    );
}

#[test]
fn destinations_never_copy_sensitive_or_evaluated_values() {
    for case in 0..7 {
        let mut value = input();
        value["configuration"]["root_module"]["resources"][0]["expressions"]["destination_cidr_block"] =
            json!({"constant_value":"192.0.2.0/24"});
        match case {
            0 => {
                value["resource_changes"][2]["change"]["after_sensitive"] =
                    json!({"destination_cidr_block":true})
            }
            1 => value["resource_changes"][2]["change"]["before_sensitive"] = json!(true),
            2 => {
                value["planned_values"] = json!({"root_module":{"resources":[{"address":"aws_route.default", "sensitive_values":{"destination_cidr_block":true}}]}})
            }
            3 => {
                value["configuration"]["root_module"]["resources"][0]["expressions"]["destination_cidr_block"] =
                    json!({"references":["var.cidr"]})
            }
            4 => {
                value["configuration"]["root_module"]["resources"][0]["expressions"]["destination_cidr_block"] =
                    json!({"constant_value":"TOP_SECRET_INVALID"})
            }
            5 => {
                value["resource_changes"][2]["change"]["after_unknown"] =
                    json!({"destination_cidr_block":true})
            }
            _ => {
                value["configuration"]["root_module"]["resources"][0]["expressions"]["destination_ipv6_cidr_block"] =
                    json!({"references":["var.ipv6"]})
            }
        }
        value["resource_changes"][2]["change"]["after"]["destination_cidr_block"] =
            "192.0.2.0/24".into();
        let svg = output(&value);
        assert!(
            !svg.contains("192.0.2.0/24") && !svg.contains("TOP_SECRET"),
            "case {case}"
        );
    }
}

#[test]
fn many_long_routes_expand_parent_and_keep_deterministic_geometry() {
    let parent_address = format!("aws_route_table.{}", "long_parent_name_".repeat(16));
    let mut value: Value = serde_json::from_str(
        &input()
            .to_string()
            .replace("aws_route_table.public", &parent_address),
    )
    .unwrap();
    let previous = format!(
        "module.{}.aws_route_table.old[\"<&>\"]",
        "長い名前.".repeat(18)
    );
    let parent_change = value["resource_changes"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|change| change["address"] == parent_address)
        .unwrap();
    parent_change["previous_address"] = previous.clone().into();
    let route_previous = format!("module.{}.aws_route.old[0]", "nested.".repeat(14));
    value["resource_changes"][2]["previous_address"] = route_previous.clone().into();
    value["resource_changes"]
        .as_array_mut()
        .unwrap()
        .push(json!({"address":"aws_iam_policy.context","type":"aws_iam_policy"}));
    value["configuration"]["root_module"]["resources"]
        .as_array_mut()
        .unwrap()
        .push(json!({"address":parent_address,"expressions":{"tags":{"references":["aws_iam_policy.context.arn"]}}}));
    for index in 0..24 {
        let address = format!("aws_route.long_{index}_{}", "長い名前".repeat(16));
        let mut change = value["resource_changes"][2].clone();
        change["address"] = address.clone().into();
        value["resource_changes"]
            .as_array_mut()
            .unwrap()
            .push(change);
        let mut config = value["configuration"]["root_module"]["resources"][0].clone();
        config["address"] = address.into();
        value["configuration"]["root_module"]["resources"]
            .as_array_mut()
            .unwrap()
            .push(config);
    }
    let svg = output(&value);
    verify_cards(&svg);
    assert_eq!(svg.matches("data-embedded-route=\"true\"").count(), 26);
    let doc = roxmltree::Document::parse(&svg).unwrap();
    let parent = doc
        .descendants()
        .find(|n| n.attribute("data-terraform-address") == Some(parent_address.as_str()))
        .unwrap();
    let y = |n: roxmltree::Node<'_, '_>| n.attribute("y").unwrap().parse::<usize>().unwrap();
    let heading: Vec<_> = parent
        .children()
        .filter(|n| n.has_tag_name("text") && n.attribute("font-size") == Some("13"))
        .collect();
    assert!(heading.len() > 2);
    assert_eq!(
        heading.iter().filter_map(|n| n.text()).collect::<String>(),
        parent.attribute("data-qualified-address").unwrap()
    );
    let moved = parent
        .children()
        .find(|n| n.attribute("data-move-annotation") == Some("true"))
        .unwrap();
    let moved_lines: Vec<_> = moved
        .children()
        .filter(|n| n.has_tag_name("tspan"))
        .collect();
    assert!(moved_lines.len() > 2);
    assert_eq!(
        moved_lines
            .iter()
            .filter_map(|n| n.text())
            .collect::<String>(),
        format!("Moved from: {previous}")
    );
    assert!(y(*heading.last().unwrap()) + 10 < y(moved_lines[0]));
    let references = parent
        .children()
        .find(|n| n.attribute("data-resource-references").is_some())
        .unwrap();
    let uses = references
        .children()
        .find(|n| n.has_tag_name("text"))
        .unwrap();
    assert!(y(*moved_lines.last().unwrap()) + 4 < y(uses));
    for child in doc.descendants().filter(|n| {
        n.attribute("data-embedded-route") == Some("true")
            && n.attribute("data-previous-address").is_some()
    }) {
        let lines: Vec<_> = child
            .children()
            .filter(|n| n.attribute("data-move-annotation") == Some("true"))
            .collect();
        assert!(lines.len() > 2);
        assert_eq!(
            lines.iter().filter_map(|n| n.text()).collect::<String>(),
            format!("Moved from: {route_previous}")
        );
    }
    let reference_bottom = references
        .descendants()
        .filter(|n| n.has_tag_name("text"))
        .map(y)
        .max()
        .unwrap();
    let routes = parent
        .children()
        .find(|n| n.attribute("data-routes").is_some())
        .unwrap();
    let route_heading = routes.children().find(|n| n.has_tag_name("text")).unwrap();
    assert!(reference_bottom + 4 < y(route_heading));
    let route_rects: Vec<_> = routes
        .descendants()
        .filter(|n| n.has_tag_name("rect"))
        .collect();
    let first_route_y = route_rects.iter().map(|n| y(*n)).min().unwrap();
    assert!(y(route_heading) + 4 < first_route_y);
    let route_bottom = route_rects
        .iter()
        .map(|n| y(*n) + n.attribute("height").unwrap().parse::<usize>().unwrap())
        .max()
        .unwrap();
    for footer in parent
        .children()
        .filter(|n| n.has_tag_name("text") && n.attribute("font-size") == Some("10"))
    {
        assert!(route_bottom + 4 < y(footer));
    }
    value["resource_changes"].as_array_mut().unwrap().reverse();
    value["configuration"]["root_module"]["resources"]
        .as_array_mut()
        .unwrap()
        .reverse();
    assert_eq!(output(&value), svg);
}

#[test]
fn focused_and_changed_routes_keep_parent_without_dangling_links() {
    let graph = semantic::transform(&plan::parse(&input().to_string()).unwrap());
    for options in [
        view::Options {
            focus: Some("aws_route.default".into()),
            depth: Some(0),
            ..Default::default()
        },
        view::Options {
            changed_only: true,
            ..Default::default()
        },
    ] {
        let selected = view::single(&graph, &options).unwrap();
        assert!(
            selected
                .nodes
                .iter()
                .any(|n| n.address == "aws_route_table.public")
        );
        assert!(
            !selected
                .nodes
                .iter()
                .any(|n| n.address == "aws_internet_gateway.main")
        );
        let svg = svg::render(&selected, &Layout::new(&selected));
        verify_cards(&svg);
        assert!(svg.contains("(unavailable)"));
    }
}

#[test]
fn deposed_routes_keep_separate_identity_and_unproven_ownership_visible() {
    let mut value = input();
    let mut old = value["resource_changes"][2].clone();
    old["deposed"] = "old".into();
    old["change"]["actions"] = json!(["delete"]);
    value["resource_changes"].as_array_mut().unwrap().push(old);
    let svg = output(&value);
    verify_cards(&svg);
    let doc = roxmltree::Document::parse(&svg).unwrap();
    let cards: Vec<_> = doc
        .descendants()
        .filter(|n| n.attribute("data-terraform-address") == Some("aws_route.default"))
        .collect();
    assert_eq!(cards.len(), 2);
    assert!(
        cards
            .iter()
            .any(|n| n.attribute("data-deposed-key") == Some("old")
                && n.attribute("data-embedded-route").is_none())
    );
    assert!(
        cards
            .iter()
            .any(|n| n.attribute("data-embedded-route") == Some("true"))
    );
}

#[test]
fn named_states_namespace_child_links_and_attach_cross_state_paths_to_parent_border() {
    use crate::model::architecture::{
        Architecture, CrossStateEdge, CrossStateProvenance, Endpoint, StateId,
    };
    let first = StateId::new("first").unwrap();
    let second = StateId::new("second").unwrap();
    let graph = semantic::transform(&plan::parse(&input().to_string()).unwrap());
    let all = Architecture {
        states: std::collections::BTreeMap::from([
            (first.clone(), graph.clone()),
            (second.clone(), graph),
        ]),
        relationships: vec![CrossStateEdge {
            from: Endpoint {
                state: first,
                address: "aws_route.default".into(),
            },
            to: Endpoint {
                state: second,
                address: "aws_route.second".into(),
            },
            provenance: CrossStateProvenance::TerraformRemoteState {
                remote: "data.terraform_remote_state.routes".into(),
                output: "route".into(),
                output_action: None,
            },
        }],
    };
    let selected = view::apply(&all, &Default::default()).unwrap();
    let svg = svg::render_states(&selected, Default::default());
    verify_cards(&svg);
    let doc = roxmltree::Document::parse(&svg).unwrap();
    let boundaries: Vec<_> = doc
        .descendants()
        .filter(|n| n.attribute("data-route-table").is_some())
        .collect();
    assert_eq!(boundaries.len(), 2);
    let mut boundary_ids = BTreeSet::new();
    for boundary in boundaries {
        let id = boundary.attribute("data-route-table").unwrap();
        assert!(boundary_ids.insert(id));
        let card = doc
            .descendants()
            .find(|n| n.attribute("id") == Some(id))
            .unwrap();
        let state = |n: roxmltree::Node<'_, '_>| {
            n.ancestors()
                .find_map(|a| a.attribute("data-state-id"))
                .unwrap()
                .to_owned()
        };
        assert_eq!(state(boundary), state(card));
        assert_eq!(
            card.attribute("data-terraform-address"),
            Some("aws_route_table.public")
        );
    }
    assert_eq!(
        doc.descendants()
            .filter(|n| n.attribute("data-embedded-route").is_some())
            .count(),
        4
    );
    let path = doc
        .descendants()
        .find(|n| n.attribute("data-cross-state") == Some("true"))
        .unwrap();
    for (endpoint, port) in [
        ("data-source", "data-source-port"),
        ("data-target", "data-target-port"),
    ] {
        let child_id = path.attribute(endpoint).unwrap();
        let port_id = path.attribute(port).unwrap();
        assert_ne!(child_id, port_id);
        let child = doc
            .descendants()
            .find(|n| n.attribute("id") == Some(child_id))
            .unwrap();
        assert!(
            child
                .ancestors()
                .any(|n| n.attribute("id") == Some(port_id))
        );
    }
    assert_eq!(all.relationships, selected.relationships());
}
