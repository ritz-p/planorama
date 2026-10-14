use super::routing_shared::{borders, scoring::Occluder, spacing};
use super::{Bounds, Layout, Point};
use crate::model::{Graph, RelationshipProvenance};
use crate::view::geometry;
use serde_json::{Value, json};
use std::collections::BTreeSet;

const PAIR_LIMIT: usize = 2_000_000;
const FINDING_LIMIT: usize = 20_000;

#[derive(Default)]
pub(crate) struct Scene {
    edges: Vec<Route>,
    frames: Vec<(Value, borders::Frame)>,
    states: Vec<Value>,
}

struct Route {
    identity: Value,
    path: Vec<Point>,
    stroke: usize,
    soft: Vec<Occluder>,
    rounded: bool,
}

#[derive(Clone, Copy)]
struct Segment {
    index: usize,
    horizontal: bool,
    axis: usize,
    low: usize,
    high: usize,
}

fn point(p: Point) -> Value {
    json!([p.x, p.y])
}

fn bounds(b: Bounds) -> Value {
    json!({"x": b.origin.x, "y": b.origin.y, "width": b.width, "height": b.height})
}

fn translated(mut b: Bounds, offset: usize) -> Bounds {
    b.origin.y += offset;
    b
}

fn identity(graph: &Graph, index: usize, state: Option<&str>) -> Value {
    let edge = &graph.edges[index];
    let from = &graph.nodes[edge.from].entity.id;
    let to = &graph.nodes[edge.to].entity.id;
    let mut provenance: Vec<_> = graph.relationships.iter()
        .filter(|r| &r.from == from && &r.to == to && r.kind == edge.kind)
        .flat_map(|r| &r.provenance)
        .filter(|p| match (p, &edge.change) {
            (RelationshipProvenance::Resource { change, .. }, Some(selected)) => change == selected,
            (RelationshipProvenance::Reference { .. }, None) => true,
            _ => false,
        })
        .map(|p| match p {
            RelationshipProvenance::Resource { source, .. } => json!({"resource": source.address, "deposed_key": source.deposed_key}),
            RelationshipProvenance::Reference { from, to } => json!({"from": from.address, "from_deposed_key": from.deposed_key, "to": to.address, "to_deposed_key": to.deposed_key}),
        }).collect();
    provenance.sort_by_key(Value::to_string);
    provenance.dedup();
    json!({"source": {"state": state, "id": from.as_str()},
        "target": {"state": state, "id": to.as_str()},
        "kind": format!("{:?}", edge.kind).to_lowercase(),
        "directionality": edge.directionality().as_str(),
        "relationship_resource": edge.change.as_ref().map(|c| &c.address), "provenance": provenance})
}

impl Scene {
    pub(crate) fn add_state(
        &mut self,
        graph: &Graph,
        layout: &Layout<'_>,
        state: Option<&str>,
        offset: usize,
        cards: Value,
    ) {
        let resources: BTreeSet<_> = graph
            .entities()
            .flat_map(|e| &e.provenance)
            .chain(
                graph
                    .relationships
                    .iter()
                    .flat_map(|r| &r.provenance)
                    .filter_map(|p| match p {
                        RelationshipProvenance::Resource { source, .. } => Some(source),
                        _ => None,
                    }),
            )
            .collect();
        self.states
            .push(json!({"state": state, "origin": [0, offset],
            "spatial_layout": {"width": layout.width, "height": layout.height},
            "resources": resources.len(), "cards": cards}));
        for mut frame in borders::frames(graph, layout) {
            let id = json!({"state": state, "node": frame.node.map(|i| graph.nodes[i].entity.id.as_str()),
                "bounds": bounds(translated(frame.bounds, offset))});
            frame.bounds = translated(frame.bounds, offset);
            frame.node = None;
            self.frames.push((id, frame));
        }
        for (i, path) in layout
            .paths
            .iter()
            .enumerate()
            .filter(|(_, p)| p.len() >= 2)
        {
            let soft = super::containers::routing::soft_obstacles(layout, &graph.edges[i])
                .into_iter()
                .map(|mut o| {
                    o.bounds = translated(o.bounds, offset);
                    o
                })
                .collect();
            self.edges.push(Route {
                identity: identity(graph, i, state),
                path: path
                    .iter()
                    .map(|p| Point {
                        x: p.x,
                        y: p.y + offset,
                    })
                    .collect(),
                stroke: geometry::relationship_stroke(graph.edges[i].kind),
                soft,
                rounded: true,
            });
        }
    }

