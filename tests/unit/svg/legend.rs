use super::*;
use crate::layout::Layout;
use crate::svg::{color, label, render};

#[test]
fn relationship_legend_explains_colors_and_omits_line_free_graphs() {
    let raw = crate::plan::parse(include_str!("../../fixtures/association-plan.json")).unwrap();
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
        let legend = Legend::new(&graph, 1120);
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
        assert_eq!(legend.render(), Legend::new(&graph, 1120).render());
    }
    graph.0.edges[0].change = None;
    assert!(render(&graph, &Layout::new(&graph)).contains("no independent change"));
    graph.0.edges[0].kind = EdgeKind::Containment;
    for empty in [false, true] {
        if empty {
            graph.0.edges.clear();
        }
        let legend = Legend::new(&graph, 400);
        assert_eq!(legend.width, 400);
        assert_eq!(legend.height, 0);
        assert!(legend.render().is_empty());
        assert!(!render(&graph, &Layout::new(&graph)).contains("id=\"relationship-legend\""));
    }
}

#[test]
fn relationship_legend_wraps_actions_and_text_inside_its_measured_bounds() {
    let raw = crate::plan::parse(include_str!("../../fixtures/association-plan.json")).unwrap();
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
    let mut heights = std::collections::BTreeMap::new();
    let mut action_rows = std::collections::BTreeMap::new();
    for (available_width, columns) in [
        (240, 1),
        (400, 1),
        (720, 1),
        (899, 1),
        (900, 2),
        (1120, 2),
        (1321, 2),
        (1322, 3),
        (1743, 3),
        (1744, 4),
        (2165, 4),
        (2166, 5),
        (2200, 5),
        (4306, 5),
        (12000, 5),
    ] {
        let legend = Legend::new(&graph, available_width);
        let document = roxmltree::Document::parse(legend.render()).unwrap();
        let root = document.root_element();
        let rectangle = root.children().find(|n| n.has_tag_name("rect")).unwrap();
        let width = number(rectangle, "width");
        let height = number(rectangle, "height");
        assert_eq!(legend.width, available_width.max(360));
        assert_eq!(width + 80, legend.width);
        assert_eq!(legend.height, height + 12);
        heights.insert(available_width, legend.height);
        let actions: Vec<_> = root
            .descendants()
            .filter(|n| n.attribute("data-legend-action").is_some())
            .collect();
        assert_eq!(actions.len(), 8);
        action_rows.insert(
            available_width,
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
            let bounds = drawing_bounds(text);
            assert!(
                bounds[0] > 0.0 && bounds[2] <= width as f64,
                "{}",
                text.text().unwrap()
            );
            assert!(bounds[1] > 0.0 && bounds[3] < height as f64);
        }
        let entries: Vec<_> = root
            .children()
            .filter(|node| node.attribute("data-legend-entry").is_some())
            .collect();
        assert_eq!(
            entries
                .iter()
                .filter_map(|entry| entry.attribute("data-legend-entry"))
                .collect::<Vec<_>>(),
            [
                "dependency",
                "association",
                "directed-association",
                "connection",
                "containment",
            ]
        );
        let positions: Vec<_> = entries.iter().map(|entry| position(*entry)).collect();
        assert_eq!(
            positions
                .iter()
                .filter(|point| point.y == positions[0].y)
                .count(),
            columns
        );
        let cell_width = if columns > 1 {
            positions[1].x - positions[0].x - COLUMN_GAP
        } else {
            width.min(MAX_ENTRY_WIDTH)
        };
        assert!(cell_width <= MAX_ENTRY_WIDTH);
        if columns > 1 {
            assert!(cell_width >= 398);
        }
        let mut entry_bounds = Vec::<[f64; 4]>::new();
        for (index, entry) in entries.iter().enumerate() {
            let origin = positions[index];
            assert_eq!(origin.x, index % columns * (cell_width + COLUMN_GAP));
            assert_eq!(origin.y, positions[index / columns * columns].y);
            if index >= columns {
                assert!(origin.y > positions[index - columns].y);
            }
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
                let bounds = drawing_bounds(symbol);
                assert!(bounds[0] >= (origin.x + PADDING) as f64);
                assert!(bounds[2] <= (origin.x + cell_width - PADDING) as f64);
                assert!(bounds[1] >= origin.y as f64 && bounds[3] < (height - PADDING) as f64);
                sample_bounds = union(sample_bounds, bounds);
            }
            let mut occupied = sample_bounds;
            for text in entry.children().filter(|node| node.has_tag_name("text")) {
                let bounds = drawing_bounds(text);
                assert!(bounds[0] > sample_bounds[2] || bounds[1] > sample_bounds[3]);
                assert!(bounds[0] >= (origin.x + PADDING) as f64);
                assert!(bounds[2] <= (origin.x + cell_width - PADDING) as f64);
                occupied = union(occupied, bounds);
            }
            for previous in &entry_bounds {
                assert!(
                    occupied[0] > previous[2]
                        || previous[0] > occupied[2]
                        || occupied[1] > previous[3]
                        || previous[1] > occupied[3]
                );
            }
            entry_bounds.push(occupied);
        }
        let entry_top = entry_bounds
            .iter()
            .map(|bounds| bounds[1])
            .fold(f64::MAX, f64::min);
        let entry_bottom = entry_bounds
            .iter()
            .map(|bounds| bounds[3])
            .fold(0.0, f64::max);
        assert!(actions.iter().all(|action| drawing_bounds(*action)[3] < entry_top));
        for text in root.children().filter(|node| node.has_tag_name("text")) {
            let bounds = drawing_bounds(text);
            assert!(bounds[2] <= (width.min(MAX_PROSE_WIDTH) - PADDING) as f64);
            if text.range().end < entries[0].range().start {
                assert!(bounds[3] < entry_top);
            } else {
                assert!(bounds[1] > entry_bottom);
            }
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
            Legend::new(&graph, available_width).render()
        );
    }
    assert!(heights[&720] > heights[&4306]);
    assert!(action_rows[&240] > action_rows[&1120]);
    assert_eq!(heights[&4306], heights[&12000]);
}

