use super::*;

#[test]
fn relationship_legend_explains_colors_and_omits_line_free_graphs() {
    let raw = crate::plan::parse(include_str!("../fixtures/association-plan.json")).unwrap();
    let mut graph = crate::semantic::transform(&raw);
    for action in [
        Action::Create,
        Action::Update,
        Action::Delete,
        Action::Replace,
        Action::Read,
        Action::Unchanged,
        Action::Other,
    ] {
        graph.0.edges[0].change.as_mut().unwrap().action = action;
        let legend = legend::Legend::new(&graph, 1120);
        let document = roxmltree::Document::parse(legend.render()).unwrap();
        let swatch = document
            .descendants()
            .find(|node| node.attribute("data-legend-action") == Some(label(action)))
            .unwrap();
        assert_eq!(swatch.attribute("stroke"), Some(color(action).1));
        assert_eq!(
            document
                .descendants()
                .filter_map(|node| node.attribute("data-legend-action"))
                .collect::<std::collections::BTreeSet<_>>(),
            ["none", "unchanged", label(action)]
                .into_iter()
                .collect::<std::collections::BTreeSet<_>>()
        );
        let text = document
            .descendants()
            .filter(|node| node.is_text())
            .filter_map(|node| node.text())
            .collect::<Vec<_>>()
            .join(" ");
        let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
        assert!(text.contains("independently of endpoint-card actions"));
        assert!(text.contains("no separate Terraform action is identified"));
        assert!(text.contains("unchanged: an explicit no-op action"));
        assert!(text.contains("from the referenced resource to the dependent resource"));
        assert!(text.contains("not exact Terraform execution order"));
        assert!(text.contains("Dashed, undirected relationship"));
        assert!(text.contains("from a Route to its next hop"));
        assert!(text.contains("destination arrow expresses direction"));
        assert!(text.contains("does not guarantee actual network reachability"));
        assert!(text.contains("nesting replaces a redundant relationship line"));
        assert!(text.contains("Hollow circles mark resource/card attachment points"));
        assert!(text.contains("neither end of an undirected line is a directional start"));
        assert!(text.contains("Filled internal dots mark bundle junctions"));
        assert!(
            text.contains("Matching [N] badges identify the same Terraform-backed relationship")
        );
        assert!(text.contains("Relationship Resources entry"));
        assert!(text.contains("They do not express direction or count connections"));
        assert!(text.contains("Reference-only Connections and Dependencies have no number"));
        assert_eq!(
            document
                .descendants()
                .filter(|node| node.attribute("data-legend-entry").is_some())
                .count(),
            5
        );
        for (key, directed, circle_count, numbered) in [
            ("dependency", true, 1, false),
            ("association", false, 2, true),
            ("directed-association", true, 2, true),
            ("connection", false, 2, false),
        ] {
            let sample = document
                .descendants()
                .find(|node| node.attribute("data-legend-sample") == Some(key))
                .unwrap();
            let line = sample
                .children()
                .find(|node| node.attribute("data-legend-kind") == Some(key))
                .unwrap();
            assert_eq!(
                line.attribute("marker-end"),
                directed.then_some("url(#arrow)")
            );
            assert_eq!(
                line.attribute("data-directionality"),
                Some(if directed { "directed" } else { "undirected" })
            );
            let circles: Vec<_> = sample
                .children()
                .filter(|node| node.attribute("data-legend-endpoint").is_some())
                .collect();
            assert_eq!(circles.len(), circle_count);
            for circle in &circles {
                assert_eq!(circle.attribute("r"), Some("3.5"));
                assert_eq!(circle.attribute("fill"), Some("#ffffff"));
                assert_eq!(circle.attribute("stroke"), line.attribute("stroke"));
                assert_eq!(circle.attribute("stroke-width"), Some("1.5"));
                assert!(circle.range().start > line.range().end);
            }
            let arrow = sample
                .children()
                .find(|node| node.attribute("data-legend-arrow").is_some());
            assert_eq!(arrow.is_some(), directed && circle_count == 2);
            if let Some(arrow) = arrow {
                assert!(arrow.range().start >= circles[1].range().end);
                assert_eq!(arrow.attribute("fill"), line.attribute("stroke"));
            }
            let badges: Vec<_> = sample
                .children()
                .filter(|node| node.attribute("data-legend-badge").is_some())
                .collect();
            assert_eq!(badges.len(), if numbered { 2 } else { 0 });
            for (index, badge) in badges.iter().enumerate() {
                assert_eq!(
                    badge.attribute("data-legend-badge"),
                    Some(if index == 0 { "source" } else { "target" })
                );
                let rectangle = badge.children().find(|n| n.has_tag_name("rect")).unwrap();
                for attribute in ["stroke", "stroke-width", "stroke-dasharray"] {
                    assert_eq!(rectangle.attribute(attribute), line.attribute(attribute));
                }
                let label = badge.children().find(|n| n.has_tag_name("text")).unwrap();
                assert_eq!(label.text(), Some("[12]"));
                assert!(badge.range().start >= circles.last().unwrap().range().end);
                if let Some(arrow) = arrow {
                    assert!(badge.range().start > arrow.range().end);
                }
            }
        }
        assert!(document.descendants().all(|node| {
            node.attribute("data-endpoint").is_none()
                && node.attribute("data-relationship-marker").is_none()
                && node.attribute("data-relationship-entry").is_none()
        }));
        for (label, stroke) in [("none", "#94a3b8"), ("unchanged", "#64748b")] {
            let swatch = document
                .descendants()
                .find(|node| node.attribute("data-legend-action") == Some(label))
                .unwrap();
            assert_eq!(swatch.attribute("stroke"), Some(stroke));
        }
        assert_eq!(legend.render(), legend::Legend::new(&graph, 1120).render());
    }
    graph.0.edges[0].change = None;
    assert!(render(&graph, &Layout::new(&graph)).contains("no independent change"));
    graph.0.edges[0].kind = EdgeKind::Containment;
    for empty in [false, true] {
        if empty {
            graph.0.edges.clear();
        }
        let legend = legend::Legend::new(&graph, 400);
        assert_eq!(legend.width, 400);
        assert_eq!(legend.height, 0);
        assert!(legend.render().is_empty());
        assert!(!render(&graph, &Layout::new(&graph)).contains("id=\"relationship-legend\""));
    }
}

