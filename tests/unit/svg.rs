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
        assert!(text.contains("usually undirected"));
        assert!(
            text.contains("route next-hop association is directed from the route to its target")
        );
        assert!(text.contains("does not guarantee actual network reachability"));
        assert!(text.contains("nesting replaces a redundant relationship line"));
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
            assert!(x > 0 && x + text_width <= width, "{}", text.text().unwrap());
            assert!(y > 12 && y + 4 < height);
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
