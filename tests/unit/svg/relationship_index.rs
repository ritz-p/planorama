use super::*;
use crate::layout::Point;
use crate::model::{EdgeKind, RelationshipProvenance};
use crate::{plan, semantic, svg};

fn graph() -> Graph {
    semantic::transform(&plan::parse(include_str!("../../fixtures/association-plan.json")).unwrap())
        .0
}

#[test]
fn provenance_scope_survives_lowering_and_stays_separate_from_endpoint_scopes() {
    let input = include_str!("../../fixtures/relationship-scopes-plan.json");
    let mut graph =
        semantic::transform(&crate::plan::evidence::tests::parse_fixture(input).unwrap()).0;
    assert_eq!(graph.nodes.len(), 2);
    let RelationshipProvenance::Resource { deployment, .. } = &graph.relationships[0].provenance[0]
    else {
        panic!()
    };
    let deployment = deployment.as_ref().unwrap();
    assert_eq!(deployment.configuration.as_ref().unwrap().key, "aws.tokyo");
    assert_eq!(
        deployment
            .scope
            .as_ref()
            .unwrap()
            .region
            .as_ref()
            .unwrap()
            .0,
        "ap-northeast-1"
    );
    let layout = Layout::new(&graph);
    let index = Index::new(&graph, &layout);
    let id = index.entries[0].id.clone();
    assert_eq!(
        index.entries[0]
            .scopes
            .values()
            .next()
            .unwrap()
            .iter()
            .next()
            .unwrap(),
        "AWS Account: 111111111111 · Region: ap-northeast-1"
    );
    let labels: Vec<_> = (0..2)
        .map(|i| index.entries[0].context[0].endpoint_label(i))
        .collect();
    assert!(labels.iter().any(|text| {
        text.contains("aws_subnet.private · AWS Account: 111111111111 · Region: ap-northeast-1")
    }));
    assert!(labels.iter().any(|text| {
        text.contains("aws_route_table.private · AWS Account: 222222222222 · Region: us-east-1")
    }));
    for (global, provider, expected) in [
        (
            true,
            crate::model::ProviderIdentity::InferredAws,
            "AWS Global",
        ),
        (
            false,
            crate::model::ProviderIdentity::InferredAws,
            "Region: unknown",
        ),
        (
            false,
            crate::model::ProviderIdentity::InferredTerraform,
            "Region: not applicable",
        ),
    ] {
        graph.nodes[0].provider = provider;
        graph.nodes[0].entity.scope = global.then(|| crate::model::architecture::DeploymentScope {
            provider: "AWS".into(),
            region: None,
            account: None,
            global: true,
        });
        let RelationshipProvenance::Resource { deployment, .. } =
            &mut graph.relationships[0].provenance[0]
        else {
            panic!()
        };
        let deployment = deployment.as_mut().unwrap();
        deployment.scope = graph.nodes[0].entity.scope.clone();
        deployment.provider = graph.nodes[0].provider.clone();
        let layout = Layout::new(&graph);
        let mut index = Index::new(&graph, &layout);
        assert_eq!(index.entries[0].id, id);
        assert!(
            index.entries[0]
                .scopes
                .values()
                .any(|labels| labels.contains(expected))
        );
        assert!(
            index.entries[0].context[0]
                .scopes
                .iter()
                .any(|label| label == expected)
        );
        let card = index.cards().remove(0);
        let doc = roxmltree::Document::parse(&card.content).unwrap();
        assert!(
            doc.descendants()
                .filter(|n| n.has_tag_name("tspan"))
                .all(|n| n.attribute("y").unwrap().parse::<usize>().unwrap() + 4 < card.height)
        );
        index.entries[0].context[0].endpoints[0].1 = None;
        let unavailable = index.entries[0].context[0].endpoint_label(0);
        assert!(unavailable.contains("unavailable in this view"));
        assert!(!unavailable.contains("Region:") && !unavailable.contains("Global"));
    }
    let mut reordered: serde_json::Value = serde_json::from_str(input).unwrap();
    reordered["resource_changes"]
        .as_array_mut()
        .unwrap()
        .reverse();
    let original =
        semantic::transform(&crate::plan::evidence::tests::parse_fixture(input).unwrap());
    let reordered = semantic::transform(
        &crate::plan::evidence::tests::parse_fixture(&reordered.to_string()).unwrap(),
    );
    assert_eq!(original, reordered);
}

#[test]
fn native_navigation_keeps_parallel_relationships_and_endpoint_identities_distinct() {
    let graph = dense_graph(12, EdgeKind::Association);
    let layout = Layout::new(&graph);
    let index = Index::new(&graph, &layout);
    assert_eq!(
        index
            .cards()
            .iter()
            .map(|card| card.content.matches("data-relationship-entry=").count())
            .collect::<Vec<_>>(),
        [5, 5, 2]
    );
    let rendered = svg::render(&graph, &layout);
    let document = roxmltree::Document::parse(&rendered).unwrap();
    let ids: Vec<_> = document
        .descendants()
        .filter_map(|node| node.attribute("id"))
        .collect();
    assert_eq!(ids.len(), ids.iter().collect::<BTreeSet<_>>().len());
    for entry in &index.entries {
        let node = document
            .descendants()
            .find(|node| node.attribute("id") == Some(entry.id.as_str()))
            .unwrap();
        assert_eq!(
            node.attribute("data-relationship-entry"),
            Some(entry.number.to_string().as_str())
        );
        let endpoints: Vec<_> = node
            .descendants()
            .filter(|node| node.attribute("data-relationship-endpoint").is_some())
            .collect();
        assert_eq!(endpoints.len(), 2);
        for endpoint in endpoints {
            assert!(
                ids.contains(
                    &endpoint
                        .attribute("href")
                        .unwrap()
                        .strip_prefix('#')
                        .unwrap()
                )
            );
        }
        assert_eq!(
            document
                .descendants()
                .filter(|node| node.attribute("data-relationship-ids") == Some(entry.id.as_str()))
                .count(),
            1
        );
        assert_eq!(
            document
                .descendants()
                .filter(|node| node.attribute("href") == Some(format!("#{}", entry.id).as_str()))
                .count(),
            3
        );
        assert!(entry.title().contains("Connects (association, undirected)"));
    }
    let mut reordered = graph.clone();
    reordered.edges.reverse();
    reordered.relationships.reverse();
    let reordered_index = Index::new(&reordered, &Layout::new(&reordered));
    assert_eq!(
        index
            .entries
            .iter()
            .map(|entry| &entry.id)
            .collect::<Vec<_>>(),
        reordered_index
            .entries
            .iter()
            .map(|entry| &entry.id)
            .collect::<Vec<_>>()
    );
}