#[test]
fn relationship_legend_wraps_actions_and_text_inside_its_measured_bounds() {
    let raw = crate::plan::parse(include_str!("../fixtures/association-plan.json")).unwrap();
    let mut graph = crate::semantic::transform(&raw);
    let template = graph.0.edges[0].clone();
    graph.0.edges = [
        Action::Create,
        Action::Update,
        Action::Delete,
        Action::Replace,
        Action::Read,
        Action::Unchanged,
        Action::Other,
    ]
    .into_iter()
    .map(|action| {
        let mut edge = template.clone();
        edge.change.as_mut().unwrap().action = action;
        edge
    })
    .collect();
    let mut heights = Vec::new();
    let mut action_rows = Vec::new();
    for available_width in [240, 400, 720, 1120, 2200] {
        let legend = legend::Legend::new(&graph, available_width);
        let document = roxmltree::Document::parse(legend.render()).unwrap();
        let root = document.root_element();
        let rectangle = root.children().find(|n| n.has_tag_name("rect")).unwrap();
        let number =
            |node: roxmltree::Node, name| node.attribute(name).unwrap().parse::<usize>().unwrap();
        let width = number(rectangle, "width");
        let height = number(rectangle, "height");
        assert!(legend.width >= available_width);
        assert!(40 + width <= legend.width);
        assert!(legend.height > height);
        heights.push(legend.height);
        let actions: Vec<_> = root
            .descendants()
            .filter(|n| n.attribute("data-legend-action").is_some())
            .collect();
        assert_eq!(actions.len(), 8);
        action_rows.push(
            actions
                .iter()
                .map(|n| {
                    n.attribute("d")
                        .unwrap()
                        .split_whitespace()
                        .nth(2)
                        .unwrap()
                        .parse::<usize>()
                        .unwrap()
                })
                .collect::<std::collections::BTreeSet<_>>()
                .len(),
        );
        for text in root.descendants().filter(|n| n.has_tag_name("text")) {
            let x = number(text, "x");
            let y = number(text, "y");
            let text_width = crate::view::text::width(text.text().unwrap()) * 8;
            let left = if text.attribute("text-anchor") == Some("middle") {
                x.checked_sub(text_width / 2).unwrap()
            } else {
                x
            };
            assert!(
                left > 0 && left + text_width <= width,
                "{}",
                text.text().unwrap()
            );
            assert!(y > 12 && y + 4 < height);
        }
        let coordinate =
            |node: roxmltree::Node, name| node.attribute(name).unwrap().parse::<f64>().unwrap();
        let mut preceding_bottom = 0.0_f64;
        for entry in root
            .children()
            .filter(|node| node.attribute("data-legend-entry").is_some())
        {
            let sample = entry
                .children()
                .find(|node| node.attribute("data-legend-sample").is_some())
                .unwrap();
            let mut sample_bounds = [f64::MAX, f64::MAX, 0.0_f64, 0.0_f64];
            for symbol in sample.descendants().filter(|node| {
                node.has_tag_name("path")
                    || node.has_tag_name("circle")
                    || node.has_tag_name("rect")
            }) {
                let stroke = symbol
                    .attribute("stroke-width")
                    .unwrap_or("1")
                    .parse::<f64>()
                    .unwrap()
                    / 2.0;
                let bounds = if symbol.has_tag_name("circle") {
                    let radius = coordinate(symbol, "r") + stroke;
                    let x = coordinate(symbol, "cx");
                    let y = coordinate(symbol, "cy");
                    [x - radius, y - radius, x + radius, y + radius]
                } else if symbol.has_tag_name("rect") {
                    let x = coordinate(symbol, "x");
                    let y = coordinate(symbol, "y");
                    [
                        x - stroke,
                        y - stroke,
                        x + coordinate(symbol, "width") + stroke,
                        y + coordinate(symbol, "height") + stroke,
                    ]
                } else {
                    let values: Vec<f64> = symbol
                        .attribute("d")
                        .unwrap()
                        .split_whitespace()
                        .filter_map(|part| part.parse().ok())
                        .collect();
                    if values.len() == 3 {
                        let arrow = symbol.attribute("marker-end").is_some();
                        let vertical = if arrow { 3.5 } else { stroke };
                        [
                            values[0] - stroke,
                            values[1] - vertical,
                            values[2] + if arrow { 0.7 } else { stroke },
                            values[1] + vertical,
                        ]
                    } else {
                        let mut bounds = [f64::MAX, f64::MAX, 0.0_f64, 0.0_f64];
                        for point in values.chunks_exact(2) {
                            bounds[0] = bounds[0].min(point[0]);
                            bounds[1] = bounds[1].min(point[1]);
                            bounds[2] = bounds[2].max(point[0]);
                            bounds[3] = bounds[3].max(point[1]);
                        }
                        bounds
                    }
                };
                assert!(bounds[0] >= 16.0 && bounds[2] <= width as f64 - 16.0);
                assert!(bounds[1] > preceding_bottom && bounds[3] < height as f64 - 16.0);
                sample_bounds[0] = sample_bounds[0].min(bounds[0]);
                sample_bounds[1] = sample_bounds[1].min(bounds[1]);
                sample_bounds[2] = sample_bounds[2].max(bounds[2]);
                sample_bounds[3] = sample_bounds[3].max(bounds[3]);
            }
            for text in entry.children().filter(|node| node.has_tag_name("text")) {
                let x = coordinate(text, "x");
                let y = coordinate(text, "y");
                assert!(x > sample_bounds[2] || y - 12.0 > sample_bounds[3]);
                preceding_bottom = preceding_bottom.max(y + 4.0);
            }
            preceding_bottom = preceding_bottom.max(sample_bounds[3]);
        }
        let containment = root
            .descendants()
            .find(|n| n.attribute("data-legend-kind") == Some("containment"))
            .unwrap();
        let boxes: Vec<_> = containment
            .children()
            .filter(|n| n.has_tag_name("rect"))
            .collect();
        assert_eq!(boxes.len(), 2);
        assert!(!containment.descendants().any(|n| n.has_tag_name("path")));
        for axis in [("x", "width"), ("y", "height")] {
            assert!(number(boxes[1], axis.0) > number(boxes[0], axis.0));
            assert!(
                number(boxes[1], axis.0) + number(boxes[1], axis.1)
                    < number(boxes[0], axis.0) + number(boxes[0], axis.1)
            );
        }
        graph.0.edges.reverse();
        assert_eq!(
            legend.render(),
            legend::Legend::new(&graph, available_width).render()
        );
    }
    assert!(heights[0] > heights[3]);
    assert!(action_rows[0] > action_rows[3]);
    assert_eq!(heights[3], heights[4]);
}

