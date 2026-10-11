use crate::model::architecture::Graph;

pub(crate) struct Projection {
    pub graph: Graph,
    pub nodes: Vec<usize>,
    pub edges: Vec<usize>,
}

pub(crate) fn project(source: &Graph) -> Projection {
    let mut nodes: Vec<_> = source
        .nodes
        .iter()
        .enumerate()
        .filter(|(_, node)| {
            node.placement != crate::model::Placement::Logical
                && super::routes::owner(source, node).is_none()
        })
        .map(|(index, _)| index)
        .collect();
    nodes.sort_by_key(|&i| (&source.nodes[i].entity.id, &source.nodes[i].address));
    let mut remap = vec![None; source.nodes.len()];
    for (new, &old) in nodes.iter().enumerate() {
        remap[old] = Some(new);
    }
    let mut graph = source.clone();
    graph.nodes = nodes
        .iter()
        .map(|&i| {
            let mut node = source.nodes[i].clone();
            node.routes.owner = None;
            node.routes.reserved_height = super::routes::height(source, i);
            let incoming = source
                .edges
                .iter()
                .filter(|e| e.to == i && super::routes::routed(source, e.from))
                .count();
            if incoming > 0 {
                let incident = source
                    .edges
                    .iter()
                    .filter(|e| e.from == i || e.to == i)
                    .count();
                node.routes.reserved_height = node.routes.reserved_height.max(
                    (41 + incident * crate::layout::relationship_markers::PORT_SPACING)
                        .saturating_sub(crate::layout::NODE_HEIGHT),
                );
            }
            node
        })
        .collect();
    let mut edges = Vec::new();
    graph.edges = source
        .edges
        .iter()
        .enumerate()
        .filter_map(|(index, edge)| {
            let mut edge = edge.clone();
            edge.from = remap[edge.from]?;
            edge.to = remap[edge.to]?;
            edges.push(index);
            Some(edge)
        })
        .collect();
    let ids: std::collections::BTreeSet<_> = graph.nodes.iter().map(|n| &n.entity.id).collect();
    graph
        .relationships
        .retain(|r| ids.contains(&r.from) && ids.contains(&r.to));
    Projection {
        graph,
        nodes,
        edges,
    }
}