#[test]
fn directed_index_context_links_embedded_routes_without_attribute_values() {
    let graph =
        semantic::transform(&plan::parse(include_str!("../../fixtures/routes-plan.json")).unwrap())
            .0;
    let layout = Layout::new(&graph);
    let index = Index::new(&graph, &layout);
    let rendered = svg::render(&graph, &layout);
    let document = roxmltree::Document::parse(&rendered).unwrap();
    for entry in index.entries.iter().filter(|entry| entry.embedded) {
        assert!(entry.title().contains("From → To (association, directed)"));
        let node = document
            .descendants()
            .find(|node| node.attribute("id") == Some(entry.id.as_str()))
            .unwrap();
        for endpoint in node
            .descendants()
            .filter(|node| node.attribute("data-relationship-endpoint").is_some())
        {
            let id = endpoint
                .attribute("href")
                .unwrap()
                .strip_prefix('#')
                .unwrap();
            assert!(
                document
                    .descendants()
                    .any(|node| node.attribute("id") == Some(id))
            );
        }
    }
}

#[test]
fn current_and_deposed_provenance_and_long_endpoints_keep_their_targets() {
    let mut graph = dense_graph(2, EdgeKind::Association);
    let address = graph.edges[0].change.as_ref().unwrap().address.clone();
    let change = graph.edges[1].change.as_mut().unwrap();
    change.address = address.clone();
    change.action = crate::model::Action::Delete;
    for provenance in &mut graph.relationships[1].provenance {
        if let RelationshipProvenance::Resource {
            source,
            change: provenance_change,
            ..
        } = provenance
        {
            source.address = address.clone();
            source.deposed_key = Some("old<&>".into());
            *provenance_change = change.clone();
        }
    }
    let target = graph.edges[0].to;
    graph.nodes[target].address = format!(
        "module.{}.aws_route_table.private",
        "long_endpoint.".repeat(30)
    );
    graph.nodes[target].deposed_key = Some("old-target".into());
    let layout = Layout::new(&graph);
    let index = Index::new(&graph, &layout);
    assert_eq!(index.entries.len(), 2);
    assert_ne!(index.entries[0].id, index.entries[1].id);
    let rendered = svg::render(&graph, &layout);
    let document = roxmltree::Document::parse(&rendered).unwrap();
    for node in document
        .descendants()
        .filter(|node| node.attribute("data-relationship-endpoint").is_some())
    {
        let target = node.attribute("href").unwrap().strip_prefix('#').unwrap();
        assert!(
            document
                .descendants()
                .any(|node| node.attribute("id") == Some(target))
        );
    }
    for card in index.cards() {
        let document = roxmltree::Document::parse(&card.content).unwrap();
        for line in document
            .descendants()
            .filter(|node| node.has_tag_name("tspan"))
        {
            assert!(
                attribute_number(line, "x") + crate::view::text::width(line.text().unwrap()) * 8
                    <= card.width - 16
            );
            assert!(attribute_number(line, "y") + 4 < card.height);
        }
        assert!(card.content.contains("deposed: old&lt;&amp;&gt;"));
        assert!(card.content.contains("Action: delete"));
    }
}

#[test]
fn overlapping_provenance_sets_do_not_mix_parallel_relationship_actions() {
    let mut graph = dense_graph(2, EdgeKind::Association);
    let address = graph.edges[0].change.as_ref().unwrap().address.clone();
    let change = graph.edges[1].change.as_mut().unwrap();
    change.address = address.clone();
    change.action = crate::model::Action::Delete;
    let extra = graph.relationships[1].provenance[0].clone();
    for provenance in &mut graph.relationships[1].provenance {
        if let RelationshipProvenance::Resource {
            source,
            change: source_change,
            ..
        } = provenance
        {
            source.address = address.clone();
            *source_change = change.clone();
        }
    }
    graph.relationships[1].provenance.push(extra);
    let layout = Layout::new(&graph);
    let index = Index::new(&graph, &layout);
    assert_eq!(index.entries.len(), 2);
    let single = index
        .entries
        .iter()
        .find(|entry| entry.sources.len() == 1)
        .unwrap();
    let multiple = index
        .entries
        .iter()
        .find(|entry| entry.sources.len() == 2)
        .unwrap();
    assert_eq!(single.edges, [0]);
    assert_eq!(multiple.edges, [1]);
    assert_eq!(
        single.actions[single.sources[0]],
        BTreeSet::from(["create".to_owned()])
    );
    let shared = multiple
        .sources
        .iter()
        .find(|source| source.address == address)
        .unwrap();
    assert_eq!(
        multiple.actions[shared],
        BTreeSet::from(["delete".to_owned()])
    );
}

fn dense_graph(count: usize, kind: EdgeKind) -> Graph {
    let mut graph = graph();
    let edge = graph.edges[0].clone();
    let relationship = graph
        .relationships
        .iter()
        .find(|r| {
            r.provenance
                .iter()
                .any(|p| matches!(p, RelationshipProvenance::Resource { .. }))
        })
        .unwrap()
        .clone();
    graph.edges.clear();
    graph.relationships.clear();
    for number in 0..count {
        let mut edge = edge.clone();
        edge.kind = kind;
        edge.change.as_mut().unwrap().address = format!("association.test_{number}");
        let mut relationship = relationship.clone();
        relationship.kind = kind;
        for provenance in &mut relationship.provenance {
            if let RelationshipProvenance::Resource { source, change, .. } = provenance {
                source.address = edge.change.as_ref().unwrap().address.clone();
                *change = edge.change.as_ref().unwrap().clone();
            }
        }
        graph.edges.push(edge);
        graph.relationships.push(relationship);
    }
    graph
}

fn attribute_number(node: roxmltree::Node, name: &str) -> usize {
    node.attribute(name).unwrap().parse().unwrap()
}

fn annotation_card_bounds(card: roxmltree::Node) -> Bounds {
    let mut position = card
        .attribute("transform")
        .unwrap()
        .strip_prefix("translate(")
        .unwrap()
        .strip_suffix(')')
        .unwrap()
        .split_whitespace()
        .map(|value| value.parse().unwrap());
    let rect = card
        .children()
        .find(|node| node.has_tag_name("rect"))
        .unwrap();
    Bounds {
        origin: Point {
            x: position.next().unwrap(),
            y: position.next().unwrap(),
        },
        width: attribute_number(rect, "width"),
        height: attribute_number(rect, "height"),
    }
}