#[test]
fn grid_rows_measure_long_explanations_and_use_the_tallest_entry() {
    let long = format!(
        "{} {} リソースの関連を省略せずに表示する。",
        "A long explanation keeps every attachment and relationship clear. ".repeat(12),
        "unbroken_description_".repeat(12),
    );
    let entries = [
        Entry {
            explanation: "Short.",
            ..ENTRIES[0]
        },
        Entry {
            explanation: &long,
            ..ENTRIES[1]
        },
        ENTRIES[2],
        ENTRIES[3],
        ENTRIES[4],
    ];
    let mut grid = Content {
        width: 820,
        y: 0,
        svg: String::new(),
    };
    grid.grid(&entries);
    let svg = format!("<g>{}</g>", grid.svg);
    let document = roxmltree::Document::parse(&svg).unwrap();
    let cells: Vec<_> = document
        .descendants()
        .filter(|node| node.attribute("data-legend-entry").is_some())
        .collect();
    let last_baseline = |cell: roxmltree::Node| {
        cell.children()
            .filter(|node| node.has_tag_name("text"))
            .map(|text| number(text, "y"))
            .max()
            .unwrap()
    };
    assert!(last_baseline(cells[1]) > last_baseline(cells[0]) + LINE_HEIGHT);
    assert_eq!(position(cells[0]), Point { x: 0, y: 0 });
    assert_eq!(position(cells[1]), Point { x: 422, y: 0 });
    let second_row = last_baseline(cells[1]) + LINE_HEIGHT - 12 + ROW_GAP;
    assert_eq!(
        position(cells[2]),
        Point {
            x: 0,
            y: second_row,
        }
    );
    assert_eq!(
        position(cells[3]),
        Point {
            x: 422,
            y: second_row,
        }
    );
    let third_row = second_row
        + last_baseline(cells[2]).max(last_baseline(cells[3]))
        + LINE_HEIGHT
        - 12
        + ROW_GAP;
    assert_eq!(
        position(cells[4]),
        Point {
            x: 0,
            y: third_row,
        }
    );
    assert_eq!(
        grid.y,
        third_row + last_baseline(cells[4]) + LINE_HEIGHT - 12 + ROW_GAP
    );
    let rendered: String = cells[1]
        .children()
        .filter(|node| node.has_tag_name("text") && node.attribute("font-weight").is_none())
        .filter_map(|node| node.text())
        .flat_map(str::chars)
        .filter(|character| !character.is_whitespace())
        .collect();
    assert_eq!(
        rendered,
        long.chars().filter(|c| !c.is_whitespace()).collect::<String>()
    );
    for text in cells[1].children().filter(|node| node.has_tag_name("text")) {
        let bounds = drawing_bounds(text);
        assert!(bounds[2] <= 820.0 - PADDING as f64);
        assert!(bounds[3] < second_row as f64);
    }
    let mut single = Content {
        width: MAX_ENTRY_WIDTH,
        y: 0,
        svg: String::new(),
    };
    single.grid(&ENTRIES);
    let mut wide = Content {
        width: ENTRIES.len() * MAX_ENTRY_WIDTH + (ENTRIES.len() - 1) * COLUMN_GAP,
        y: 0,
        svg: String::new(),
    };
    wide.grid(&ENTRIES);
    assert!(wide.y < single.y);
}

