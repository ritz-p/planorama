mod annotations;
mod checks;
mod components;
mod containers;
mod endpoints;
mod entities;
mod icons;
mod identity;
mod legend;
mod logical;
mod occlusion;
mod operations;
mod paths;
mod provenance;
mod references;
mod relationship_index;
mod relationships;
mod roles;
mod routes;
mod scopes;
mod states;
mod style;
pub(crate) use identity::resource as resource_id;
pub use states::render_states;
pub(crate) use states::{prefix as state_prefix, render_content as render_states_content};
#[cfg(test)]
#[path = "../tests/unit/svg.rs"]
mod tests;

use crate::layout::Layout;
use crate::model::architecture::{Action, EdgeKind, Graph};
use std::fmt::Write;
use style::{color, label, shorten};

#[derive(Clone, Copy, Default)]
pub enum AddressFormat {
    #[default]
    Qualified,
    Terraform,
}

#[derive(Clone, Copy)]
pub(crate) enum Content {
    Standard,
    ValueFree,
}

fn escape(value: &str) -> String {
    value
        .chars()
        .filter(|&c| c >= ' ' || matches!(c, '\n' | '\r' | '\t'))
        .map(|c| match c {
            '&' => "&amp;".into(),
            '<' => "&lt;".into(),
            '>' => "&gt;".into(),
            '"' => "&quot;".into(),
            '\'' => "&apos;".into(),
            _ => c.to_string(),
        })
        .collect()
}

pub(crate) fn render(graph: &Graph, layout: &Layout<'_>) -> String {
    render_with_format(graph, layout, AddressFormat::Qualified)
}

#[cfg(test)]
fn dimensions(graph: &Graph, layout: &Layout<'_>) -> (usize, usize) {
    let index = relationship_index::Index::new(graph, layout);
    dimensions_with_index(graph, layout, &index, AddressFormat::Qualified)
}

fn dimensions_with_index(
    graph: &Graph,
    layout: &Layout<'_>,
    index: &relationship_index::Index<'_>,
    address_format: AddressFormat,
) -> (usize, usize) {
    let annotations = annotation_area(graph, index, address_format);
    (
        annotations.width,
        layout.height
            + if graph.checks.is_empty() { 0 } else { 24 }
            + legend::height(graph)
            + annotations.height,
    )
}

fn annotation_area(
    graph: &Graph,
    index: &relationship_index::Index<'_>,
    address_format: AddressFormat,
) -> annotations::Area {
    annotations::Area::sections(
        vec![
            (
                "Logical Resources",
                style::LOGICAL_RESOURCES,
                logical::cards(graph, address_format),
            ),
            (
                "Logical Components",
                style::LOGICAL_COMPONENTS,
                components::cards(graph),
            ),
            (
                "Relationship Resources",
                style::RELATIONSHIP_RESOURCES,
                index.cards(),
            ),
        ],
        index.width,
    )
}

pub(crate) fn render_with_format(
    graph: &Graph,
    layout: &Layout<'_>,
    address_format: AddressFormat,
) -> String {
    render_content(graph, layout, address_format, Content::Standard)
}

pub(crate) fn render_content(
    graph: &Graph,
    layout: &Layout<'_>,
    address_format: AddressFormat,
    content: Content,
) -> String {
    let relationship_index = relationship_index::Index::new(graph, layout);
    render_with_index(graph, layout, address_format, &relationship_index, content)
}

fn render_with_index(
    graph: &Graph,
    layout: &Layout<'_>,
    address_format: AddressFormat,
    relationship_index: &relationship_index::Index<'_>,
    content: Content,
) -> String {
    let check_offset = if graph.checks.is_empty() { 0 } else { 24 };
    let (width, height) = dimensions_with_index(graph, layout, relationship_index, address_format);
    let membership = components::membership(graph);
    let (description, summary) = relationships::captions(graph);
    let markers = relationships::markers(graph);
    let icon_definitions = icons::definitions(&graph.nodes);
    let occlusion = occlusion::Masks::new(graph, layout);
    let occlusion_definitions = occlusion.definitions(layout);
    let semantic_edges = graph
        .edges
        .iter()
        .any(|edge| edge.kind != EdgeKind::Dependency);
    let mut svg = format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}" role="img" aria-labelledby="title description">