#[test]
fn association_numbering_preserves_relationship_directions_and_arrows() {
    for fixture in [
        include_str!("../../fixtures/association-plan.json"),
        include_str!("../../fixtures/aws-relationships-plan.json"),
        include_str!("../../fixtures/routes-plan.json"),
    ] {
        let graph = semantic::transform(&plan::parse(fixture).unwrap()).0;
        let layout = Layout::new(&graph);
        let svg = svg::render(&graph, &layout);
        let doc = roxmltree::Document::parse(&svg).unwrap();
        let paths: Vec<_> = doc
            .descendants()
            .filter(|n| n.has_tag_name("path") && n.attribute("data-edge-kind").is_some())
            .collect();
        let edges = graph
            .edges
            .iter()
            .zip(&layout.paths)
            .filter(|(_, p)| !p.is_empty())
            .map(|(e, _)| e);
        for (path, edge) in paths.iter().zip(edges) {
            let directed = edge.directionality() == crate::model::Directionality::Directed;
            assert_eq!(path.attribute("marker-end").is_some(), directed);
            assert_eq!(
                path.attribute("data-directionality"),
                Some(edge.directionality().as_str())
            );
            if edge.kind == EdgeKind::Association {
                assert_eq!(path.attribute("stroke-dasharray"), Some("7 4"));
            }
        }
        let index = Index::new(&graph, &layout);
        for entry in &index.entries {
            let expected: usize = entry
                .edges
                .iter()
                .map(|&e| {
                    if graph.edges[e].kind == EdgeKind::Association
                        || graph.edges[e].directionality()
                            == crate::model::Directionality::Undirected
                    {
                        2
                    } else {
                        1
                    }
                })
                .sum();
            assert_eq!(
                index
                    .markers
                    .iter()
                    .filter(|m| m.number == entry.number)
                    .count(),
                expected
            );
        }
        assert_marker_routes(&graph, &layout);
    }
}

#[test]
fn upward_routes_and_numbered_badges_clear_the_annotation_sections() {
    let graph = graph();
    for horizontal_terminals in [false, true] {
        let mut layout = Layout::new(&graph);
        let edge = graph
            .edges
            .iter()
            .position(|edge| edge.change.is_some())
            .unwrap();
        let first = layout.paths[edge][0];
        let last = *layout.paths[edge].last().unwrap();
        layout.paths[edge] = if horizontal_terminals {
            vec![Point { x: first.x, y: 100 }, Point { x: last.x, y: 100 }]
        } else {
            vec![
                first,
                Point {
                    x: first.x + 48,
                    y: first.y,
                },
                Point {
                    x: first.x + 48,
                    y: 100,
                },
                Point {
                    x: last.x + 48,
                    y: 100,
                },
                Point {
                    x: last.x + 48,
                    y: last.y,
                },
                last,
            ]
        };
        let index = Index::new(&graph, &layout);
        if horizontal_terminals {
            assert!(index.marker_bounds().any(|bounds| bounds.origin.y < 100));
        }
        let bounds = layout.bounds.clone();
        let paths = layout.paths.clone();
        let rendered = svg::render(&graph, &layout);
        let document = roxmltree::Document::parse(&rendered).unwrap();
        let number = |node: roxmltree::Node, name| {
            node.attribute(name).unwrap_or("0").parse::<f64>().unwrap()
        };
        let translated_y = |node: roxmltree::Node| {
            node.ancestors()
                .filter_map(|parent| parent.attribute("transform"))
                .map(|value| {
                    value
                        .trim_start_matches("translate(")
                        .trim_end_matches(')')
                        .split_whitespace()
                        .nth(1)
                        .unwrap()
                        .parse::<f64>()
                        .unwrap()
                })
                .sum::<f64>()
        };
        let section_bottom = document
            .descendants()
            .filter(|node| {
                node.attribute("id") == Some("relationship-legend")
                    || node.attribute("data-annotation-section").is_some()
            })
            .map(|section| {
                let rectangle = section
                    .children()
                    .find(|node| node.has_tag_name("rect"))
                    .unwrap();
                translated_y(rectangle) + number(rectangle, "y") + number(rectangle, "height")
            })
            .reduce(f64::max)
            .unwrap();
        assert!(document.descendants().any(|node| {
            node.attribute("data-annotation-section") == Some("Relationship Resources")
        }));
        let architecture = document
            .descendants()
            .find(|node| node.attribute("data-architecture") == Some("true"))
            .unwrap();
        let offset = translated_y(architecture);
        for shape in architecture
            .descendants()
            .filter(|node| node.has_tag_name("rect") || node.has_tag_name("circle"))
        {
            let top = translated_y(shape)
                + if shape.has_tag_name("circle") {
                    number(shape, "cy") - number(shape, "r")
                } else {
                    number(shape, "y")
                };
            let stroke = shape
                .attribute("stroke-width")
                .unwrap_or("1")
                .parse::<f64>()
                .unwrap();
            assert!(top - stroke / 2.0 >= section_bottom + 16.0, "{shape:?}");
        }
        for point in layout.paths.iter().flatten() {
            assert!(offset + point.y as f64 - 7.0 >= section_bottom + 16.0);
        }
        assert_eq!(
            number(document.root_element(), "height"),
            layout.height as f64 + offset
        );
        assert_eq!(layout.bounds, bounds);
        assert_eq!(layout.paths, paths);
    }
}

#[test]
fn dense_targets_keep_distinct_numbers_readable() {
    for (kind, directed, count) in [
        (EdgeKind::Association, false, 24),
        (EdgeKind::Association, true, 24),
        (EdgeKind::Connection, false, 24),
        (EdgeKind::Dependency, true, 12),
    ] {
        let mut graph = dense_graph(12, kind);
        if directed {
            for edge in &mut graph.edges {
                edge.change.as_mut().unwrap().directionality =
                    crate::model::Directionality::Directed;
            }
            for relationship in &mut graph.relationships {
                for provenance in &mut relationship.provenance {
                    if let RelationshipProvenance::Resource { change, .. } = provenance {
                        change.directionality = crate::model::Directionality::Directed;
                    }
                }
            }
        }
        let layout = Layout::new(&graph);
        let index = Index::new(&graph, &layout);
        assert_eq!(index.markers.len(), count);
        assert_eq!(index.entries.len(), 12);
        assert_marker_routes(&graph, &layout);
        for (i, marker) in index.markers.iter().enumerate() {
            for other in &index.markers[i + 1..] {
                let (a, b) = (marker.bounds, other.bounds);
                assert!(
                    a.right() + 3 <= b.origin.x
                        || b.right() + 3 <= a.origin.x
                        || a.origin.y + a.height + 3 <= b.origin.y
                        || b.origin.y + b.height + 3 <= a.origin.y,
                    "{kind:?}: {a:?} overlaps {b:?}"
                );
            }
        }
    }
}

