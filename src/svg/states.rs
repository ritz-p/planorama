use crate::layout::{Bounds, Layout, Point};
use crate::model::{
    StateId,
    cross_state::{Architecture, Endpoint},
};
use std::collections::BTreeMap;
use std::fmt::Write;

struct StateObstacles {
    bounds: Vec<Bounds>,
    headers: Vec<usize>,
    parents: Vec<Option<usize>>,
}

impl StateObstacles {
    fn for_endpoint(&self, endpoint: usize) -> Vec<Bounds> {
        let mut ancestors = std::collections::BTreeSet::new();
        let mut current = Some(endpoint);
        while let Some(index) = current {
            ancestors.insert(index);
            current = self.parents[index];
        }
        self.bounds
            .iter()
            .enumerate()
            .map(|(i, b)| {
                if ancestors.contains(&i) {
                    b.header(self.headers[i])
                } else {
                    *b
                }
            })
            .collect()
    }
}

pub fn render_states(architecture: &Architecture, format: super::AddressFormat) -> String {
    let graphs = &architecture.states;
    let mut endpoints = BTreeMap::new();
    let mut obstacles = BTreeMap::new();
    let mut sections = String::new();
    let mut top = 60;
    let mut width = 1120;
    for (id, graph) in graphs {
        let layout = Layout::new(graph);
        width = width.max(layout.width);
        let (_, height) = super::dimensions(graph, &layout);
        let prefix = prefix(id);
        let check_offset = if graph.checks.is_empty() { 0 } else { 24 };
        obstacles.insert(
            id.clone(),
            StateObstacles {
                bounds: layout
                    .bounds
                    .iter()
                    .map(|b| {
                        let mut bounds = *b;
                        bounds.origin.y += top + 40 + check_offset;
                        bounds
                    })
                    .collect::<Vec<_>>(),
                headers: layout.header_heights.clone(),
                parents: layout.parents.clone(),
            },
        );
        for (index, node) in graph
            .nodes
            .iter()
            .enumerate()
            .filter(|(_, n)| n.deposed_key.is_none())
        {
            let bounds = layout.bounds[index];
            endpoints.insert(
                Endpoint {
                    state: id.clone(),
                    address: node.address.clone(),
                },
                (
                    bounds.right(),
                    top + 40 + check_offset + bounds.origin.y + layout.header_heights[index] / 2,
                    format!("{prefix}resource-{index}"),
                    index,
                ),
            );
        }
        let document = namespace(&super::render_with_format(graph, &layout, format), &prefix);
        writeln!(sections, r##"<g id="{prefix}state" data-state-id="{}"><text x="40" y="{}" font-family="ui-monospace, Consolas, monospace" font-size="20" font-weight="700" fill="#0f172a">State: {}</text>"##, super::escape(id.as_str()), top + 25, super::escape(id.as_str())).unwrap();
        sections.push_str(&document.replacen("<svg ", &format!("<svg y=\"{}\" ", top + 40), 1));
        sections.push_str("</g>\n");
        top += height + 60;
    }
    let mut links = String::new();
    let mut link_count = 0;
    let mut relationships: Vec<_> = architecture.relationships.iter().collect();
    relationships.sort();
    for edge in relationships {
        let (Some((sx, sy, source, source_index)), Some((tx, ty, target, target_index))) =
            (endpoints.get(&edge.from), endpoints.get(&edge.to))
        else {
            continue;
        };
        let lane = width + 30 + link_count * 12;
        let mut points = crate::layout::route_to_margin(
            Point { x: *sx, y: *sy },
            Point { x: lane, y: *sy },
            &obstacles[&edge.from.state].for_endpoint(*source_index),
        );
        let mut arrival = crate::layout::route_to_margin(
            Point { x: *tx, y: *ty },
            Point { x: lane, y: *ty },
            &obstacles[&edge.to.state].for_endpoint(*target_index),
        );
        arrival.reverse();
        points.extend(arrival);
        let mut path = String::new();
        for (i, p) in points.iter().enumerate() {
            write!(path, "{} {} {} ", if i == 0 { "M" } else { "L" }, p.x, p.y).unwrap();
        }
        link_count += 1;
        let title = super::escape(&format!(
            "{}:{} -> {}:{} via {}.outputs.{} (cross-state dependency)",
            edge.from.state.as_str(),
            edge.from.address,
            edge.to.state.as_str(),
            edge.to.address,
            edge.remote,
            edge.output
        ));
        writeln!(links, r##"<path data-edge-kind="dependency" data-cross-state="true" data-source="{source}" data-target="{target}" data-source-state="{}" data-target-state="{}" data-remote-state="{}" data-output="{}" d="{}" fill="none" stroke="#2563eb" stroke-width="1.5" stroke-dasharray="7 4" marker-end="url(#cross-state-arrow)"><title>{title}</title></path>"##, super::escape(edge.from.state.as_str()),super::escape(edge.to.state.as_str()),super::escape(&edge.remote),super::escape(&edge.output),path.trim()).unwrap();
    }
    if !links.is_empty() {
        width += 80 + (link_count - 1) * 12;
        sections.push_str("<defs><marker id=\"cross-state-arrow\" viewBox=\"0 0 10 10\" refX=\"9\" refY=\"5\" markerWidth=\"7\" markerHeight=\"7\" orient=\"auto-start-reverse\"><path d=\"M 0 0 L 10 5 L 0 10 z\" fill=\"#2563eb\"/></marker></defs>\n");
        sections.push_str(&links);
        sections.push_str("<text x=\"350\" y=\"35\" font-family=\"ui-monospace, Consolas, monospace\" font-size=\"12\" fill=\"#2563eb\">Dashed blue: cross-state dependency</text>\n");
    }
    let description = if link_count == 0 {
        format!(
            "{} independent states. Resource identities and relationships are scoped to each state.",
            graphs.len()
        )
    } else {
        format!(
            "{} named states with {link_count} cross-state dependencies. Resource identities are scoped to each state; blue dashed paths connect output producers to consumers across states.",
            graphs.len()
        )
    };
    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{top}" viewBox="0 0 {width} {top}" role="img" aria-labelledby="multi-title multi-description">
<title id="multi-title">Terraform plans</title><desc id="multi-description">{description}</desc>
<rect width="100%" height="100%" fill="#ffffff"/><text x="40" y="35" font-family="ui-monospace, Consolas, monospace" font-size="25" fill="#0f172a">Terraform plans</text>
{sections}</svg>
"##
    )
}

// Hex encoding is injective and independent of file paths and input ordering.
fn prefix(id: &StateId) -> String {
    let mut prefix = String::from("state-");
    for byte in id.as_str().bytes() {
        write!(prefix, "{byte:02x}").unwrap();
    }
    prefix.push('-');
    prefix
}

fn namespace(svg: &str, prefix: &str) -> String {
    // These are renderer-owned quoted attributes. User content is XML-escaped
    // before this stage, so it cannot masquerade as an attribute or reference.
    svg.replace(" id=\"", &format!(" id=\"{prefix}"))
        .replace(" href=\"#", &format!(" href=\"#{prefix}"))
        .replace("marker-end=\"url(#", &format!("marker-end=\"url(#{prefix}"))
        .replace(
            "data-source=\"resource-",
            &format!("data-source=\"{prefix}resource-"),
        )
        .replace(
            "data-target=\"resource-",
            &format!("data-target=\"{prefix}resource-"),
        )
        .replace(
            "aria-labelledby=\"title description\"",
            &format!("aria-labelledby=\"{prefix}title {prefix}description\""),
        )
}

#[cfg(test)]
#[test]
fn cross_state_paths_avoid_intervening_cards_and_describe_dependencies() {
    let raw = crate::plan::parse(r#"{"format_version":"1.2","resource_changes":[{"address":"test.a","type":"test"},{"address":"test.b","type":"test"}],"configuration":{"root_module":{"resources":[{"address":"test.b","expressions":{"input":{"references":["test.a.id"]}}}]}}}"#).unwrap();
    let graph = crate::semantic::transform(&raw);
    let first = StateId::new("first").unwrap();
    let second = StateId::new("second").unwrap();
    let local = Layout::new(&graph);
    assert!(local.bounds[0].right() < local.bounds[1].right());
    let (_, height) = super::dimensions(&graph, &local);
    let obstacles: Vec<_> = [100, 100 + height + 60]
        .into_iter()
        .flat_map(|offset| {
            local.bounds.iter().map(move |b| {
                let mut b = *b;
                b.origin.y += offset;
                b
            })
        })
        .collect();
    let mut architecture = Architecture {
        states: BTreeMap::from([(first.clone(), graph.clone()), (second.clone(), graph)]),
        relationships: vec![crate::model::cross_state::CrossStateEdge {
            from: Endpoint {
                state: first,
                address: "test.a".into(),
            },
            to: Endpoint {
                state: second,
                address: "test.a".into(),
            },
            remote: "data.terraform_remote_state.x".into(),
            output: "value".into(),
        }],
    };
    let svg = render_states(&architecture, super::AddressFormat::Qualified);
    assert!(svg.contains("2 named states with 1 cross-state dependencies"));
    let line = svg
        .lines()
        .find(|l| l.contains("data-cross-state=\"true\""))
        .unwrap();
    let path = line
        .split(" d=\"")
        .nth(1)
        .unwrap()
        .split('"')
        .next()
        .unwrap();
    let tokens: Vec<_> = path.split_whitespace().collect();
    let points: Vec<Point> = tokens
        .chunks_exact(3)
        .map(|p| Point {
            x: p[1].parse().unwrap(),
            y: p[2].parse().unwrap(),
        })
        .collect();
    assert!(points.len() > 4);
    for segment in points.windows(2) {
        let (a, b) = (segment[0], segment[1]);
        for obstacle in &obstacles {
            let crosses = if a.x == b.x {
                a.x > obstacle.origin.x
                    && a.x < obstacle.right()
                    && a.y.max(b.y) > obstacle.origin.y
                    && a.y.min(b.y) < obstacle.origin.y + obstacle.height
            } else {
                a.y > obstacle.origin.y
                    && a.y < obstacle.origin.y + obstacle.height
                    && a.x.max(b.x) > obstacle.origin.x
                    && a.x.min(b.x) < obstacle.right()
            };
            assert!(!crosses, "{segment:?} crosses {obstacle:?}");
        }
    }
    let mut parallel = architecture.relationships[0].clone();
    parallel.output = "second_value".into();
    architecture.relationships.push(parallel);
    let svg = render_states(&architecture, super::AddressFormat::Qualified);
    let paths: std::collections::BTreeSet<_> = svg
        .lines()
        .filter(|line| line.contains("data-cross-state=\"true\""))
        .map(|line| {
            line.split(" d=\"")
                .nth(1)
                .unwrap()
                .split('"')
                .next()
                .unwrap()
        })
        .collect();
    assert_eq!(paths.len(), 2);
    assert!(svg.contains("outputs.second_value"));
    assert!(svg.contains("outputs.value"));
    architecture.relationships.reverse();
    assert_eq!(
        svg,
        render_states(&architecture, super::AddressFormat::Qualified)
    );
}

#[cfg(test)]
#[test]
fn cross_state_obstacles_only_relax_the_endpoint_and_its_ancestors() {
    let rectangle = |x, y, width, height| Bounds {
        origin: Point { x, y },
        width,
        height,
    };
    let state = StateObstacles {
        bounds: vec![
            rectangle(50, 100, 400, 300),
            rectangle(80, 180, 150, 96),
            rectangle(500, 120, 300, 300),
        ],
        headers: vec![60, 96, 60],
        parents: vec![None, Some(0), None],
    };
    let obstacles = state.for_endpoint(1);
    assert_eq!(obstacles[0], state.bounds[0].header(60));
    assert_eq!(obstacles[2], state.bounds[2]);
    let route = crate::layout::route_to_margin(
        Point { x: 230, y: 228 },
        Point { x: 900, y: 228 },
        &obstacles,
    );
    let unrelated = state.bounds[2];
    for segment in route.windows(2) {
        let (a, b) = (segment[0], segment[1]);
        let crosses = if a.x == b.x {
            a.x > unrelated.origin.x
                && a.x < unrelated.right()
                && a.y.max(b.y) > unrelated.origin.y
                && a.y.min(b.y) < unrelated.origin.y + unrelated.height
        } else {
            a.y > unrelated.origin.y
                && a.y < unrelated.origin.y + unrelated.height
                && a.x.max(b.x) > unrelated.origin.x
                && a.x.min(b.x) < unrelated.right()
        };
        assert!(!crosses, "{segment:?}");
    }
}