    pub(crate) fn add_cross_state(
        &mut self,
        edge: &crate::model::architecture::CrossStateEdge,
        path: &[Point],
    ) {
        let crate::model::architecture::CrossStateProvenance::TerraformRemoteState {
            remote,
            output,
            ..
        } = &edge.provenance;
        self.edges.push(Route {
            identity: json!({"source": {"state": edge.from.state.as_str(), "id": edge.from.address},
                "target": {"state": edge.to.state.as_str(), "id": edge.to.address}, "kind": "dependency",
                "directionality": "directed", "provenance": {"remote": remote, "output": output}, "cross_state": true}),
            path: path.to_vec(), stroke: geometry::DEPENDENCY_STROKE, soft: Vec::new(), rounded: false,
        });
    }

    pub(crate) fn finish(mut self, width: usize, height: usize) -> Value {
        self.edges.sort_by_key(|e| {
            (
                e.identity.to_string(),
                e.path.iter().map(|p| (p.x, p.y)).collect::<Vec<_>>(),
            )
        });
        self.frames.sort_by_key(|(id, _)| id.to_string());
        let index = borders::Index::new(self.frames.iter().map(|(_, f)| *f).collect());
        let mut findings = Vec::new();
        let mut routes = Vec::new();
        let mut segments = Vec::new();
        let mut raw_length = 0u128;
        let mut raw_bends = 0usize;
        let mut unsupported = 0;
        let mut border_deficit = 0u128;
        for edge in &self.edges {
            let raw = super::metrics::measure(std::slice::from_ref(&edge.path));
            raw_length += raw.total_path_length;
            raw_bends += raw.bend_count;
            unsupported += edge
                .path
                .windows(2)
                .filter(|p| p[0].x != p[1].x && p[0].y != p[1].y)
                .count();
            let border = index.path(&edge.path, edge.stroke, &edge.soft);
            border_deficit += border.minimum_deficit;
            let id = edge.identity.to_string();
            for contact in index.contacts(&edge.path, edge.stroke, &edge.soft) {
                if findings.len() == FINDING_LIMIT {
                    break;
                }
                let frame = self
                    .frames
                    .iter()
                    .find(|(_, f)| *f == contact.frame)
                    .map(|(id, _)| id);
                findings.push(json!({"category": "visible_border_clearance", "edge": id,
                    "segment": contact.segment, "horizontal": contact.horizontal, "coordinate": contact.coordinate,
                    "interval": [contact.low, contact.high], "frame": frame, "below_minimum": contact.urgent}));
            }
            segments.push(visible_segments(edge, &index));
            routes.push(json!({"id": id, "identity": edge.identity, "polyline": edge.path.iter().copied().map(point).collect::<Vec<_>>(),
                "raw_polyline": {"length": raw.total_path_length, "bends": raw.bend_count},
                "visible_border": {"minimum_gap_within_8px": border.minimum.map(|v| v as f64 / 4.0),
                    "close_length": border.close_length, "minimum_deficit": border.minimum_deficit as f64 / 4.0,
                    "preferred_deficit": border.preferred_deficit as f64 / 4.0}}));
        }
        let mut comparisons = 0;
        let mut crossing_count = 0;
        let mut overlap = 0;
        let mut parallel_deficit = 0;
        let mut shared_length = 0;
        'pairs: for a in 0..self.edges.len() {
            for b in a + 1..self.edges.len() {
                let mut crossings = BTreeSet::new();
                for sa in &segments[a] {
                    for sb in &segments[b] {
                        if comparisons == PAIR_LIMIT {
                            break 'pairs;
                        }
                        comparisons += 1;
                        let (category, location) = if sa.horizontal != sb.horizontal {
                            let (h, v) = if sa.horizontal { (sa, sb) } else { (sb, sa) };
                            if h.low < v.axis
                                && v.axis < h.high
                                && v.low < h.axis
                                && h.axis < v.high
                                && crossings.insert((v.axis, h.axis))
                            {
                                crossing_count += 1;
                                ("crossing", json!({"point": [v.axis, h.axis]}))
                            } else {
                                continue;
                            }
                        } else {
                            let mut low = sa.low.max(sb.low);
                            let mut high = sa.high.min(sb.high);
                            if low >= high {
                                continue;
                            }
                            let distance = sa.axis.abs_diff(sb.axis);
                            let ea = &self.edges[a];
                            let eb = &self.edges[b];
                            if distance == 0 {
                                for (low, high, is_shared) in
                                    shared_sections(ea, eb, *sa, low, high)
                                {
                                    if is_shared {
                                        shared_length += high - low;
                                    } else {
                                        overlap += high - low;
                                        if findings.len() < FINDING_LIMIT {
                                            findings.push(json!({"category": "independent_overlap",
                                                "edge": ea.identity.to_string(), "peer": eb.identity.to_string(),
                                                "segment": sa.index, "peer_segment": sb.index,
                                                "location": {"horizontal": sa.horizontal, "coordinates": [sa.axis, sb.axis],
                                                    "interval": [low, high], "visible_gap": -(ea.stroke as f64 + eb.stroke as f64) / 8.0}}));
                                        }
                                    }
                                }
                                continue;
                            }
                            let gap = distance as f64 - (ea.stroke + eb.stroke) as f64 / 8.0;
                            let category = if gap < 0.0 {
                                overlap += high - low;
                                "independent_overlap"
                            } else if distance < 16 {
                                trim_endpoint(ea, *sa, &mut low, &mut high);
                                trim_endpoint(eb, *sb, &mut low, &mut high);
                                if high.saturating_sub(low) < 64 {
                                    continue;
                                }
                                parallel_deficit += (16 - distance) * (high - low);
                                "parallel_proximity"
                            } else {
                                continue;
                            };
                            (
                                category,
                                json!({"horizontal": sa.horizontal, "coordinates": [sa.axis, sb.axis], "interval": [low, high], "visible_gap": gap}),
                            )
                        };
                        if findings.len() < FINDING_LIMIT {
                            findings.push(json!({"category": category, "edge": self.edges[a].identity.to_string(),
                                "peer": self.edges[b].identity.to_string(), "segment": sa.index, "peer_segment": sb.index, "location": location}));
                        }
                    }
                }
            }
        }
        findings.sort_by_key(Value::to_string);
        json!({"schema_version": 1, "category": "layout", "scope": "whole_diagram", "units": "SVG user units (px)",
            "canvas": {"width": width, "height": height}, "states": self.states, "edges": routes, "findings": findings,
            "summary": {"routed_paths": self.edges.len(), "raw_polyline": {"length": raw_length, "bends": raw_bends},
                "visible_straight_sections": {"crossings": crossing_count, "independent_overlap_length": overlap,
                    "shared_trunk_length": shared_length, "parallel_deficit": parallel_deficit},
                "visible_border_minimum_deficit": border_deficit as f64 / 4.0},
            "limits": {"pair_comparisons": comparisons, "pair_limit": PAIR_LIMIT, "finding_limit": FINDING_LIMIT,
                "pair_metrics_complete": comparisons < PAIR_LIMIT, "findings_complete": findings.len() < FINDING_LIMIT, "unsupported_diagonal_segments": unsupported},
            "policy": {"parallel_centerline_preference": 16, "parallel_minimum_run": 64, "endpoint_convergence": 32,
                "border_minimum_gap": 4, "border_preferred_gap": 8,
                "approximations": ["Pair metrics use rounded straight sections with conservative integer mask clipping; curve-to-curve and curve-to-line pair metrics unavailable.",
                    "Border metrics share routing's quarter-pixel clearance and 32-sample quadratic corner approximation; cross-state paths have square corners, approximated as rounded for border measurement.",
                    "Strokes use continuous envelopes: dash gaps, markers, text, scope/band borders and annotation cards are excluded from spatial conflicts.",
                    "Border minimum is null when no eligible border is within 8px; deficits use px squared; shared prefix/suffix trunks are excluded from independent overlap."]}})
    }
}

