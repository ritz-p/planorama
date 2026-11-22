use super::{Key, Snapshot};
use crate::svg::escape;
use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet, BinaryHeap};
use std::fmt::Write;
use std::{fs, io, path::Path};

#[derive(Default)]
struct Slot {
    parent: Option<usize>,
    depth: usize,
    children: Vec<usize>,
    label_width: usize,
    top: usize,
    bottom: usize,
    ports: usize,
}

#[derive(Clone, Copy, PartialOrd, Ord, PartialEq, Eq)]
struct Edge {
    from: usize,
    to: usize,
    kind: &'static str,
    direction: &'static str,
}

struct Diagram {
    cards: BTreeMap<usize, String>,
    edges: BTreeSet<Edge>,
    nested: BTreeSet<Edge>,
}

#[derive(Default)]
struct Layout {
    slots: Vec<Slot>,
    by_parent_and_identity: BTreeMap<(Option<usize>, usize), usize>,
    roots: Vec<usize>,
    card_right: usize,
    height: usize,
    lanes: usize,
    routes: BTreeMap<Edge, (usize, usize, usize)>,
}

fn label(key: &Key, ordinal: usize, anonymous: bool) -> String {
    if anonymous {
        return format!(
            "{} {}",
            if key.component {
                "Component"
            } else {
                "Resource"
            },
            ordinal + 1
        );
    }
    let mut name = format!("{} · {}", key.state, key.address);
    if let Some(deposed) = &key.deposed {
        write!(name, " · deposed {deposed}").unwrap();
    }
    name
}

impl Layout {
    fn add(
        &mut self,
        snapshot: &Snapshot,
        identities: &BTreeMap<&Key, usize>,
        anonymous: bool,
    ) -> Diagram {
        let mut children = BTreeMap::<Option<&Key>, Vec<&Key>>::new();
        for key in snapshot.nodes.keys() {
            children
                .entry(snapshot.parents.get(key))
                .or_default()
                .push(key);
        }
        let mut stack: Vec<_> = children
            .remove(&None)
            .unwrap_or_default()
            .into_iter()
            .rev()
            .map(|key| (None, key))
            .collect();
        let mut indices = BTreeMap::new();
        let mut cards = BTreeMap::new();
        while let Some((parent, key)) = stack.pop() {
            let identity = identities[key];
            let slot = *self
                .by_parent_and_identity
                .entry((parent, identity))
                .or_insert_with(|| {
                    let index = self.slots.len();
                    let depth = if let Some(p) = parent {
                        self.slots[p].children.push(index);
                        self.slots[p].depth + 1
                    } else {
                        self.roots.push(index);
                        0
                    };
                    self.slots.push(Slot {
                        parent,
                        depth,
                        ..Slot::default()
                    });
                    index
                });
            let label = label(key, identity, anonymous);
            let width = crate::view::text::width(&label) * 9 + 36;
            self.slots[slot].label_width = self.slots[slot].label_width.max(width);
            indices.insert(key, slot);
            cards.insert(slot, label);
            for child in children
                .remove(&Some(key))
                .unwrap_or_default()
                .into_iter()
                .rev()
            {
                stack.push((Some(slot), child));
            }
        }
        let mut edges = BTreeSet::new();
        let mut nested = BTreeSet::new();
        for link in &snapshot.edges {
            let mut edge = Edge {
                from: indices[&link.from],
                to: indices[&link.to],
                kind: link.kind,
                direction: link.direction,
            };
            if edge.direction == "undirected" && edge.from > edge.to {
                std::mem::swap(&mut edge.from, &mut edge.to);
            }
            if edge.kind == "containment" && self.slots[edge.to].parent == Some(edge.from) {
                nested.insert(edge);
            } else {
                edges.insert(edge);
            }
        }
        Diagram {
            cards,
            edges,
            nested,
        }
    }

    fn place(&mut self, before: &Diagram, after: &Diagram) {
        let mut ports = BTreeMap::new();
        for &edge in before.edges.union(&after.edges) {
            let start = self.slots[edge.from].ports;
            self.slots[edge.from].ports += 1;
            let end = self.slots[edge.to].ports;
            self.slots[edge.to].ports += 1;
            ports.insert(edge, (start, end));
        }
        let mut y = 56;
        let mut stack: Vec<_> = self.roots.iter().rev().map(|&i| (i, false)).collect();
        self.card_right = 220;
        while let Some((i, leave)) = stack.pop() {
            let slot = &mut self.slots[i];
            if leave {
                slot.bottom = y;
                y += 12;
                continue;
            }
            slot.top = y;
            self.card_right = self.card_right.max(32 + slot.depth * 28 + slot.label_width);
            y += (slot.ports * 6 + 16).max(38);
            stack.push((i, true));
            stack.extend(slot.children.iter().rev().map(|&c| (c, false)));
        }
        self.height = y + 16;
        let mut spans: Vec<_> = ports
            .into_iter()
            .map(|(edge, (a, b))| {
                let start = self.slots[edge.from].top + 10 + a * 6;
                let end = self.slots[edge.to].top + 10 + b * 6;
                (start.min(end), start.max(end), edge, start, end)
            })
            .collect();
        spans.sort_unstable();
        let mut active = BinaryHeap::new();
        for (low, high, edge, start, end) in spans {
            let lane = match active.peek().copied() {
                Some(Reverse((last, lane))) if last + 4 < low => {
                    active.pop();
                    lane
                }
                _ => {
                    self.lanes += 1;
                    self.lanes
                }
            };
            active.push(Reverse((high, lane)));
            self.routes.insert(edge, (lane, start, end));
        }
    }