fn number(node: roxmltree::Node<'_, '_>, name: &str) -> usize {
    node.attribute(name).unwrap().parse().unwrap()
}

fn position(node: roxmltree::Node<'_, '_>) -> Point {
    let mut values = node
        .attribute("transform")
        .unwrap()
        .strip_prefix("translate(")
        .unwrap()
        .trim_end_matches(')')
        .split_whitespace()
        .map(|value| value.parse().unwrap());
    Point {
        x: values.next().unwrap(),
        y: values.next().unwrap(),
    }
}

fn union(a: [f64; 4], b: [f64; 4]) -> [f64; 4] {
    [
        a[0].min(b[0]),
        a[1].min(b[1]),
        a[2].max(b[2]),
        a[3].max(b[3]),
    ]
}

fn drawing_bounds(node: roxmltree::Node<'_, '_>) -> [f64; 4] {
    let coordinate = |name| node.attribute(name).unwrap().parse::<f64>().unwrap();
    let stroke = node
        .attribute("stroke-width")
        .unwrap_or("1")
        .parse::<f64>()
        .unwrap()
        / 2.0;
    let mut bounds = if node.has_tag_name("text") {
        let x = coordinate("x");
        let y = coordinate("y");
        let width = (crate::view::text::width(node.text().unwrap()) * 8) as f64;
        let left = if node.attribute("text-anchor") == Some("middle") {
            x - width / 2.0
        } else {
            x
        };
        let height = node
            .attribute("font-size")
            .unwrap_or("12")
            .parse::<f64>()
            .unwrap();
        [left, y - height, left + width, y + 4.0]
    } else if node.has_tag_name("circle") {
        let radius = coordinate("r") + stroke;
        let x = coordinate("cx");
        let y = coordinate("cy");
        [x - radius, y - radius, x + radius, y + radius]
    } else if node.has_tag_name("rect") {
        let x = coordinate("x");
        let y = coordinate("y");
        [
            x - stroke,
            y - stroke,
            x + coordinate("width") + stroke,
            y + coordinate("height") + stroke,
        ]
    } else {
        assert!(node.has_tag_name("path"));
        let values: Vec<f64> = node
            .attribute("d")
            .unwrap()
            .split_whitespace()
            .filter_map(|part| part.parse().ok())
            .collect();
        if values.len() == 3 {
            let arrow = node.attribute("marker-end").is_some();
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
                bounds = union(bounds, [point[0], point[1], point[0], point[1]]);
            }
            bounds
        }
    };
    for ancestor in node
        .ancestors()
        .filter(|parent| parent.attribute("data-legend-entry").is_some())
    {
        let origin = position(ancestor);
        bounds[0] += origin.x as f64;
        bounds[2] += origin.x as f64;
        bounds[1] += origin.y as f64;
        bounds[3] += origin.y as f64;
    }
    bounds
}