#[test]
fn indexed_flat_graphs_preserve_root_and_child_module_bands() {
    let mut graph = dense_graph(12, EdgeKind::Association);
    for node in &mut graph.nodes {
        node.role = crate::model::ResourceRole::Node;
    }
    graph.nodes[0].module = "module.network".into();
    graph.nodes[1].module = "module.application".into();
    let mut root = graph.nodes[0].clone();
    root.module = "root".into();
    root.address = "test.root".into();
    root.entity = crate::model::ArchitectureEntity::terraform(TerraformEntityId {
        address: root.address.clone(),
        deposed_key: None,
    });
    graph.nodes.push(root);
    let layout = Layout::new(&graph);
    assert_eq!(layout.bands.len(), 3);
    assert_eq!(layout.bands[0].label, "root");
    for (node, bounds) in graph.nodes.iter().zip(&layout.bounds) {
        let band = layout
            .bands
            .iter()
            .find(|band| band.label == node.module)
            .unwrap();
        assert!(bounds.origin.y >= band.top + 48);
        assert!(bounds.origin.y + bounds.height <= band.top + band.height);
    }
    assert_marker_routes(&graph, &layout);
}

#[test]
fn one_numbered_port_does_not_expand_every_port_on_a_hub() {
    let mut graph = dense_graph(1, EdgeKind::Association);
    let target = graph.edges[0].to;
    let source = graph.edges[0].from;
    for _ in 0..99 {
        graph.edges.push(crate::model::Edge::from((target, source)));
    }
    let layout = Layout::new(&graph);
    assert!(
        layout.header_heights[target] < 41 + 100 * 24,
        "{}",
        layout.header_heights[target]
    );
    assert_marker_routes(&graph, &layout);
}

#[test]
fn hundreds_of_numbered_relationships_keep_clear_terminal_corridors() {
    let graph = dense_graph(200, EdgeKind::Association);
    let started = std::time::Instant::now();
    let layout = Layout::new(&graph);
    eprintln!("200 numbered relationships: {:?}", started.elapsed());
    assert_eq!(Index::new(&graph, &layout).markers.len(), 400);
    assert_marker_routes(&graph, &layout);
}

#[test]
fn relationship_cards_fill_the_svg_width_at_exact_and_one_pixel_short_boundaries() {
    let base = dense_graph(1, EdgeKind::Association);
    let mut layout = Layout::new(&base);
    let path = layout.paths[0].clone();
    let bounds = layout.bounds.clone();
    for count in [0usize, 1, 5, 6, 10, 11, 12, 20, 21, 30, 31, 40, 41] {
        let mut graph = dense_graph(count, EdgeKind::Association);
        for (edge, relationship) in graph.edges.iter_mut().zip(&mut graph.relationships) {
            let change = edge.change.as_mut().unwrap();
            change.address = format!("module.{}.{}", "long_module.".repeat(8), change.address);
            for provenance in &mut relationship.provenance {
                if let RelationshipProvenance::Resource {
                    source,
                    change: source_change,
                    ..
                } = provenance
                {
                    source.address = change.address.clone();
                    *source_change = change.clone();
                }
            }
        }
        let mut addresses: Vec<_> = graph
            .edges
            .iter()
            .map(|edge| edge.change.as_ref().unwrap().address.as_str())
            .collect();
        addresses.sort();
        layout.paths = vec![path.clone(); count];
        let mut architecture_content = None;
        let mut expected_section = String::new();
        for capacity in [2usize, 3, 4, 6] {
            for shortfall in [0, 1] {
                let columns = capacity - shortfall;
                layout.width = 112 + capacity * 640 + (capacity - 1) * 20 - shortfall;
                let index = Index::new(&graph, &layout);
                assert_eq!(index.width, layout.width);
                assert_eq!(index.markers.len(), count * 2);
                assert!(index.cards().iter().all(|card| card.width == 640));
                let rendered = svg::render(&graph, &layout);
                let document = roxmltree::Document::parse(&rendered).unwrap();
                assert_eq!(
                    attribute_number(document.root_element(), "width"),
                    layout.width
                );
                assert_eq!(
                    rendered.matches(">Relationship Resources</text>").count(),
                    usize::from(count > 0)
                );
                let section = document.descendants().find(|node| {
                    node.attribute("data-annotation-section") == Some("Relationship Resources")
                });
                if count == 0 {
                    assert!(section.is_none());
                    assert!(!rendered.contains("data-relationship-entry="));
                    continue;
                }
                let section = section.unwrap();
                let section_rect = section
                    .children()
                    .find(|node| node.has_tag_name("rect"))
                    .unwrap();
                assert_eq!(attribute_number(section_rect, "width"), layout.width - 80);
                let cards: Vec<_> = section
                    .children()
                    .filter(|node| node.attribute("data-annotation-card").is_some())
                    .collect();
                assert_eq!(cards.len(), count.div_ceil(5));
                let mut y = 48;
                for row in cards.chunks(columns) {
                    for (column, card) in row.iter().enumerate() {
                        let bounds = annotation_card_bounds(*card);
                        assert_eq!(
                            bounds.origin,
                            Point {
                                x: 16 + column * 660,
                                y
                            }
                        );
                        assert_eq!(bounds.width, 640);
                        assert!(bounds.right() <= layout.width - 96);
                    }
                    y += row
                        .iter()
                        .map(|card| annotation_card_bounds(*card).height)
                        .max()
                        .unwrap()
                        + 20;
                }
                assert_eq!(attribute_number(section_rect, "height"), y - 20 + 16);
                for (card_index, card) in cards.iter().enumerate() {
                    let entries: Vec<_> = card
                        .descendants()
                        .filter(|node| node.attribute("data-relationship-entry").is_some())
                        .collect();
                    assert_eq!(
                        entries
                            .iter()
                            .map(|node| attribute_number(*node, "data-relationship-entry"))
                            .collect::<Vec<_>>(),
                        (card_index * 5 + 1..=count.min((card_index + 1) * 5)).collect::<Vec<_>>()
                    );
                    for entry in entries {
                        let source = entry
                            .descendants()
                            .find(|node| node.attribute("data-source-address").is_some())
                            .unwrap();
                        let visible: String = source
                            .children()
                            .filter(|node| node.has_tag_name("tspan"))
                            .filter_map(|node| node.text())
                            .collect();
                        let address =
                            addresses[attribute_number(entry, "data-relationship-entry") - 1];
                        assert_eq!(source.attribute("data-source-address"), Some(address));
                        assert_eq!(visible, address);
                    }
                }
                let architecture = document
                    .descendants()
                    .find(|node| node.attribute("data-architecture").is_some())
                    .unwrap();
                let content: Vec<_> = architecture
                    .children()
                    .filter(|node| {
                        node.attribute("data-terraform-address").is_some()
                            || node.attribute("data-edge-kind").is_some()
                            || node.attribute("data-relationship-marker").is_some()
                    })
                    .map(|node| rendered[node.range()].to_owned())
                    .collect();
                assert!(!content.is_empty());
                if let Some(expected) = &architecture_content {
                    assert_eq!(&content, expected);
                } else {
                    architecture_content = Some(content);
                }
                expected_section = rendered[section.range()].to_owned();
            }
        }
        assert_eq!(layout.bounds, bounds);
        assert!(layout.paths.iter().all(|points| points == &path));
        if count == 0 {
            continue;
        }
        graph.edges.reverse();
        graph.relationships.reverse();
        let rendered = svg::render(&graph, &layout);
        let document = roxmltree::Document::parse(&rendered).unwrap();
        let section = document
            .descendants()
            .find(|node| {
                node.attribute("data-annotation-section") == Some("Relationship Resources")
            })
            .unwrap();
        assert_eq!(rendered[section.range()], expected_section);
    }
}

