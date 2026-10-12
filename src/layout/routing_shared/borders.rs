use super::scoring::Occluder;
use crate::layout::{Bounds, Layout, Point, Side};
use crate::model::{Graph, Placement};
use crate::view::geometry;
use std::collections::{BTreeMap, BTreeSet};

pub(in crate::layout) const MINIMUM: usize = 4;
pub(in crate::layout) const PREFERRED: usize = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::layout) struct Frame {
    pub bounds: Bounds,
    pub radius: usize,
    pub stroke: usize,
    pub node: Option<usize>,
}

impl Frame {
    pub fn card(bounds: Bounds) -> Self {
        Self {
            bounds,
            radius: geometry::CARD_RADIUS,
            stroke: geometry::CARD_STROKE,
            node: None,
        }
    }

    pub fn clearance(self, stroke: usize, gap: usize) -> usize {
        (gap * 4 + (stroke + self.stroke) / 2).div_ceil(4)
    }
}

pub(in crate::layout) fn frames(graph: &Graph, layout: &Layout<'_>) -> Vec<Frame> {
    let mut result: Vec<_> = graph
        .nodes
        .iter()
        .zip(&layout.bounds)
        .enumerate()
        .filter(|(_, (node, bounds))| {
            node.placement != Placement::Logical && bounds.width > 0 && bounds.height > 0
        })
        .map(|(node, (resource, &bounds))| {
            let container = layout.containers.contains(&node);
            let embedded = crate::view::routes::owner(graph, resource).is_some();
            let (radius, stroke) = if container {
                (geometry::CONTAINER_RADIUS, geometry::CONTAINER_STROKE)
            } else if embedded {
                (geometry::ROUTE_RADIUS, geometry::ROUTE_STROKE)
            } else {
                (geometry::CARD_RADIUS, geometry::CARD_STROKE)
            };
            Frame {
                bounds,
                radius,
                stroke,
                node: Some(node),
            }
        })
        .collect();
    for &parent in &layout.containers {
        let bounds = layout.bounds[parent];
        let mut y =
            bounds.origin.y + crate::view::card::measure(&graph.nodes[parent]).routes_y + 12;
        for card in crate::view::routes::cards(graph, parent) {
            let height = card.height();
            if card.node.is_none() {
                result.push(Frame {
                    bounds: Bounds {
                        origin: Point {
                            x: bounds.origin.x + 12,
                            y,
                        },
                        width: crate::view::routes::WIDTH,
                        height,
                    },
                    radius: geometry::ROUTE_RADIUS,
                    stroke: geometry::ROUTE_STROKE,
                    node: None,
                });
            }
            y += height + 12;
        }
    }
    result
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(in crate::layout) struct Metrics {
    pub minimum: Option<i64>,
    pub close_length: usize,
    pub minimum_deficit: u128,
    pub preferred_deficit: u128,
    weighted: u128,
}

impl Metrics {
    pub fn cost(&self) -> u128 {
        self.weighted
    }

    fn observe(&mut self, gap: i64, length: usize) {
        if gap > (PREFERRED * 4) as i64 {
            return;
        }
        self.minimum = Some(self.minimum.map_or(gap, |old| old.min(gap)));
        let minimum = ((MINIMUM * 4) as i64 - gap).max(0) as u128;
        let preferred = ((PREFERRED * 4) as i64 - gap).max(0) as u128;
        self.close_length += usize::from(preferred > 0) * length;
        self.minimum_deficit += minimum * length as u128;
        self.preferred_deficit += preferred * length as u128;
        self.weighted += (minimum + preferred.div_ceil(16)) * length as u128;
    }

    fn merge(&mut self, other: Self) {
        if let Some(gap) = other.minimum {
            self.minimum = Some(self.minimum.map_or(gap, |old| old.min(gap)));
        }
        self.close_length += other.close_length;
        self.minimum_deficit += other.minimum_deficit;
        self.preferred_deficit += other.preferred_deficit;
        self.weighted += other.weighted;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::layout) struct Contact {
    pub segment: usize,
    pub horizontal: bool,
    pub coordinate: usize,
    pub low: usize,
    pub high: usize,
    pub frame: Frame,
    pub side: Side,
    pub minimum: usize,
    pub preferred: usize,
    pub urgent: bool,
}

#[derive(Default)]
pub(in crate::layout) struct Index {
    frames: Vec<Frame>,
    horizontal: BTreeMap<usize, Vec<(usize, Side)>>,
    vertical: BTreeMap<usize, Vec<(usize, Side)>>,
    radii: BTreeMap<(usize, usize, usize, usize), usize>,
    reach: usize,
}

impl Index {
    pub fn new(mut frames: Vec<Frame>) -> Self {
        frames.retain(|frame| frame.bounds.width > 0 && frame.bounds.height > 0);
        let mut result = Self {
            frames,
            ..Self::default()
        };
        for (index, frame) in result.frames.iter().enumerate() {
            let b = frame.bounds;
            result
                .radii
                .insert((b.origin.x, b.origin.y, b.width, b.height), frame.radius);
            result
                .horizontal
                .entry(b.origin.y)
                .or_default()
                .push((index, Side::Top));
            result
                .horizontal
                .entry(b.origin.y + b.height)
                .or_default()
                .push((index, Side::Bottom));
            result
                .vertical
                .entry(b.origin.x)
                .or_default()
                .push((index, Side::Left));
            result
                .vertical
                .entry(b.right())
                .or_default()
                .push((index, Side::Right));
            result.reach = result.reach.max(frame.radius + frame.stroke.div_ceil(8));
        }
        result
    }

    pub fn frames(&self) -> &[Frame] {
        &self.frames
    }

    pub fn axes(&self, start: Point, end: Point, stroke: usize) -> Vec<(bool, usize)> {
        let rectangle = (
            start.x.min(end.x),
            start.x.max(end.x),
            start.y.min(end.y),
            start.y.max(end.y),
        );
        let interval_distance =
            |a: usize, b: usize, c: usize, d: usize| a.saturating_sub(d).max(c.saturating_sub(b));
        let mut candidates = [BTreeMap::<usize, usize>::new(), BTreeMap::new()];
        for frame in &self.frames {
            let b = frame.bounds;
            let dx = interval_distance(b.origin.x, b.right(), rectangle.0, rectangle.1);
            let dy = interval_distance(b.origin.y, b.origin.y + b.height, rectangle.2, rectangle.3);
            if dx > 128 || dy > 128 {
                continue;
            }
            for (axis, low, high, from, to) in [
                (0, b.origin.x, b.right(), rectangle.0, rectangle.1),
                (
                    1,
                    b.origin.y,
                    b.origin.y + b.height,
                    rectangle.2,
                    rectangle.3,
                ),
            ] {
                for gap in [MINIMUM, PREFERRED] {
                    let offset = frame.clearance(stroke, gap);
                    for coordinate in [
                        low.saturating_sub(offset),
                        low.saturating_add(offset),
                        high.saturating_sub(offset),
                        high.saturating_add(offset),
                    ] {
                        let distance = interval_distance(coordinate, coordinate, from, to);
                        if distance <= 128 {
                            candidates[axis]
                                .entry(coordinate)
                                .and_modify(|old| *old = (*old).min(dx + dy + distance))
                                .or_insert(dx + dy + distance);
                        }
                    }
                }
            }
        }
        let mut result = Vec::new();
        for (axis, candidates) in candidates.into_iter().enumerate() {
            let mut candidates: Vec<_> = candidates.into_iter().collect();
            candidates.sort_by_key(|&(coordinate, distance)| (distance, coordinate));
            result.extend(
                candidates
                    .into_iter()
                    .take(32)
                    .map(|(coordinate, _)| (axis == 1, coordinate)),
            );
        }
        result.sort_unstable();
        result
    }

    fn visible(
        &self,
        horizontal: bool,
        coordinate: f64,
        low: usize,
        high: usize,
        stroke: usize,
        soft: &[Occluder],
    ) -> Vec<(usize, usize)> {
        let mut hidden = Vec::new();
        for occluder in soft {
            let Some((bounds, radius)) = self.cover(occluder) else {
                continue;
            };
            let Some((from, to)) =
                bounds.interval(horizontal, coordinate, radius, stroke as f64 / 8.0 + 0.5)
            else {
                continue;
            };
            let from = ((from - 0.5).ceil().max(0.0) as usize).max(low);
            let to = (((to - 0.5).floor() + 1.0).max(0.0) as usize).min(high);
            if from < to {
                hidden.push((from, to));
            }
        }
        hidden.sort_unstable();
        let mut result = Vec::new();
        let mut cursor = low;
        for (from, to) in hidden {
            if from > cursor {
                result.push((cursor, from));
            }
            cursor = cursor.max(to);
        }
        if cursor < high {
            result.push((cursor, high));
        }
        result
    }

    fn cover(&self, occluder: &Occluder) -> Option<(Rectangle, f64)> {
        if occluder.step == 2 {
            let padding = geometry::MASK_PADDING;
            let b = occluder.bounds;
            Some((
                Rectangle::from(Bounds {
                    origin: Point {
                        x: b.origin.x.saturating_sub(padding),
                        y: b.origin.y.saturating_sub(padding),
                    },
                    width: b.width + 2 * padding,
                    height: b.height + 2 * padding,
                }),
                (geometry::CONTAINER_RADIUS + padding) as f64,
            ))
        } else if occluder.step == 1 && occluder.cost == 2 {
            let b = occluder.bounds;
            let radius = self
                .radii
                .get(&(b.origin.x, b.origin.y, b.width, b.height))
                .copied()
                .unwrap_or(geometry::CARD_RADIUS);
            Some((Rectangle::from(occluder.bounds), radius as f64))
        } else {
            None
        }
    }

    fn visit(
        &self,
        a: Position,
        b: Position,
        stroke: usize,
        soft: &[Occluder],
        mut visitor: impl FnMut(Frame, Side, usize, usize, i64),
    ) {
        if a == b || (a.x != b.x && a.y != b.y) {
            return;
        }
        let horizontal = a.y == b.y;
        let (coordinate, low, high) = if horizontal {
            (a.y, a.x.min(b.x), a.x.max(b.x))
        } else {
            (a.x, a.y.min(b.y), a.y.max(b.y))
        };
        let reach = self.reach + PREFERRED + stroke.div_ceil(8);
        let center = coordinate as usize;
        let index = if horizontal {
            &self.horizontal
        } else {
            &self.vertical
        };
        let mut nearby = index
            .range(center.saturating_sub(reach)..=center.saturating_add(reach))
            .peekable();
        if nearby.peek().is_none() {
            return;
        }
        let visible = self.visible(
            horizontal,
            coordinate,
            low.floor() as usize,
            high.ceil() as usize,
            stroke,
            soft,
        );
        for (&axis, sides) in nearby {
            for &(frame_index, side) in sides {
                let frame = self.frames[frame_index];
                let bounds = frame.bounds;
                let radius = frame.radius.min(bounds.width / 2).min(bounds.height / 2);
                let (from, to) = if horizontal {
                    (bounds.origin.x, bounds.right())
                } else {
                    (bounds.origin.y, bounds.origin.y + bounds.height)
                };
                let padding = frame.clearance(stroke, PREFERRED) + 1;
                let first = low.max(from.saturating_sub(padding) as f64).floor() as usize;
                let last = high.min(to.saturating_add(padding) as f64).ceil() as usize;
                if first >= last {
                    continue;
                }
                let gap = ((coordinate - axis as f64).abs() * 4.0).floor() as i64
                    - ((stroke + frame.stroke) / 2) as i64;
                for &(visible_low, visible_high) in &visible {
                    let visible_low = visible_low.max(first);
                    let visible_high = visible_high.min(last);
                    if visible_low >= visible_high {
                        continue;
                    }
                    let straight_low = visible_low.max(from + radius);
                    let straight_high = visible_high.min(to - radius);
                    if straight_low < straight_high && gap <= (PREFERRED * 4) as i64 {
                        visitor(frame, side, straight_low, straight_high, gap);
                    }
                    for (lo, hi, along, sign) in [
                        (
                            visible_low,
                            visible_high.min(from + radius),
                            from + radius,
                            -1.0,
                        ),
                        (visible_low.max(to - radius), visible_high, to - radius, 1.0),
                    ] {
                        let outward = if matches!(side, Side::Top | Side::Left) {
                            -1.0
                        } else {
                            1.0
                        };
                        let normal = axis as f64 - outward * radius as f64;
                        for pixel in lo..hi {
                            let a = (pixel as f64 - along as f64) * sign;
                            let b = ((pixel + 1) as f64 - along as f64) * sign;
                            let (near, far) = (a.min(b), a.max(b));
                            let v = (coordinate - normal) * outward;
                            let radius = radius as f64;
                            let distance = if near >= 0.0 && v >= 0.0 {
                                (near.hypot(v) - radius).max(radius - far.hypot(v)).max(0.0)
                            } else {
                                (radius.clamp(near, far) - radius)
                                    .hypot(v)
                                    .min(0.0f64.clamp(near, far).hypot(v - radius))
                            };
                            let gap = (distance * 4.0).floor() as i64
                                - ((stroke + frame.stroke) / 2) as i64;
                            if gap <= (PREFERRED * 4) as i64 {
                                visitor(frame, side, pixel, pixel + 1, gap);
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn segment(&self, a: Point, b: Point, stroke: usize, soft: &[Occluder]) -> Metrics {
        let mut result = Metrics::default();
        self.visit(a.into(), b.into(), stroke, soft, |_, _, low, high, gap| {
            result.observe(gap, high - low)
        });
        result
    }

    pub fn contacts(&self, path: &[Point], stroke: usize, soft: &[Occluder]) -> Vec<Contact> {
        let mut result: Vec<Contact> = Vec::new();
        for (segment, p) in path.windows(2).enumerate() {
            let horizontal = p[0].y == p[1].y;
            let coordinate = if horizontal { p[0].y } else { p[0].x };
            self.visit(
                p[0].into(),
                p[1].into(),
                stroke,
                soft,
                |frame, side, low, high, gap| {
                    if gap >= (PREFERRED * 4) as i64 {
                        return;
                    }
                    if let Some(previous) = result.last_mut() {
                        if previous.segment == segment
                            && previous.frame == frame
                            && previous.side == side
                            && previous.high == low
                        {
                            previous.high = high;
                            previous.urgent |= gap < (MINIMUM * 4) as i64;
                            return;
                        }
                    }
                    result.push(Contact {
                        segment,
                        horizontal,
                        coordinate,
                        low,
                        high,
                        frame,
                        side,
                        minimum: frame.clearance(stroke, MINIMUM),
                        preferred: frame.clearance(stroke, PREFERRED),
                        urgent: gap < (MINIMUM * 4) as i64,
                    });
                },
            );
        }
        for (segment, p) in path.windows(3).enumerate() {
            if p[0] == p[1]
                || p[1] == p[2]
                || !((p[0].x == p[1].x && p[1].y == p[2].y)
                    || (p[0].y == p[1].y && p[1].x == p[2].x))
            {
                continue;
            }
            let distance = |a: Point, b: Point| a.x.abs_diff(b.x) + a.y.abs_diff(b.y);
            let radius = geometry::relationship_radius(distance(p[0], p[1]), distance(p[1], p[2]));
            let corner = Position::from(p[1]);
            self.curve(
                corner.toward(p[0].into(), radius),
                corner,
                corner.toward(p[2].into(), radius),
                stroke,
                soft,
                |frame, point, gap| {
                    if gap >= (PREFERRED * 4) as i64 {
                        return;
                    }
                    for (segment, other) in [(segment, p[0]), (segment + 1, p[2])] {
                        let horizontal = other.y == p[1].y;
                        let (
                            coordinate,
                            along,
                            end,
                            normal,
                            low_side,
                            high_side,
                            low_axis,
                            high_axis,
                        ) = if horizontal {
                            (
                                p[1].y,
                                p[1].x,
                                other.x,
                                point.y,
                                Side::Top,
                                Side::Bottom,
                                frame.bounds.origin.y,
                                frame.bounds.origin.y + frame.bounds.height,
                            )
                        } else {
                            (
                                p[1].x,
                                p[1].y,
                                other.y,
                                point.x,
                                Side::Left,
                                Side::Right,
                                frame.bounds.origin.x,
                                frame.bounds.right(),
                            )
                        };
                        let (side, axis) = if (normal - low_axis as f64).abs()
                            <= (normal - high_axis as f64).abs()
                        {
                            (low_side, low_axis)
                        } else {
                            (high_side, high_axis)
                        };
                        if (normal - axis as f64).abs()
                            > (frame.radius + frame.clearance(stroke, PREFERRED)) as f64
                        {
                            continue;
                        }
                        let extra = radius.ceil() as usize;
                        let (low, high) = if along < end {
                            (along, (along + extra).min(end))
                        } else {
                            (along.saturating_sub(extra).max(end), along)
                        };
                        result.push(Contact {
                            segment,
                            horizontal,
                            coordinate,
                            low,
                            high,
                            frame,
                            side,
                            minimum: frame.clearance(stroke, MINIMUM) + extra,
                            preferred: frame.clearance(stroke, PREFERRED) + extra,
                            urgent: gap < (MINIMUM * 4) as i64,
                        });
                    }
                },
            );
        }
        result.sort_by_key(|c| {
            (
                c.segment,
                c.side as usize,
                c.frame.node,
                c.frame.bounds.origin.x,
                c.frame.bounds.origin.y,
                c.low,
            )
        });
        let mut merged: Vec<Contact> = Vec::new();
        for contact in result {
            if let Some(previous) = merged.last_mut() {
                if previous.segment == contact.segment
                    && previous.frame == contact.frame
                    && previous.side == contact.side
                    && previous.high >= contact.low
                {
                    previous.high = previous.high.max(contact.high);
                    previous.minimum = previous.minimum.max(contact.minimum);
                    previous.preferred = previous.preferred.max(contact.preferred);
                    previous.urgent |= contact.urgent;
                    continue;
                }
            }
            merged.push(contact);
        }
        merged
    }

    pub fn path(&self, path: &[Point], stroke: usize, soft: &[Occluder]) -> Metrics {
        if self.frames.is_empty() || path.len() < 2 {
            return Metrics::default();
        }
        let mut points = path.to_vec();
        points.dedup();
        let mut result = Metrics::default();
        let mut radii = vec![0.0; points.len()];
        for (i, p) in points.windows(3).enumerate() {
            if (p[0].x == p[1].x && p[1].y == p[2].y) || (p[0].y == p[1].y && p[1].x == p[2].x) {
                let distance = |a: Point, b: Point| a.x.abs_diff(b.x) + a.y.abs_diff(b.y);
                radii[i + 1] =
                    geometry::relationship_radius(distance(p[0], p[1]), distance(p[1], p[2]));
            }
        }
        for (i, p) in points.windows(2).enumerate() {
            let a = Position::from(p[0]).toward(p[1].into(), radii[i]);
            let b = Position::from(p[1]).toward(p[0].into(), radii[i + 1]);
            self.visit(a, b, stroke, soft, |_, _, low, high, gap| {
                result.observe(gap, high - low)
            });
        }
        for (i, p) in points.windows(3).enumerate() {
            if radii[i + 1] == 0.0 {
                continue;
            }
            let corner = Position::from(p[1]);
            let enter = corner.toward(p[0].into(), radii[i + 1]);
            let exit = corner.toward(p[2].into(), radii[i + 1]);
            result.merge(self.curve(enter, corner, exit, stroke, soft, |_, _, _| {}));
        }
        result
    }

    fn curve(
        &self,
        enter: Position,
        corner: Position,
        exit: Position,
        stroke: usize,
        soft: &[Occluder],
        mut observer: impl FnMut(Frame, Position, i64),
    ) -> Metrics {
        let mut frames = BTreeSet::new();
        let reach = self.reach + PREFERRED + stroke.div_ceil(8);
        for (index, low, high) in [
            (
                &self.horizontal,
                enter.y.min(exit.y).min(corner.y),
                enter.y.max(exit.y).max(corner.y),
            ),
            (
                &self.vertical,
                enter.x.min(exit.x).min(corner.x),
                enter.x.max(exit.x).max(corner.x),
            ),
        ] {
            for (_, entries) in index.range(
                (low as usize).saturating_sub(reach)..=(high.ceil() as usize).saturating_add(reach),
            ) {
                frames.extend(entries.iter().map(|&(frame, _)| frame));
            }
        }
        frames.retain(|&index| {
            let frame = self.frames[index];
            let b = Rectangle::from(frame.bounds);
            let padding = frame.clearance(stroke, PREFERRED) as f64 + 1.0;
            b.left <= enter.x.max(exit.x).max(corner.x) + padding
                && b.right + padding >= enter.x.min(exit.x).min(corner.x)
                && b.top <= enter.y.max(exit.y).max(corner.y) + padding
                && b.bottom + padding >= enter.y.min(exit.y).min(corner.y)
        });
        if frames.is_empty() {
            return Metrics::default();
        }
        let covers: Vec<_> = soft
            .iter()
            .filter_map(|o| self.cover(o))
            .filter(|(bounds, _)| {
                bounds.left <= enter.x.max(exit.x).max(corner.x)
                    && bounds.right >= enter.x.min(exit.x).min(corner.x)
                    && bounds.top <= enter.y.max(exit.y).max(corner.y)
                    && bounds.bottom >= enter.y.min(exit.y).min(corner.y)
            })
            .collect();
        let point = |t: f64| Position {
            x: (1.0 - t).powi(2) * enter.x + 2.0 * (1.0 - t) * t * corner.x + t * t * exit.x,
            y: (1.0 - t).powi(2) * enter.y + 2.0 * (1.0 - t) * t * corner.y + t * t * exit.y,
        };
        let mut result = Metrics::default();
        let mut previous = enter;
        let mut total = 0.0;
        let mut counted = 0;
        for sample in 0..32 {
            let end = point((sample + 1) as f64 / 32.0);
            total += (end.x - previous.x).hypot(end.y - previous.y);
            let length = if sample == 31 {
                total.ceil() as usize
            } else {
                total.floor() as usize
            };
            let weight = length - counted;
            counted = length;
            previous = end;
            let p = point((sample as f64 + 0.5) / 32.0);
            if covers.iter().any(|&(bounds, radius)| {
                bounds.distance(p, radius) <= -(stroke as f64 / 8.0 + 0.25)
            }) {
                continue;
            }
            for &index in &frames {
                let frame = self.frames[index];
                let distance = Rectangle::from(frame.bounds)
                    .distance(p, frame.radius as f64)
                    .abs();
                let gap = ((distance - 0.25).max(0.0) * 4.0).floor() as i64
                    - ((stroke + frame.stroke) / 2) as i64;
                result.observe(gap, weight);
                if gap <= (PREFERRED * 4) as i64 {
                    observer(frame, p, gap);
                }
            }
        }
        result
    }
}

#[derive(Clone, Copy, PartialEq)]
struct Position {
    x: f64,
    y: f64,
}

impl From<Point> for Position {
    fn from(point: Point) -> Self {
        Self {
            x: point.x as f64,
            y: point.y as f64,
        }
    }
}

impl Position {
    fn toward(self, other: Self, amount: f64) -> Self {
        let offset = |a: f64, b: f64| {
            if a < b {
                a + amount
            } else if a > b {
                a - amount
            } else {
                a
            }
        };
        Self {
            x: offset(self.x, other.x),
            y: offset(self.y, other.y),
        }
    }
}

#[derive(Clone, Copy)]
struct Rectangle {
    left: f64,
    right: f64,
    top: f64,
    bottom: f64,
}

impl From<Bounds> for Rectangle {
    fn from(b: Bounds) -> Self {
        Self {
            left: b.origin.x as f64,
            right: b.right() as f64,
            top: b.origin.y as f64,
            bottom: (b.origin.y + b.height) as f64,
        }
    }
}

impl Rectangle {
    fn distance(self, point: Position, radius: f64) -> f64 {
        let width = (self.right - self.left) / 2.0;
        let height = (self.bottom - self.top) / 2.0;
        let radius = radius.min(width).min(height);
        let x = (point.x - (self.left + width)).abs() - (width - radius);
        let y = (point.y - (self.top + height)).abs() - (height - radius);
        x.max(0.0).hypot(y.max(0.0)) + x.max(y).min(0.0) - radius
    }

    fn interval(
        self,
        horizontal: bool,
        coordinate: f64,
        radius: f64,
        inset: f64,
    ) -> Option<(f64, f64)> {
        let (low, high, near, far) = if horizontal {
            (self.left, self.right, self.top, self.bottom)
        } else {
            (self.top, self.bottom, self.left, self.right)
        };
        let radius = radius.min((high - low) / 2.0).min((far - near) / 2.0);
        let (low, high, near, far) = (low + inset, high - inset, near + inset, far - inset);
        if low >= high || near > far || coordinate < near || coordinate > far {
            return None;
        }
        let radius = (radius - inset).max(0.0);
        let distance = (near + radius - coordinate)
            .max(coordinate - (far - radius))
            .max(0.0);
        let offset = radius - (radius * radius - distance * distance).max(0.0).sqrt();
        Some((low + offset, high - offset))
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/layout/routing_shared/borders.rs"]
mod tests;