fn visible_segments(edge: &Route, index: &borders::Index) -> Vec<Segment> {
    let mut radii = vec![0; edge.path.len()];
    if edge.rounded {
        for (i, p) in edge.path.windows(3).enumerate() {
            if (p[0].x == p[1].x && p[1].y == p[2].y) || (p[0].y == p[1].y && p[1].x == p[2].x) {
                let length = |a: Point, b: Point| a.x.abs_diff(b.x) + a.y.abs_diff(b.y);
                radii[i + 1] = geometry::relationship_radius(length(p[0], p[1]), length(p[1], p[2]))
                    .ceil() as usize;
            }
        }
    }
    let mut result = Vec::new();
    for (i, p) in edge.path.windows(2).enumerate() {
        if p[0] == p[1] || (p[0].x != p[1].x && p[0].y != p[1].y) {
            continue;
        }
        let horizontal = p[0].y == p[1].y;
        let (a, b, axis) = if horizontal {
            (p[0].x, p[1].x, p[0].y)
        } else {
            (p[0].y, p[1].y, p[0].x)
        };
        let (low, high) = if a < b {
            (a + radii[i], b.saturating_sub(radii[i + 1]))
        } else {
            (b + radii[i + 1], a.saturating_sub(radii[i]))
        };
        if low >= high {
            continue;
        }
        for (low, high) in
            index.visible(horizontal, axis as f64, low, high, edge.stroke, &edge.soft)
        {
            result.push(Segment {
                index: i,
                horizontal,
                axis,
                low,
                high,
            });
        }
    }
    result.sort_by_key(|s| (s.horizontal, s.axis, s.low, s.high, s.index));
    let mut merged: Vec<Segment> = Vec::new();
    for segment in result {
        if let Some(previous) = merged.last_mut() {
            if previous.horizontal == segment.horizontal
                && previous.axis == segment.axis
                && segment.low <= previous.high
            {
                previous.high = previous.high.max(segment.high);
                previous.index = previous.index.min(segment.index);
                continue;
            }
        }
        merged.push(segment);
    }
    merged
}