#[test]
fn wide_addresses_wrap_within_the_canvas_and_reserved_height() {
    let mut graph = graph();
    let address = format!(
        "aws_route_table_association.example[\"{}\"]",
        "東京大阪".repeat(80)
    );
    for relationship in &mut graph.relationships {
        for provenance in &mut relationship.provenance {
            if let RelationshipProvenance::Resource { source, .. } = provenance {
                source.address = address.clone();
            }
        }
    }
    let layout = Layout::new(&graph);
    let index = Index::new(&graph, &layout);
    let cards = index.cards();
    assert_eq!(cards.len(), 1);
    let svg = &cards[0].content;
    let document = roxmltree::Document::parse(svg).unwrap();
    let lines: Vec<_> = document
        .descendants()
        .filter(|n| {
            n.has_tag_name("tspan")
                && n.parent()
                    .unwrap()
                    .attribute("data-source-address")
                    .is_some()
        })
        .collect();
    assert_eq!(
        lines.iter().filter_map(|n| n.text()).collect::<String>(),
        address
    );
    for line in &lines {
        let width: usize = line
            .text()
            .unwrap()
            .chars()
            .map(|c| if c.is_ascii() { 8 } else { 16 })
            .sum();
        assert!(
            line.attribute("x").unwrap().parse::<usize>().unwrap() + width <= cards[0].width - 16
        );
        assert!(line.attribute("y").unwrap().parse::<usize>().unwrap() + 4 < cards[0].height);
    }
    assert!(cards[0].height > 44 + lines.len() * 20);
}

#[test]
fn a_long_multisource_fifth_entry_stays_in_the_first_card() {
    let mut graph = dense_graph(12, EdgeKind::Association);
    let fifth_address = crate::layout::relationship_markers::entries(&graph)[4].sources[0]
        .address
        .clone();
    let relationship = graph
        .relationships
        .iter_mut()
        .find(|relationship| {
            relationship.provenance.iter().any(|provenance| {
                matches!(provenance, RelationshipProvenance::Resource { source, .. } if source.address == fifth_address)
            })
        })
        .unwrap();
    let RelationshipProvenance::Resource { change, .. } = relationship.provenance[0].clone() else {
        panic!()
    };
    let address = format!("z.{}.extra", "long_module.".repeat(100));
    relationship
        .provenance
        .push(RelationshipProvenance::Resource {
            deployment: None,
            source: TerraformEntityId {
                address: address.clone(),
                deposed_key: Some("old-key".into()),
            },
            change,
        });
    let mut layout = Layout::new(&graph);
    let index = Index::new(&graph, &layout);
    let cards = index.cards();
    assert_eq!(cards.len(), 3);
    let first = roxmltree::Document::parse(&cards[0].content).unwrap();
    let fifth = first
        .descendants()
        .find(|n| n.attribute("data-relationship-entry") == Some("5"))
        .unwrap();
    let sources: Vec<_> = fifth
        .descendants()
        .filter(|n| n.attribute("data-source-address").is_some())
        .collect();
    assert_eq!(sources.len(), 2);
    let visible: String = sources[1]
        .children()
        .filter(|n| n.has_tag_name("tspan"))
        .filter_map(|n| n.text())
        .collect();
    assert_eq!(visible, format!("{address} (deposed: old-key)"));
    assert!(
        sources[1]
            .children()
            .filter(|n| n.has_tag_name("tspan"))
            .count()
            > 10
    );
    assert!(cards[0].height > cards[1].height);
    assert_eq!(
        first
            .descendants()
            .filter(|n| n.attribute("data-relationship-entry").is_some())
            .count(),
        5
    );
    let second = roxmltree::Document::parse(&cards[1].content).unwrap();
    assert_eq!(
        second
            .descendants()
            .filter_map(|n| n.attribute("data-relationship-entry"))
            .collect::<Vec<_>>(),
        (6..=10)
            .map(|number| number.to_string())
            .collect::<Vec<_>>()
    );
    layout.width = cards[0].width * 2 + 20 + 112;
    let rendered = svg::render(&graph, &layout);
    let document = roxmltree::Document::parse(&rendered).unwrap();
    let section = document
        .descendants()
        .find(|node| node.attribute("data-annotation-section") == Some("Relationship Resources"))
        .unwrap();
    let placed: Vec<_> = section
        .children()
        .filter(|node| node.attribute("data-annotation-card").is_some())
        .map(annotation_card_bounds)
        .collect();
    assert_eq!(placed.len(), 3);
    assert_eq!(placed[0].origin.y, placed[1].origin.y);
    assert_eq!(
        placed[2].origin.y,
        placed[0].origin.y + cards[0].height.max(cards[1].height) + 20
    );
    for (placed, card) in placed.iter().zip(&cards) {
        assert_eq!(placed.height, card.height);
        let document = roxmltree::Document::parse(&card.content).unwrap();
        for line in document
            .descendants()
            .filter(|node| node.has_tag_name("tspan"))
        {
            assert!(attribute_number(line, "y") + 4 < placed.height);
        }
    }
}