    fn render(&self, diagram: &Diagram, name: &str) -> String {
        let width = self.card_right + self.lanes * 14 + 44;
        let height = self.height;
        let mut svg = format!(
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}" role="group" aria-labelledby="title" font-family="monospace">
<title id="title">{name} architecture</title>
<defs><marker id="arrow" viewBox="0 0 8 8" refX="7" refY="4" markerWidth="6" markerHeight="6" orient="auto"><path d="M0 0L8 4L0 8z" fill="#475569"/></marker></defs>
<rect width="100%" height="100%" fill="white"/><text x="20" y="32" font-size="20" fill="#0f172a">{name} architecture</text>
"##
        );
        for (&id, label) in &diagram.cards {
            let slot = &self.slots[id];
            let x = 20 + 18 * slot.depth;
            let right = self.card_right - 10 * slot.depth;
            let y = slot.top;
            let parent = slot
                .parent
                .map(|p| format!(" data-parent=\"slot-{p}\""))
                .unwrap_or_default();
            let display = escape(label);
            let fill = if slot.children.is_empty() {
                "#ffffff"
            } else {
                "#eef6fa"
            };
            writeln!(svg, r##"<g id="slot-{id}"{parent} role="group"><title>{display}</title><rect x="{x}" y="{y}" width="{}" height="{}" rx="5" fill="{fill}" stroke="#94a3b8"/><text x="{}" y="{}" font-size="14" fill="#0f172a">{display}</text></g>"##,
                right-x,slot.bottom-y,x+10,y+24).unwrap();
        }
        for edge in diagram.edges.union(&diagram.nested) {
            let Edge {
                from,
                to,
                kind,
                direction,
            } = *edge;
            let title = format!(
                "{} · {kind} ({direction}) · {}",
                escape(&diagram.cards[&from]),
                escape(&diagram.cards[&to])
            );
            write!(svg,r#"<g data-edge-kind="{kind}" data-directionality="{direction}" data-source="slot-{from}" data-target="slot-{to}"><title>{title}</title>"#).unwrap();
            if let Some(&(lane, y1, y2)) = self.routes.get(edge) {
                let x1 = self.card_right - 10 * self.slots[from].depth;
                let x2 = self.card_right - 10 * self.slots[to].depth;
                let x = self.card_right + 14 * lane;
                let arrow = if direction == "directed" {
                    r#" marker-end="url(#arrow)""#
                } else {
                    ""
                };
                let (color, dash) = match kind {
                    "association" => ("#7c3aed", "6 4"),
                    "connection" => ("#0f766e", "2 3"),
                    "containment" => ("#047857", "4 2"),
                    _ => ("#475569", "none"),
                };
                write!(svg,r#"<path d="M{x1} {y1}H{x}V{y2}H{x2}" fill="none" stroke="{color}" stroke-dasharray="{dash}"{arrow}/>"#).unwrap();
            }
            svg.push_str("</g>\n");
        }
        svg.push_str("</svg>\n");
        svg
    }
}

pub(super) fn write(
    dir: &Path,
    before: &Snapshot,
    after: &Snapshot,
    mapping: &BTreeMap<Key, Key>,
    anonymous: bool,
) -> io::Result<()> {
    if dir == Path::new("-") {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "--svg-dir requires a directory path",
        ));
    }
    match fs::symlink_metadata(dir) {
        Ok(_) => {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "--svg-dir requires a new directory",
            ));
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => {}
        Err(e) => return Err(e),
    }
    let old: BTreeMap<_, _> = before
        .nodes
        .keys()
        .enumerate()
        .map(|(i, k)| (k, i))
        .collect();
    let mut new: BTreeMap<_, _> = mapping.iter().map(|(a, b)| (b, old[a])).collect();
    let mut next = old.len();
    for key in after.nodes.keys() {
        new.entry(key).or_insert_with(|| {
            let id = next;
            next += 1;
            id
        });
    }
    let mut layout = Layout::default();
    let left = layout.add(before, &old, anonymous);
    let right = layout.add(after, &new, anonymous);
    layout.place(&left, &right);
    let parent = dir
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let temp = tempfile::tempdir_in(parent)?;
    fs::write(
        temp.path().join("before.svg"),
        layout.render(&left, "Before"),
    )?;
    fs::write(
        temp.path().join("after.svg"),
        layout.render(&right, "After"),
    )?;
    fs::rename(temp.path(), dir)
}
