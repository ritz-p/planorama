use crate::layout::{Bounds, Layout, Point};
use crate::model::architecture::{Endpoint, StateId};
use std::collections::BTreeMap;
use std::fmt::Write;

struct StateObstacles {
    scopes: Vec<Bounds>,
    markers: Vec<Bounds>,
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
            .chain(self.scopes.iter().copied())
            .chain(self.markers.iter().copied())
            .collect()
    }
}

pub fn render_states(
    architecture: &crate::view::ViewArchitecture,
    options: impl Into<super::Options>,
) -> String {
    render_content(architecture, options, super::Content::Standard)
}

pub(crate) fn render_content(
    architecture: &crate::view::ViewArchitecture,
    options: impl Into<super::Options>,
    content: super::Content,
) -> String {
    render_report(architecture, options, content, None)
}

pub(crate) fn render_report(
    architecture: &crate::view::ViewArchitecture,
    options: impl Into<super::Options>,
    content: super::Content,
    report: Option<&mut serde_json::Value>,
) -> String {
    let options = options.into();
    let format = options.address_format;
    let mut scene = report
        .as_ref()
        .map(|_| crate::layout::diagnostics::Scene::default());
    let graphs = &architecture.states;
    let mut endpoints = BTreeMap::new();
    let mut obstacles = BTreeMap::new();
    let mut sections = String::new();
    let mut relationships: Vec<_> = architecture.relationships().iter().collect();
    relationships.sort();
    if !architecture.relationships().is_empty() {
        let metadata: Vec<_> = relationships.iter().map(|edge| {
            let crate::model::architecture::CrossStateProvenance::TerraformRemoteState { remote, output, output_action } = &edge.provenance;
            serde_json::json!({"from_state":edge.from.state.as_str(), "from":edge.from.address, "to_state":edge.to.state.as_str(), "to":edge.to.address, "remote":remote, "output":output, "output_action":output_action.map(super::label)})
        }).collect();
        writeln!(
            sections,
            "<metadata id=\"cross-state-relationships\">{}</metadata>",
            super::escape(&serde_json::Value::Array(metadata).to_string())
        )
        .unwrap();
    }
    let mut prepared_graphs = Vec::with_capacity(graphs.len());
    for (id, graph) in graphs {
        let mut graph = graph.clone();
        for edge in &relationships {
            if &edge.to.state != id {
                continue;
            }
            let Some(source) = graphs.get(&edge.from.state).and_then(|g| {
                g.nodes
                    .iter()
                    .find(|n| n.address == edge.from.address && n.deposed_key.is_none())
            }) else {
                continue;
            };
            let Some(consumer) = graph
                .0
                .nodes
                .iter_mut()
                .find(|n| n.address == edge.to.address && n.deposed_key.is_none())
            else {
                continue;
            };
            if source.placement != crate::model::Placement::Logical
                && consumer.placement != crate::model::Placement::Logical
            {
                continue;
            }
            let crate::model::architecture::CrossStateProvenance::TerraformRemoteState {
                remote,
                output,
                ..
            } = &edge.provenance;
            consumer.references.push(crate::model::ResourceReference {
                label: "References",
                attribute: format!("{remote}.outputs.{output}"),
                targets: vec![crate::model::ReferenceTarget {
                    state: Some(edge.from.state.clone()),
                    id: crate::model::TerraformEntityId {
                        address: source.address.clone(),
                        deposed_key: None,
                    },
                    placement: source.placement,
                }],
                complete: true,
                dependency: false,
                issues: Default::default(),
            });
        }
        prepared_graphs.push((id, graph));
    }
    let mut width = 1120;
    let mut states = Vec::with_capacity(prepared_graphs.len());
    for (id, graph) in &prepared_graphs {
        let layout = Layout::new(graph);
        let index = super::relationship_index::Index::new(graph, &layout);
        let cards = super::annotation_cards(graph, &index, format);
        width = width.max(super::annotations::Area::required_width(
            &cards,
            index.width,
        ));
        states.push((*id, graph, layout, index, cards));
    }
    let legend = super::legend::MeasuredLegend::from_graphs(
        prepared_graphs.iter().map(|(_, graph)| &graph.0),
        width,
        60,
        options.hide_relationship_legend,
    );
    if legend.height > 0 {
        if let Some((_, graph)) = prepared_graphs.first() {
            writeln!(
                sections,
                "<defs>{}</defs>",
                super::relationships::markers(graph)
            )
            .unwrap();
        }
    }
    sections.push_str(&legend.render());
    let mut top = 60 + legend.height;
    for (id, graph, layout, index, cards) in states {
        let heading = format!("State: {}", id.as_str());
        let heading_lines = crate::view::text::wrap(&heading, (width - 80) / 12);
        let header_height = heading_lines.len() * 26 + 14;
        let measured_sections =
            super::Sections::from_cards(graph, &layout, &index, cards, width, None);
        let (_, height) = measured_sections.dimensions(&layout);
        let prefix = prefix(id);
        let architecture_offset = measured_sections.height();
        if let Some(scene) = &mut scene {
            scene.add_state(
                graph,
                &layout,
                Some(id.as_str()),
                top + header_height + architecture_offset,
                super::diagnostic_cards(graph, &layout),
            );
        }
        obstacles.insert(
            id.clone(),
            StateObstacles {
                markers: index
                    .marker_bounds()
                    .map(|mut bounds| {
                        bounds.origin.y += top + header_height + architecture_offset;
                        bounds
                    })
                    .collect(),
                scopes: layout
                    .scopes
                    .iter()
                    .map(|panel| {
                        let mut header = panel.bounds.header(36);
                        header.origin.y += top + header_height + architecture_offset;
                        header
                    })
                    .collect(),
                bounds: layout
                    .bounds
                    .iter()
                    .map(|b| {
                        let mut bounds = *b;
                        bounds.origin.y += top + header_height + architecture_offset;
                        bounds
                    })
                    .collect::<Vec<_>>(),
                headers: layout.header_heights.clone(),
                parents: layout.containment.parents.clone(),
            },
        );
        for (index, node) in graph.nodes.iter().enumerate().filter(|(_, n)| {
            n.deposed_key.is_none() && n.placement != crate::model::Placement::Logical
        }) {
            let port_index = crate::view::routes::owner(graph, node).unwrap_or(index);
            let bounds = layout.bounds[port_index];
            endpoints.insert(
                Endpoint {
                    state: id.clone(),
                    address: node.address.clone(),
                },
                (
                    bounds.right(),
                    top + header_height
                        + architecture_offset
                        + bounds.origin.y
                        + layout.header_heights[port_index] / 2,
                    format!("{prefix}{}", super::identity::resource(node)),
                    port_index,
                    format!(
                        "{prefix}{}",
                        super::identity::resource(&graph.nodes[port_index])
                    ),
                ),
            );
        }
        let document = namespace(
            &super::render_with_index(graph, &layout, format, &index, &measured_sections, content),
            &prefix,
        );
        writeln!(
            sections,
            r#"<g id="{prefix}state" data-state-id="{}">"#,
            super::escape(id.as_str())
        )
        .unwrap();
        for (line, text) in heading_lines.iter().enumerate() {
            writeln!(sections, r##"<text x="40" y="{}" font-family="ui-monospace, Consolas, monospace" font-size="20" font-weight="700" fill="#0f172a">{}</text>"##, top + 25 + line * 26, super::escape(text)).unwrap();
        }
        sections.push_str(&document.replacen(
            "<svg ",
            &format!("<svg y=\"{}\" ", top + header_height),
            1,
        ));
        sections.push_str("</g>\n");
        top += height + header_height + 20;
    }
    let mut links = String::new();
    let mut link_count = 0;
    for edge in relationships {
        let crate::model::architecture::CrossStateProvenance::TerraformRemoteState {
            remote,
            output,
            output_action,
        } = &edge.provenance;
        let (
            Some((sx, sy, source, source_index, source_port)),
            Some((tx, ty, target, target_index, target_port)),
        ) = (endpoints.get(&edge.from), endpoints.get(&edge.to))
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
        if let Some(scene) = &mut scene {
            scene.add_cross_state(edge, &points);
        }
        let mut path = String::new();
        for (i, p) in points.iter().enumerate() {
            write!(path, "{} {} {} ", if i == 0 { "M" } else { "L" }, p.x, p.y).unwrap();
        }
        link_count += 1;
        let action_metadata = output_action
            .map(|action| format!(" data-output-action=\"{}\"", super::label(action)))
            .unwrap_or_default();
        let title = super::escape(&format!(
            "{}:{} -> {}:{} via {}.outputs.{} (cross-state dependency)",
            edge.from.state.as_str(),
            edge.from.address,
            edge.to.state.as_str(),
            edge.to.address,
            remote,
            output
        ));
        writeln!(links, r##"<path data-edge-kind="dependency" data-cross-state="true"{action_metadata} data-source="{source}" data-target="{target}" data-source-port="{source_port}" data-target-port="{target_port}" data-source-state="{}" data-target-state="{}" data-remote-state="{}" data-output="{}" d="{}" fill="none" stroke="#2563eb" stroke-width="1.5" stroke-dasharray="7 4" marker-end="url(#cross-state-arrow)"><title>{title}</title></path>"##, super::escape(edge.from.state.as_str()),super::escape(edge.to.state.as_str()),super::escape(remote),super::escape(output),path.trim()).unwrap();
        links.push_str(&super::endpoints::circle(points[0], "source", "#2563eb"));
    }
    if !links.is_empty() {
        width += 80 + (link_count - 1) * 12;
        sections.push_str("<defs><marker id=\"cross-state-arrow\" viewBox=\"0 0 10 10\" refX=\"9\" refY=\"5\" markerWidth=\"7\" markerHeight=\"7\" orient=\"auto-start-reverse\"><path d=\"M 0 0 L 10 5 L 0 10 z\" fill=\"#2563eb\"/></marker></defs>\n");
        sections.push_str(&links);
        if !options.hide_relationship_legend {
            sections.push_str("<text x=\"350\" y=\"35\" font-family=\"ui-monospace, Consolas, monospace\" font-size=\"12\" fill=\"#2563eb\">Dashed blue: cross-state dependency</text>\n");
        }
    }
    let description = if architecture.relationships().len() > link_count {
        format!(
            "{} named states with {link_count} routed cross-state dependencies and {} logical cross-state references. Logical references link resource cards without routed paths.",
            graphs.len(),
            architecture.relationships().len() - link_count
        )
    } else if link_count == 0 {
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
    if let (Some(scene), Some(report)) = (scene, report) {
        *report = scene.finish(width, top);
    }
    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{top}" viewBox="0 0 {width} {top}" font-family="ui-monospace, Consolas, monospace" role="img" aria-labelledby="multi-title multi-description">
<title id="multi-title">Terraform plans</title><desc id="multi-description">{description}</desc>
<rect width="100%" height="100%" fill="#ffffff"/><text x="40" y="35" font-family="ui-monospace, Consolas, monospace" font-size="25" fill="#0f172a">Terraform plans</text>
{sections}</svg>
"##
    )
}

pub(crate) fn prefix(id: &StateId) -> String {
    let mut prefix = String::from("state-");
    for byte in id.as_str().bytes() {
        write!(prefix, "{byte:02x}").unwrap();
    }
    prefix.push('-');
    prefix
}

fn namespace(svg: &str, prefix: &str) -> String {
    let mut scoped = svg
        .replace(" id=\"", &format!(" id=\"{prefix}"))
        .replace(
            "data-container=\"resource-",
            &format!("data-container=\"{prefix}resource-"),
        )
        .replace(
            "data-route-table=\"resource-",
            &format!("data-route-table=\"{prefix}resource-"),
        )
        .replace(" href=\"#", &format!(" href=\"#{prefix}"))
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
        );
    for tail in svg.split("data-relationship-ids=\"").skip(1) {
        let ids = tail.split('"').next().unwrap();
        let namespaced = ids
            .split_whitespace()
            .map(|id| format!("{prefix}{id}"))
            .collect::<Vec<_>>()
            .join(" ");
        scoped = scoped.replace(
            &format!("data-relationship-ids=\"{ids}\""),
            &format!("data-relationship-ids=\"{namespaced}\""),
        );
    }
    for attribute in ["marker-end", "fill", "stroke", "clip-path", "mask"] {
        scoped = scoped.replace(
            &format!(" {attribute}=\"url(#"),
            &format!(" {attribute}=\"url(#{prefix}"),
        );
    }
    scoped.replace(" data-global-href=", " href=")
}

#[cfg(test)]
#[path = "../../tests/unit/svg/states.rs"]
mod tests;