#[test]
fn marker_borders_match_relationship_actions_and_line_patterns() {
    use crate::model::Action;
    for kind in [
        EdgeKind::Dependency,
        EdgeKind::Association,
        EdgeKind::Connection,
    ] {
        for action in [
            None,
            Some(Action::Create),
            Some(Action::Delete),
            Some(Action::Update),
            Some(Action::Replace),
            Some(Action::Read),
            Some(Action::Unchanged),
            Some(Action::Other),
        ] {
            let mut graph = graph();
            graph.edges[0].kind = kind;
            if let Some(action) = action {
                graph.edges[0].change.as_mut().unwrap().action = action;
            } else {
                graph.edges[0].change = None;
            }
            for relationship in &mut graph.relationships {
                relationship.kind = kind;
                for source in &mut relationship.provenance {
                    if let RelationshipProvenance::Resource { change, .. } = source {
                        if let Some(action) = action {
                            change.action = action;
                        }
                    }
                }
            }
            let svg = svg::render(&graph, &Layout::new(&graph));
            let document = roxmltree::Document::parse(&svg).unwrap();
            let path = document
                .descendants()
                .find(|n| n.has_tag_name("path") && n.attribute("data-edge-kind").is_some())
                .unwrap();
            let marker = document
                .descendants()
                .find(|n| n.attribute("data-relationship-marker") == Some("1"))
                .unwrap();
            let border = marker
                .descendants()
                .find(|n| n.has_tag_name("rect"))
                .unwrap();
            if kind == EdgeKind::Dependency {
                let arrow_id = path
                    .attribute("marker-end")
                    .unwrap()
                    .strip_prefix("url(#")
                    .unwrap()
                    .strip_suffix(')')
                    .unwrap();
                let arrow = document
                    .descendants()
                    .find(|n| n.attribute("id") == Some(arrow_id))
                    .unwrap();
                assert_eq!(arrow.attribute("markerUnits"), Some("userSpaceOnUse"));
                let arrow_width: f64 = arrow.attribute("markerWidth").unwrap().parse().unwrap();
                let border_width: f64 = border.attribute("stroke-width").unwrap().parse().unwrap();
                assert!(arrow_width + border_width / 2.0 < 10.0);
            } else {
                assert!(path.attribute("marker-end").is_none());
            }
            let entry = document
                .descendants()
                .find(|n| n.attribute("data-relationship-entry") == Some("1"))
                .unwrap();
            let index_border = entry
                .descendants()
                .find(|n| n.has_tag_name("rect"))
                .unwrap();
            for attribute in [
                "stroke",
                "stroke-width",
                "stroke-dasharray",
                "stroke-linecap",
            ] {
                assert_eq!(
                    border.attribute(attribute),
                    path.attribute(attribute),
                    "{kind:?} {action:?}: {attribute}"
                );
                assert_eq!(
                    index_border.attribute(attribute),
                    border.attribute(attribute),
                    "index {kind:?} {action:?}: {attribute}"
                );
            }
        }
    }
}

#[test]
fn markers_stay_on_the_line_near_numbered_ends_in_each_direction() {
    use crate::layout::Point;
    let mut graph = dense_graph(10, EdgeKind::Association);
    for edge in &mut graph.edges {
        edge.change.as_mut().unwrap().directionality = crate::model::Directionality::Directed;
    }
    for relationship in &mut graph.relationships {
        for provenance in &mut relationship.provenance {
            if let RelationshipProvenance::Resource { change, .. } = provenance {
                change.directionality = crate::model::Directionality::Directed;
            }
        }
    }
    for (start, end) in [
        (Point { x: 2000, y: 2000 }, Point { x: 2300, y: 2000 }),
        (Point { x: 2300, y: 2000 }, Point { x: 2000, y: 2000 }),
        (Point { x: 2000, y: 2000 }, Point { x: 2000, y: 2300 }),
        (Point { x: 2000, y: 2300 }, Point { x: 2000, y: 2000 }),
    ] {
        let mut layout = Layout::new(&graph);
        layout.paths.iter_mut().for_each(Vec::clear);
        layout.paths[9] = vec![start, end];
        layout.width = 2600;
        layout.height = 2600;
        let index = Index::new(&graph, &layout);
        assert_eq!(index.entries.len(), 1);
        assert_eq!(index.markers.len(), 2);
        for (marker, port) in index.markers.iter().zip([end, start]) {
            assert_eq!(marker.number, 10);
            let bounds = marker.bounds;
            assert_eq!(bounds.width, 32);
            let center = Point {
                x: bounds.origin.x + bounds.width / 2,
                y: bounds.origin.y + bounds.height / 2,
            };
            assert_eq!(center.x == port.x, start.x == end.x);
            assert_eq!(center.y == port.y, start.y == end.y);
            let distance = center.x.abs_diff(port.x) + center.y.abs_diff(port.y);
            assert_eq!(
                distance,
                if start.x == end.x {
                    bounds.height / 2 + 10
                } else {
                    bounds.width / 2 + 10
                }
            );
        }
        assert_eq!(index.width, layout.width);
    }
}

#[test]
fn routed_badges_fit_terminal_segments_and_avoid_other_paths() {
    for fixture in [
        include_str!("../../fixtures/association-plan.json"),
        include_str!("../../fixtures/aws-relationships-plan.json"),
    ] {
        let graph = semantic::transform(&plan::parse(fixture).unwrap());
        let layout = Layout::new(&graph);
        assert_marker_routes(&graph, &layout);
    }
}

