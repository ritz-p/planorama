use crate::model::{Graph, Node, RouteEntry, TerraformEntityId};

pub(crate) const WIDTH: usize = crate::layout::NODE_WIDTH - 24;
pub(crate) const LINE_LIMIT: usize = (WIDTH - 24) / 8;

pub(crate) struct Row {
    pub text: String,
    pub qualified_text: Option<String>,
    pub target: Option<TerraformEntityId>,
    pub moved: bool,
    pub destination: bool,
    address: bool,
}

impl Row {
    pub(crate) fn lines<'a>(&self, text: &'a str) -> Vec<&'a str> {
        if self.address {
            super::text::wrap_address(text, LINE_LIMIT)
        } else {
            super::text::wrap(text, LINE_LIMIT)
        }
    }
}

pub(crate) struct Card<'a> {
    pub node: Option<&'a Node>,
    pub rows: Vec<Row>,
    pub inline: bool,
}

impl Card<'_> {
    pub fn type_heading(&self) -> Option<String> {
        self.node
            .map(|node| super::heading::text(node, true, node.member_owner.is_some()))
    }

    pub fn type_heading_height(&self) -> usize {
        self.type_heading().map_or(0, |heading| {
            super::heading::lines(&heading, LINE_LIMIT).len() * super::heading::LINE_HEIGHT + 8
        })
    }

    pub fn badge_height(&self) -> usize {
        self.node.map_or(0, |node| {
            super::card::import_height(node)
                + if node.member_owner.is_some() {
                    crate::services::display_name(node).map_or(32, |name| {
                        super::service_label::height(name, WIDTH - 24).max(32)
                    })
                } else {
                    0
                }
        })
    }

    pub fn height(&self) -> usize {
        24 + self.type_heading_height()
            + self.badge_height()
            + self
                .node
                .filter(|node| node.member_owner.is_some())
                .map_or(0, |node| {
                    super::references::height_at_width(node, WIDTH - 24)
                })
            + self
                .rows
                .iter()
                .enumerate()
                .map(|(index, r)| {
                    let lines = r.lines(&r.text).len().max(
                        r.qualified_text
                            .as_deref()
                            .map_or(0, |text| r.lines(text).len()),
                    );
                    let lines = if index == 0 && self.node.is_some() {
                        lines.max(2)
                    } else {
                        lines
                    };
                    lines * 18
                })
                .sum::<usize>()
    }
}

pub(crate) fn heading(node: &Node, qualified: bool) -> String {
    let address = if qualified {
        node.resource_address().qualified()
    } else {
        std::borrow::Cow::Borrowed(node.address.as_str())
    };
    address.into_owned()
}

pub(crate) fn owner(graph: &Graph, node: &Node) -> Option<usize> {
    super::presentation::owner(graph, node)
}

pub(crate) fn routed(graph: &Graph, index: usize) -> bool {
    owner(graph, &graph.nodes[index]).is_some()
        && graph.edges.iter().any(|e| {
            e.from == index
                && e.kind == crate::model::EdgeKind::Association
                && graph.nodes[e.to].placement != crate::model::Placement::Logical
        })
}

pub(crate) fn insets(graph: &Graph) -> Vec<usize> {
    let mut insets = vec![12; graph.nodes.len()];
    for (index, entry) in crate::layout::relationship_markers::entries(graph)
        .into_iter()
        .enumerate()
    {
        let inset = crate::layout::relationship_markers::Terminal::marker(
            index + 1,
            crate::layout::Side::Right,
        )
        .clearance;
        for edge in entry.edges {
            let edge = &graph.edges[edge];
            for (end, node) in [edge.from, edge.to].into_iter().enumerate() {
                if end == 0 && !crate::layout::relationship_markers::both_ends(edge) {
                    continue;
                }
                if let Some(parent) = owner(graph, &graph.nodes[node]) {
                    insets[parent] = insets[parent].max(inset);
                }
            }
        }
    }
    insets
}

fn details(entry: &RouteEntry) -> Vec<Row> {
    vec![
        Row {
            qualified_text: None,
            text: format!(
                "Destination: {}",
                entry.destination.as_deref().unwrap_or("unresolved")
            ),
            target: None,
            moved: false,
            destination: true,
            address: false,
        },
        Row {
            qualified_text: match (&entry.target_kind, &entry.target) {
                (Some(kind), Some(target)) => {
                    Some(target_text(&format!("Target ({kind})"), target, true))
                }
                _ => None,
            },
            text: match (&entry.target_kind, &entry.target) {
                (Some(kind), Some(target)) => {
                    target_text(&format!("Target ({kind})"), target, false)
                }
                _ => "Target: unresolved / unsupported".into(),
            },
            target: entry.target.clone(),
            moved: false,
            destination: false,
            address: entry.target_kind.is_some() && entry.target.is_some(),
        },
    ]
}

