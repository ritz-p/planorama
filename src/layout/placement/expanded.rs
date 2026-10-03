use crate::model::Graph;

#[cfg(test)]
#[path = "../../../tests/unit/layout/placement/expanded.rs"]
mod tests;

pub(super) struct Vertex<'a> {
    pub rank: usize,
    pub module: &'a str,
    pub resource: Option<usize>,
}

pub(super) struct Expanded<'a> {
    pub vertices: Vec<Vertex<'a>>,
    pub incoming: Vec<Vec<usize>>,
    pub outgoing: Vec<Vec<usize>>,
}

impl<'a> Expanded<'a> {
    pub(super) fn new(graph: &'a Graph, ranks: &[usize]) -> Self {
        let mut vertices: Vec<_> = graph
            .nodes
            .iter()
            .enumerate()
            .map(|(index, node)| Vertex {
                rank: ranks[index],
                module: &node.module,
                resource: Some(index),
            })
            .collect();
        let mut edges = Vec::new();
        for edge in &graph.edges {
            let (source, target) = edge.endpoints();
            let mut previous = source;
            for rank in ranks[source] + 1..ranks[target] {
                let dummy = vertices.len();
                vertices.push(Vertex {
                    rank,
                    module: &graph.nodes[source].module,
                    resource: None,
                });
                edges.push((previous, dummy));
                previous = dummy;
            }
            edges.push((previous, target));
        }
        let mut incoming = vec![Vec::new(); vertices.len()];
        let mut outgoing = vec![Vec::new(); vertices.len()];
        for (source, target) in edges {
            incoming[target].push(source);
            outgoing[source].push(target);
        }
        Self {
            vertices,
            incoming,
            outgoing,
        }
    }
}