fn assert_marker_routes(graph: &Graph, layout: &Layout<'_>) {
    use crate::layout::{Port, Side, relationship_markers};
    let index = Index::new(graph, layout);
    let shared = relationship_markers::entries(graph);
    for entry in &index.entries {
        assert_eq!(entry.sources, shared[entry.number - 1].sources);
        assert!(
            entry
                .edges
                .iter()
                .all(|edge| shared[entry.number - 1].edges.contains(edge))
        );
    }
    assert!(!index.markers.is_empty());
    let edges = index
        .entries
        .iter()
        .flat_map(|entry| entry.edges.iter())
        .flat_map(|&edge| {
            let mut ends = vec![(edge, false)];
            if graph.edges[edge].kind == EdgeKind::Association
                || graph.edges[edge].directionality() == crate::model::Directionality::Undirected
            {
                ends.push((edge, true));
            }
            ends
        });
    for (marker, (edge, source)) in index.markers.iter().zip(edges) {
        let bounds = marker.bounds;
        let terminal = if source {
            &layout.paths[edge][..2]
        } else {
            layout.paths[edge]
                .windows(2)
                .rev()
                .find(|s| s[0] != s[1])
                .unwrap()
        };
        let (before, target) = if source {
            (terminal[1], terminal[0])
        } else {
            (terminal[0], terminal[1])
        };
        let side = if before.x < target.x {
            Side::Left
        } else if before.x > target.x {
            Side::Right
        } else if before.y < target.y {
            Side::Top
        } else {
            Side::Bottom
        };
        let port = Port {
            point: target,
            side,
        };
        assert_eq!(bounds, relationship_markers::bounds(marker.number, port));
        let reservation = relationship_markers::Terminal::marker(marker.number, side)
            .corridor(port)
            .unwrap();
        assert!(bounds.origin.x >= reservation.origin.x && bounds.right() <= reservation.right());
        assert!(
            bounds.origin.y >= reservation.origin.y
                && bounds.origin.y + bounds.height <= reservation.origin.y + reservation.height
        );
        if before.y == target.y {
            assert_eq!(bounds.origin.y + bounds.height / 2, target.y);
            assert!(
                bounds.origin.x >= before.x.min(target.x)
                    && bounds.right() <= before.x.max(target.x),
                "marker {}: {bounds:?}, terminal {terminal:?}",
                marker.number
            );
        } else {
            assert_eq!(bounds.origin.x + bounds.width / 2, target.x);
            assert!(
                bounds.origin.y >= before.y.min(target.y)
                    && bounds.origin.y + bounds.height <= before.y.max(target.y)
            );
        }
        for (other, path) in layout.paths.iter().enumerate() {
            let segments = path.windows(2).count();
            for (position, segment) in path.windows(2).enumerate() {
                if other == edge
                    && ((source && position == 0) || (!source && position + 1 == segments))
                {
                    continue;
                }
                let (a, b) = (segment[0], segment[1]);
                let crosses = if a.x == b.x {
                    a.x > bounds.origin.x.saturating_sub(2)
                        && a.x < bounds.right() + 2
                        && a.y.max(b.y) > bounds.origin.y.saturating_sub(2)
                        && a.y.min(b.y) < bounds.origin.y + bounds.height + 2
                } else {
                    a.y > bounds.origin.y.saturating_sub(2)
                        && a.y < bounds.origin.y + bounds.height + 2
                        && a.x.max(b.x) > bounds.origin.x.saturating_sub(2)
                        && a.x.min(b.x) < bounds.right() + 2
                };
                assert!(
                    !crosses,
                    "marker {} on edge {edge}: {bounds:?} crosses edge {other} segment {segment:?}",
                    marker.number
                );
            }
        }
    }
}

#[test]
fn missing_paths_keep_the_shared_number_and_its_actual_badge_width() {
    let graph = dense_graph(10, EdgeKind::Association);
    let mut layout = Layout::new(&graph);
    let shared = crate::layout::relationship_markers::entries(&graph);
    for entry in &shared[..9] {
        for &edge in &entry.edges {
            layout.paths[edge].clear();
        }
    }
    let index = Index::new(&graph, &layout);
    assert_eq!(index.entries.len(), 1);
    assert_eq!(index.entries[0].number, 10);
    assert!(
        index
            .markers
            .iter()
            .all(|marker| marker.number == 10 && marker.bounds.width == 32)
    );
    assert!(
        index.cards()[0]
            .content
            .contains("data-relationship-entry=\"10\"")
    );
    assert_marker_routes(&graph, &layout);
}

#[test]
fn embedded_next_hops_share_the_routed_numbering_and_marker_geometry() {
    let graph =
        semantic::transform(&plan::parse(include_str!("../../fixtures/routes-plan.json")).unwrap())
            .0;
    let layout = Layout::new(&graph);
    let index = Index::new(&graph, &layout);
    let svg = svg::render(&graph, &layout);
    let document = roxmltree::Document::parse(&svg).unwrap();
    assert!(index.entries.iter().any(|entry| entry.embedded));
    for entry in index.entries.iter().filter(|entry| entry.embedded) {
        assert_eq!(
            index
                .markers
                .iter()
                .filter(|marker| marker.number == entry.number)
                .count(),
            entry.edges.len() * 2
        );
        let number = entry.number.to_string();
        assert_eq!(
            document
                .descendants()
                .filter(|node| node.attribute("data-relationship-entry") == Some(number.as_str()))
                .count(),
            1
        );
        assert_eq!(
            document
                .descendants()
                .filter(|node| node.attribute("data-relationship-marker") == Some(number.as_str()))
                .count(),
            entry.edges.len() * 2
        );
        for &edge in &entry.edges {
            let route = &graph.nodes[graph.edges[edge].from];
            let card = document
                .descendants()
                .find(|node| {
                    node.attribute("data-embedded-route") == Some("true")
                        && node.attribute("data-terraform-address") == Some(route.address.as_str())
                })
                .unwrap();
            let label = format!("Next hop [{}]", entry.number);
            assert!(
                card.descendants()
                    .any(|node| node.text() == Some(label.as_str()))
            );
            assert_eq!(entry.sources.len(), 1);
            assert!(route.entity.provenance.contains(entry.sources[0]));
        }
    }
    assert_marker_routes(&graph, &layout);
}

#[test]
fn resource_provenance_is_indexed_and_reference_provenance_is_not() {
    let graph = graph();
    let layout = Layout::new(&graph);
    let index = Index::new(&graph, &layout);
    assert_eq!(index.entries.len(), 1);
    assert_eq!(index.markers.len(), 2);
    let svg = svg::render(&graph, &layout);
    assert!(svg.contains("data-relationship-index="));
    assert_eq!(svg.matches(">Relationship Resources</text>").count(), 1);
    assert!(svg.contains("data-relationship-marker=\"1\""));
    assert!(svg.contains("data-relationship-entry=\"1\""));
    assert!(svg.contains(&format!(
        "data-source-address=\"{}\"",
        index.entries[0].sources[0].address
    )));
    let mut inferred = graph.clone();
    inferred.relationships.iter_mut().for_each(|r| {
        r.provenance
            .retain(|p| matches!(p, RelationshipProvenance::Reference { .. }))
    });
    let empty = Index::new(&inferred, &layout);
    assert!(empty.entries.is_empty());
    assert!(empty.cards().is_empty());
    assert!(!svg::render(&inferred, &layout).contains("data-relationship-index="));
    assert_eq!(
        svg::dimensions(&graph, &layout).1 - svg::dimensions(&inferred, &layout).1,
        index.cards()[0].height + 48 + 16 + 24
    );
}