fn target_text(label: &str, target: &TerraformEntityId, qualified: bool) -> String {
    let address = target.resource_address().formatted(qualified);
    let deposed = target
        .deposed_key
        .as_ref()
        .map(|key| format!(" (deposed: {key})"))
        .unwrap_or_default();
    format!("{label}: {address}{deposed}")
}

pub(crate) fn cards(graph: &Graph, parent: usize) -> Vec<Card<'_>> {
    let node = &graph.nodes[parent];
    let mut cards = Vec::new();
    for entry in &node.routes.entries {
        let inline = node.routes.inline;
        let mut rows = vec![Row {
            qualified_text: None,
            text: if inline {
                "Inline route (table action)".into()
            } else {
                "Route details".into()
            },
            target: None,
            moved: false,
            destination: false,
            address: false,
        }];
        rows.extend(details(entry));
        if !inline && node.routes.owner.is_none() {
            rows.push(Row {
                qualified_text: None,
                text: "Route Table: unresolved".into(),
                target: None,
                moved: false,
                destination: false,
                address: false,
            });
        }
        cards.push(Card {
            node: None,
            rows,
            inline,
        });
    }
    let mut children: Vec<_> = graph
        .nodes
        .iter()
        .enumerate()
        .filter(|(_, n)| owner(graph, n) == Some(parent))
        .collect();
    children.sort_by_key(|(_, n)| &n.entity.id);
    for (index, child) in children {
        let mut rows = vec![Row {
            qualified_text: Some(heading(child, true)),
            text: heading(child, false),
            target: None,
            moved: false,
            destination: false,
            address: true,
        }];
        if let Some(owner) = &child.member_owner {
            rows.push(Row {
                qualified_text: None,
                text: super::scope::label(child.entity.scope.as_ref(), &child.provider),
                target: None,
                moved: false,
                destination: false,
                address: false,
            });
            if !super::references::rows(child).iter().any(|row| {
                row.reference.attribute == owner.attribute
                    && row
                        .target
                        .is_some_and(|target| target.state.is_none() && target.id == owner.target)
            }) {
                rows.push(Row {
                    qualified_text: Some(target_text(owner.label, &owner.target, true)),
                    text: target_text(owner.label, &owner.target, false),
                    target: Some(owner.target.clone()),
                    moved: false,
                    destination: false,
                    address: true,
                });
            }
        }
        if let Some(previous) = &child.previous_address {
            rows.push(Row {
                qualified_text: None,
                text: super::text::moved_from(previous),
                target: None,
                moved: true,
                destination: false,
                address: true,
            });
        }
        for entry in &child.routes.entries {
            let mut detail = details(entry);
            if routed(graph, index) {
                detail.truncate(1);
                detail.push(Row {
                    qualified_text: None,
                    text: "Next hop".into(),
                    target: None,
                    moved: false,
                    destination: false,
                    address: false,
                });
            }
            rows.extend(detail);
        }
        let mut related: Vec<_> = graph
            .edges
            .iter()
            .filter_map(|e| {
                let (other, label) = if e.from == index {
                    (e.to, "Referenced by")
                } else if e.to == index {
                    (e.from, "References")
                } else {
                    return None;
                };
                if other == parent
                    || (child.member_owner.is_some()
                        && e.to == index
                        && super::references::rows(child).iter().any(|row| {
                            row.target.is_some_and(|target| {
                                target.state.is_none()
                                    && graph.nodes[other].entity.provenance.contains(&target.id)
                            })
                        }))
                    || child.routes.entries.iter().any(|r| {
                        r.target
                            .as_ref()
                            .is_some_and(|id| graph.nodes[other].entity.provenance.contains(id))
                    })
                {
                    return None;
                }
                Some((label, &graph.nodes[other]))
            })
            .collect();
        related.sort_by_key(|(label, n)| (*label, &n.entity.id));
        related.dedup();
        for (label, other) in related {
            rows.push(Row {
                qualified_text: Some(format!("{label}: {}", other.resource_address().qualified())),
                text: format!("{label}: {}", other.address),
                target: other.entity.provenance.first().cloned(),
                moved: false,
                destination: false,
                address: true,
            });
        }
        cards.push(Card {
            node: Some(child),
            rows,
            inline: false,
        });
    }
    for card in &mut cards {
        for row in &mut card.rows {
            if let Some(id) = &row.target {
                let status = match graph
                    .nodes
                    .iter()
                    .find(|n| n.entity.provenance.contains(id))
                {
                    None => " (unavailable)",
                    Some(n) if n.placement == crate::model::Placement::Logical => " (nonspatial)",
                    _ => "",
                };
                row.text.push_str(status);
                if let Some(text) = &mut row.qualified_text {
                    text.push_str(status);
                }
            }
        }
    }
    cards
}

pub(crate) fn height(graph: &Graph, node: usize) -> usize {
    let cards = cards(graph, node);
    if cards.is_empty() {
        0
    } else {
        28 + cards.iter().map(|c| c.height() + 12).sum::<usize>()
    }
}