#[test]
fn policy_and_controller_fallbacks_keep_resources_actions_and_dependencies() {
    let raw = crate::plan::parse(r#"{"format_version":"1.2","resource_changes":[
        {"address":"aws_security_group.app","type":"aws_security_group","change":{"actions":["update"]}},
        {"address":"aws_ecs_service.app","type":"aws_ecs_service","change":{"actions":["create"]}}
    ],"configuration":{"root_module":{"resources":[
        {"address":"aws_security_group.app"},
        {"address":"aws_ecs_service.app","expressions":{"security_groups":{"references":["aws_security_group.app.id"]}}}
    ]}}}"#).unwrap();
    let graph = crate::semantic::transform(&raw);
    assert_eq!(graph.0, crate::semantic::base_graph(&raw));
    let output = render(&graph, &Layout::new(&graph));
    for value in [
        "data-role=\"policy\"",
        "data-role=\"controller\"",
        "aws_security_group.app",
        "aws_ecs_service.app",
        "update (1)",
        "create (1)",
    ] {
        assert!(output.contains(value), "missing {value}");
    }
    assert_eq!(
        output
            .lines()
            .filter(|line| line.contains("data-edge-kind=") && line.contains("marker-end="))
            .count(),
        1
    );
    assert_eq!(output, render(&graph, &Layout::new(&graph)));
}

#[test]
fn mixed_relationships_identify_each_kind_without_dependency_only_captions() {
    let raw = crate::plan::parse(include_str!("../fixtures/association-plan.json")).unwrap();
    let mut graph = crate::semantic::transform(&raw);
    graph.0.edges.push(crate::model::Edge::from((0, 1)));
    let output = render(&graph, &Layout::new(&graph));
    assert!(output.contains(" (dependency)</title>"));
    assert!(output.contains(" (association; create:"));
    assert!(output.contains("3 resources (2 cards), 2 relationships"));
    assert!(!output.contains("reference edges"));
    assert!(!output.contains("Arrows point from dependencies"));
}

#[test]
fn escapes_markup_and_filters_invalid_control_characters() {
    assert_eq!(
        escape("<script>&\"'\u{0}\u{1}\n\r\t"),
        "&lt;script&gt;&amp;&quot;&apos;\n\r\t"
    );
}