<title id="title">Terraform plan</title>
<desc id="description">{description} Thin solid: dependency. Dashed: association. Thick dotted: connection. Nested boxes: containment. Edge color preserves the Terraform action when available.</desc>
<defs><marker id="arrow" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="7" markerHeight="7" markerUnits="userSpaceOnUse" orient="auto-start-reverse"><path d="M 0 0 L 10 5 L 0 10 z" fill="#94a3b8"/></marker>{markers}{icon_definitions}{occlusion_definitions}</defs>
<rect width="100%" height="100%" fill="#ffffff"/>
<g font-family="ui-monospace, SFMono-Regular, Consolas, monospace">
<text x="40" y="45" font-size="25" font-weight="700" fill="#0f172a">Terraform plan</text>
<text x="40" y="73" font-size="13" fill="#475569">{summary}</text>
"##,
    );
    svg.push_str(&provenance::metadata(graph, content));
    for (i, action) in [
        Action::Create,
        Action::Update,
        Action::Delete,
        Action::Replace,
        Action::Read,
        Action::Unchanged,
        Action::Other,
    ]
    .iter()
    .enumerate()
    {
        let x = 40 + i * 140;
        let (background, border) = color(*action);
        let count = graph.nodes.iter().filter(|n| n.action == *action).count()
            + graph
                .edges
                .iter()
                .filter(|edge| {
                    edge.change.as_ref().is_some_and(|change| {
                        change.action == *action && graph.nodes[edge.from].address != change.address
                    })
                })
                .count();
        writeln!(svg, r#"<rect x="{x}" y="100" width="12" height="12" fill="{background}" stroke="{border}"/><text x="{}" y="111" font-size="12" fill="{border}">{} ({count})</text>"#, x + 19, label(*action)).unwrap();
    }
    if graph.status != Default::default() {
        svg.push_str("<g id=\"plan-status\"");
        let flags = [
            ("applyable", graph.status.applyable),
            ("complete", graph.status.complete),
            ("errored", graph.status.errored),
        ];
        for (name, value) in flags {
            if let Some(value) = value {
                write!(svg, " data-plan-{name}=\"{value}\"").unwrap();
            }
        }
        svg.push_str(">\n<text x=\"40\" y=\"137\" font-size=\"12\" fill=\"#475569\">Plan status:");
        for (name, value) in flags {
            let value = match value {
                Some(true) => "true",
                Some(false) => "false",
                None => "unknown",
            };
            write!(svg, " {name}: {value}").unwrap();
        }
        svg.push_str("</text>\n</g>\n");
    }
    let legend_height = legend::height(graph);
    if legend_height != 0 {
        svg.push_str(&legend::render(graph));
        writeln!(svg, "<g transform=\"translate(0 {legend_height})\">").unwrap();
    }
    if check_offset != 0 {
        svg.push_str(&checks::render(&graph.checks));
        svg.push_str("<g transform=\"translate(0 24)\">\n");
    }
    let annotations = annotation_area(graph, relationship_index, address_format);
    svg.push_str(&annotations.render());
    writeln!(
        svg,
        "<g data-architecture=\"true\" transform=\"translate(0 {})\">",
        annotations.height
    )
    .unwrap();
    for band in &layout.bands {
        let (module, y, h) = (band.label, band.top, band.height);
        writeln!(svg, r##"<rect x="35" y="{y}" width="{}" height="{h}" rx="12" fill="#f8fafc" stroke="#cbd5e1" stroke-dasharray="5 4"/><text x="52" y="{}" font-size="13" font-weight="700" fill="#475569"><title>{}</title>{}</text>"##, relationship_index.width - 70, y + 25, escape(module), escape(&shorten(module, 110))).unwrap();
    }
    svg.push_str(&scopes::render(layout));
    svg.push_str(&containers::boundaries(graph, layout));
    for (index, (edge, points)) in graph.edges.iter().zip(&layout.paths).enumerate() {
        let relation = match edge.kind {
            EdgeKind::Dependency => " data-edge-kind=\"dependency\"",
            EdgeKind::Association => " data-edge-kind=\"association\"",
            EdgeKind::Connection => " data-edge-kind=\"connection\"",
            EdgeKind::Containment => " data-edge-kind=\"containment\"",
        };
        let title = relationships::title(graph, edge, semantic_edges);
        let operation = edge
            .change
            .as_ref()
            .map(|change| operations::attributes(&change.metadata))
            .unwrap_or_default();
        let previous = edge
            .change
            .as_ref()
            .and_then(|change| change.previous_address.as_ref())
            .map(|address| format!(" data-previous-address=\"{}\"", escape(address)))
            .unwrap_or_default();
        let source = identity::resource(&graph.nodes[edge.from]);
        let target = identity::resource(&graph.nodes[edge.to]);
        if points.is_empty() {
            writeln!(svg, r#"<g{relation}{previous}{operation} data-source="{source}" data-target="{target}"><title>{title}</title></g>"#).unwrap();
            continue;
        }
        let stroke = relationships::stroke(edge);
        let marker = match &edge.change {
            Some(change) => format!("arrow-{}", label(change.action)),
            None => "arrow".into(),
        };
        let path = paths::rounded(points);
        let style = relationships::style(edge.kind);
        let direction = edge.directionality().as_str();
        let arrow = if edge.directionality() == crate::model::Directionality::Directed {
            format!(r#" marker-end="url(#{marker})""#)
        } else {
            String::new()
        };
        let mask = occlusion.attribute(index);
        writeln!(svg, r##"<path d="{path}"{relation}{previous}{operation} data-directionality="{direction}" data-source="{source}" data-target="{target}" fill="none" stroke="{stroke}" {style} stroke-linejoin="round"{arrow}{mask}><title>{title}</title></path>"##).unwrap();
    }
    for point in &layout.junctions {
        let mask = layout
            .paths
            .iter()
            .position(|path| path.contains(point))
            .map(|index| occlusion.attribute(index))
            .unwrap_or_default();
        writeln!(
            svg,
            r##"<circle cx="{}" cy="{}" r="3" fill="#94a3b8"{mask}/>"##,
            point.x, point.y
        )
        .unwrap();
    }
    for (i, node) in graph.nodes.iter().enumerate() {
        if node.placement == crate::model::Placement::Logical
            || crate::view::routes::owner(graph, node).is_some()
        {
            continue;
        }
        let resource_id = identity::resource(node);
        let architecture_id = escape(node.entity.id.as_str());
        let bounds = layout.bounds[i];
        let (x, y) = (bounds.origin.x, bounds.origin.y);
        let card_width = bounds.width;
        let header_height = if node.role != crate::model::ResourceRole::Container
            && layout.containers.contains(&i)
        {
            bounds.height
        } else {
            layout.header_heights[i]
        };
        let inferred = layout.containment.parents[i]
            .is_some_and(|parent| graph.derived_containment(parent, i));
        let background = if node.role != crate::model::ResourceRole::Container
            && layout.containers.contains(&i)
        {
            "none"
        } else {
            containers::background(graph, layout, i)
        };
        let border = color(node.action).1;
        let resource_address = node.resource_address();
        let address = escape(resource_address.terraform());
        let qualified = escape(&resource_address.qualified());
        let selected = match address_format {
            AddressFormat::Qualified => resource_address.qualified(),
            AddressFormat::Terraform => resource_address.terraform().into(),
        };
        let selected_title = escape(&selected);
        let icon = icons::for_node(node);
        let metrics = crate::view::card::measure(node);
        let lines = crate::view::text::wrap_address(&selected, metrics.line_limit);
        let mode = entities::border(node.mode);
        let operation = operations::attributes(&node.metadata);
        let component = membership
            .get(node.address.as_str())
            .map(|id| format!(" data-component-id=\"{}\"", escape(id)))
            .unwrap_or_default();
        let operation_label = operations::label(node.action, &node.metadata);
        let operation_details = escape(&operations::details(&node.metadata));
        let move_details = escape(&operations::move_details(node.previous_address.as_deref()));
        let previous = node
            .previous_address
            .as_ref()
            .map(|address| format!(" data-previous-address=\"{}\"", escape(address)))
            .unwrap_or_default();
        let deposed = node
            .deposed_key
            .as_ref()
            .map(|key| format!(" data-deposed-key=\"{}\"", escape(key)))
            .unwrap_or_default();
        writeln!(
            svg,
            r#"<g id="{resource_id}" data-architecture-id="{architecture_id}" data-qualified-address="{qualified}" data-terraform-address="{address}"{deposed}{previous}{operation}{component}><title>{selected_title} — {operation_label}{operation_details}{move_details}</title>"#,
        )
        .unwrap();
        if node.role != crate::model::architecture::ResourceRole::Container
            && !layout.containers.contains(&i)
        {
            let radius = crate::view::geometry::CARD_RADIUS;
            let stroke = crate::view::geometry::CARD_STROKE as f64 / 4.0;
            writeln!(svg, r#"<rect x="{x}" y="{y}" width="{card_width}" height="{header_height}" rx="{radius}" fill="{background}" stroke="{border}" stroke-width="{stroke}"{mode}/>"#).unwrap();
        }
        svg.push_str(&entities::badge(node.mode, x, y));
        svg.push_str(&entities::import_badge(node, x + 12, y + 28));
        let visible_operation = operations::action_label(node.action, &node.metadata);
        writeln!(
            svg,
            r#"<text x="{}" y="{}" font-size="11" fill="{border}">{} · {}</text>"#,
            x + 12,
            y + 20,
            escape(&shorten(
                &node.resource_type,
                entities::type_limit(node.mode).saturating_sub(
                    visible_operation
                        .len()
                        .saturating_sub(label(node.action).len())
                )
            )),
            visible_operation
        )
        .unwrap();
        if let Some(icon) = icon {
            svg.push_str(&icons::render(icon, x + 12, y + 28 + metrics.import_height));
        }
        for (line, text) in lines.iter().enumerate() {
            writeln!(
                svg,
                r##"<text x="{}" y="{}" font-size="13" fill="#0f172a" xml:space="preserve">{}</text>"##,
                x + metrics.inset,
                y + 45 + metrics.import_height + line * 20,
                escape(text)
            )
            .unwrap();
        }
        if let Some(previous) = &node.previous_address {
            let (annotation, _) = operations::moved_from(
                previous,
                x + 12,
                y + metrics.previous_y,
                metrics.previous_line_limit,
            );
            svg.push_str(&annotation);
        }
        svg.push_str(&references::render(
            graph,
            node,
            x + 12,
            y + metrics.references_y,
            card_width - 24,
        ));
        svg.push_str(&routes::render(
            graph,
            i,
            x + 12,
            y + metrics.routes_y,
            relationship_index,
            content,
        ));
        let role = roles::annotation(node.role, &node.module);
        let footer = match role.is_empty() {
            false => role,
            true if !layout.containers.is_empty() => escape(&shorten(&node.module, 43)),
            true => String::new(),
        };
        if !footer.is_empty() {
            writeln!(svg, r##"<text x="{}" y="{}" font-size="10" fill="#64748b"{}><title>{}</title>{footer}</text>"##, x + 12, y + header_height - 12, roles::attributes(node.role), escape(&node.module)).unwrap();
        }
        if inferred {
            writeln!(svg, r##"<text x="{}" y="{}" font-size="10" fill="#64748b" data-placement="inferred">inferred placement</text>"##, x + 12, y + header_height - 30).unwrap();
        }
        svg.push_str("</g>\n");
    }
    if graph.nodes.is_empty() {
        svg.push_str("<text x=\"40\" y=\"185\" font-size=\"16\" fill=\"#64748b\">No resources to display.</text>\n");
    }
    svg.push_str(&endpoints::render(graph, layout));
    svg.push_str(&relationship_index.markers());
    svg.push_str("</g>\n");
    if check_offset != 0 {
        svg.push_str("</g>\n");
    }
    if legend_height != 0 {
        svg.push_str("</g>\n");
    }
    svg.push_str("</g>\n</svg>\n");
    svg
}