fn shared_sections(
    a: &Route,
    b: &Route,
    segment: Segment,
    low: usize,
    high: usize,
) -> Vec<(usize, usize, bool)> {
    if a.identity["source"] != b.identity["source"] && a.identity["target"] != b.identity["target"]
    {
        return vec![(low, high, false)];
    }
    let mut boundaries = vec![low, high];
    for p in a.path.iter().chain(&b.path) {
        let (axis, along) = if segment.horizontal {
            (p.y, p.x)
        } else {
            (p.x, p.y)
        };
        if axis == segment.axis && low < along && along < high {
            boundaries.push(along);
        }
    }
    boundaries.sort_unstable();
    boundaries.dedup();
    let mut sections: Vec<(usize, usize, bool)> = Vec::new();
    for interval in boundaries.windows(2) {
        let (low, high) = (interval[0], interval[1]);
        let is_shared = shared(a, b, segment, low, high);
        if let Some(previous) = sections.last_mut() {
            if previous.2 == is_shared {
                previous.1 = high;
                continue;
            }
        }
        sections.push((low, high, is_shared));
    }
    sections
}

fn shared(a: &Route, b: &Route, segment: Segment, low: usize, high: usize) -> bool {
    (a.identity["source"] == b.identity["source"]
        && spacing::shared_prefix(
            &a.path,
            &b.path,
            segment.horizontal,
            segment.axis,
            low,
            high,
        ))
        || (a.identity["target"] == b.identity["target"]
            && spacing::shared_prefix(
                &a.path.iter().rev().copied().collect::<Vec<_>>(),
                &b.path.iter().rev().copied().collect::<Vec<_>>(),
                segment.horizontal,
                segment.axis,
                low,
                high,
            ))
}

fn trim_endpoint(edge: &Route, segment: Segment, low: &mut usize, high: &mut usize) {
    for p in [edge.path.first(), edge.path.last()].into_iter().flatten() {
        let (axis, along) = if segment.horizontal {
            (p.y, p.x)
        } else {
            (p.x, p.y)
        };
        if axis != segment.axis {
            continue;
        }
        if along <= *low {
            *low = (*low).max(along.saturating_add(32));
        }
        if along >= *high {
            *high = (*high).min(along.saturating_sub(32));
        }
    }
}

#[cfg(test)]
#[path = "../../tests/unit/layout/diagnostics.rs"]
mod tests;
