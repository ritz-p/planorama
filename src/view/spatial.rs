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
        .filter(|(_, node)| node.placement != crate::model::Placement::Logical)
        .map(|(index, _)| index)
        .collect();
    nodes.sort_by_key(|&i| (&source.nodes[i].entity.id, &source.nodes[i].address));
    let mut remap = vec![None; source.nodes.len()];
    for (new, &old) in nodes.iter().enumerate() {
        remap[old] = Some(new);
    }
    let mut graph = source.clone();
    graph.nodes = nodes.iter().map(|&i| source.nodes[i].clone()).collect();
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