#[test]
fn multiple_sources_long_addresses_and_xml_are_preserved_without_values() {
    let mut graph = graph();
    let first_previous = "aws_route_table_association.old[0]";
    for edge in &mut graph.edges {
        if let Some(change) = &mut edge.change {
            change.previous_address = Some(first_previous.into());
        }
    }
    let relationship = graph
        .relationships
        .iter_mut()
        .find(|r| {
            r.provenance
                .iter()
                .any(|p| matches!(p, RelationshipProvenance::Resource { .. }))
        })
        .unwrap();
    for provenance in &mut relationship.provenance {
        if let RelationshipProvenance::Resource { change, .. } = provenance {
            change.previous_address = Some(first_previous.into());
        }
    }
    let RelationshipProvenance::Resource { mut change, .. } = relationship.provenance[0].clone()
    else {
        panic!()
    };
    let address = format!(
        "module.{}.aws_route_table_association.extra[\"<&>\"]",
        "nested.".repeat(70)
    );
    let previous = format!(
        "module.{}.aws_route_table_association.old[\"<&>\"]",
        "以前の名前.".repeat(50)
    );
    change.previous_address = Some(previous.clone());
    relationship
        .provenance
        .push(RelationshipProvenance::Resource {
            deployment: None,
            source: TerraformEntityId {
                address: address.clone(),
                deposed_key: Some("old<&>".into()),
            },
            change,
        });
    let layout = Layout::new(&graph);
    let index = Index::new(&graph, &layout);
    assert_eq!(index.entries[0].sources.len(), 2);
    let svg = svg::render(&graph, &layout);
    let document = roxmltree::Document::parse(&svg).unwrap();
    let source = document
        .descendants()
        .find(|n| n.attribute("data-source-address") == Some(&address))
        .unwrap();
    let visible: String = source
        .children()
        .filter(|n| n.has_tag_name("tspan"))
        .filter_map(|n| n.text())
        .collect();
    assert_eq!(visible, format!("{address} (deposed: old<&>)"));
    assert!(svg.contains("2 Terraform resources"));
    let height: usize = document
        .root_element()
        .attribute("height")
        .unwrap()
        .parse()
        .unwrap();
    for node in source.children().filter(|n| n.has_tag_name("tspan")) {
        assert!(node.attribute("y").unwrap().parse::<usize>().unwrap() < height);
    }
    let card = &index.cards()[0];
    let doc = roxmltree::Document::parse(&card.content).unwrap();
    let entry = doc
        .descendants()
        .find(|n| n.attribute("data-relationship-entry") == Some("1"))
        .unwrap();
    let texts: Vec<_> = entry
        .children()
        .filter(|n| n.has_tag_name("text"))
        .collect();
    for source in texts
        .iter()
        .filter(|n| n.attribute("data-source-address").is_some())
    {
        let position = texts.iter().position(|n| n == source).unwrap();
        let annotation = texts[position + 1];
        assert_eq!(annotation.attribute("data-move-annotation"), Some("true"));
        let expected = if source.attribute("data-source-address") == Some(address.as_str()) {
            &previous
        } else {
            first_previous
        };
        let lines: Vec<_> = annotation
            .children()
            .filter(|n| n.has_tag_name("tspan"))
            .collect();
        assert_eq!(
            lines.iter().filter_map(|n| n.text()).collect::<String>(),
            format!("Moved from: {expected}")
        );
        assert_eq!(
            annotation
                .children()
                .find(|n| n.has_tag_name("title"))
                .unwrap()
                .text(),
            Some(format!("Moved from: {expected}").as_str())
        );
        for line in lines {
            let width = crate::view::text::width(line.text().unwrap()) * 8;
            assert!(
                line.attribute("x").unwrap().parse::<usize>().unwrap() + width <= card.width - 16
            );
            assert!(line.attribute("y").unwrap().parse::<usize>().unwrap() + 4 < card.height);
        }
    }
    let expected = card.content.clone();
    for relationship in &mut graph.relationships {
        relationship
            .provenance
            .extend(relationship.provenance.clone());
        relationship.provenance.reverse();
    }
    graph.relationships.reverse();
    assert_eq!(
        Index::new(&graph, &Layout::new(&graph)).cards()[0].content,
        expected
    );
}

#[test]
fn numbering_comes_from_sources_and_survives_reordered_edges_and_provenance() {
    let raw = plan::parse(include_str!("../../fixtures/aws-relationships-plan.json")).unwrap();
    let mut graph = semantic::transform(&raw).0;
    let keys = |graph: &Graph| {
        let layout = Layout::new(graph);
        Index::new(graph, &layout)
            .entries
            .iter()
            .map(|e| {
                e.sources
                    .iter()
                    .map(|s| s.address.clone())
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>()
    };
    let expected = keys(&graph);
    assert!(expected.len() > 1);
    assert!(expected.windows(2).all(|w| w[0] <= w[1]));
    graph.edges.reverse();
    graph.relationships.reverse();
    for relationship in &mut graph.relationships {
        relationship.provenance.reverse();
    }
    assert_eq!(keys(&graph), expected);
}

#[test]
fn marker_boxes_avoid_cards_boundaries_junctions_and_paths() {
    let raw = plan::parse(include_str!("../../fixtures/aws-relationships-plan.json")).unwrap();
    let graph = semantic::transform(&raw);
    let layout = Layout::new(&graph);
    let index = Index::new(&graph, &layout);
    assert!(!index.markers.is_empty());
    for marker in &index.markers {
        for (i, bounds) in layout.bounds.iter().enumerate() {
            let header = bounds.header(layout.header_heights[i]);
            assert!(
                marker.bounds.right() <= header.origin.x
                    || marker.bounds.origin.x >= header.right()
                    || marker.bounds.origin.y + marker.bounds.height <= header.origin.y
                    || marker.bounds.origin.y >= header.origin.y + header.height
            );
        }
    }
}

#[test]
fn regional_markers_keep_clear_of_scope_headers_and_frame_edges() {
    let mut value: serde_json::Value =
        serde_json::from_str(include_str!("../../fixtures/association-plan.json")).unwrap();
    value["configuration"]["provider_config"] = serde_json::json!({"aws":{"full_name":"hashicorp/aws","expressions":{"region":{"constant_value":"ap-northeast-1"}}}});
    for resource in value["configuration"]["root_module"]["resources"]
        .as_array_mut()
        .unwrap()
    {
        resource["provider_config_key"] = serde_json::json!("aws");
    }
    let graph = semantic::transform(&plan::parse(&value.to_string()).unwrap());
    let layout = Layout::new(&graph);
    assert_eq!(layout.scopes.len(), 1);
    let index = Index::new(&graph, &layout);
    assert_eq!(index.markers.len(), 2);
    let panel = layout.scopes[0].bounds;
    let marker = index.markers[0].bounds;
    assert!(
        marker.origin.y >= panel.origin.y + 36
            || marker.right() < panel.origin.x
            || marker.origin.x > panel.right()
    );
    for x in [panel.origin.x, panel.right()] {
        assert!(
            x < marker.origin.x
                || x > marker.right()
                || marker.origin.y > panel.origin.y + panel.height
                || marker.origin.y + marker.height < panel.origin.y
        );
    }
}
