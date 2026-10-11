use crate::model::{Graph, Node, RouteEntry, TerraformEntityId};

pub(crate) const WIDTH: usize = crate::layout::NODE_WIDTH - 24;
pub(crate) const LINE_LIMIT: usize = (WIDTH - 24) / 8;

pub(crate) struct Row {
    pub text: String,
    pub target: Option<TerraformEntityId>,
}

pub(crate) struct Card<'a> {
    pub node: Option<&'a Node>,
    pub rows: Vec<Row>,
    pub inline: bool,
}

impl Card<'_> {
    pub fn height(&self) -> usize {
        24 + self
            .rows
            .iter()
            .map(|r| lines(&r.text).len() * 18)
            .sum::<usize>()
    }
}

pub(crate) fn lines(text: &str) -> Vec<&str> {
    super::text::wrap(text, LINE_LIMIT)
}

pub(crate) fn owner(graph: &Graph, node: &Node) -> Option<usize> {
    let owner = node.routes.owner.as_ref()?;
    graph
        .nodes
        .iter()
        .position(|n| n.entity.provenance.contains(owner))
}

fn details(entry: &RouteEntry) -> Vec<Row> {
    vec![
        Row {
            text: format!(
                "Destination: {}",
                entry.destination.as_deref().unwrap_or("unresolved")
            ),
            target: None,
        },
        Row {
            text: match (&entry.target_kind, &entry.target) {
                (Some(kind), Some(target)) => format!("Target ({kind}): {}", target.address),
                _ => "Target: unresolved / unsupported".into(),
            },
            target: entry.target.clone(),
        },
    ]
}

pub(crate) fn cards(graph: &Graph, parent: usize) -> Vec<Card<'_>> {
    let node = &graph.nodes[parent];
    let mut cards = Vec::new();
    for entry in &node.routes.entries {
        let inline = node.routes.inline;
        let mut rows = vec![Row {
            text: if inline {
                "Inline route (table action)".into()
            } else {
                "Route details".into()
            },
            target: None,
        }];
        rows.extend(details(entry));
        if !inline && node.routes.owner.is_none() {
            rows.push(Row {
                text: "Route Table: unresolved".into(),
                target: None,
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
            text: format!(
                "{} [{}]",
                child.address,
                format!("{:?}", child.action).to_lowercase()
            ),
            target: None,
        }];
        for entry in &child.routes.entries {
            rows.extend(details(entry));
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
                text: format!("{label}: {}", other.address),
                target: other.entity.provenance.first().cloned(),
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
            if row
                .target
                .as_ref()
                .is_some_and(|id| !graph.nodes.iter().any(|n| n.entity.provenance.contains(id)))
            {
                row.text.push_str(" (unavailable)");
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
