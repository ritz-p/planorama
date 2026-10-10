mod associations;
mod classification;
mod components;
mod containment;
mod data;
pub(super) mod diagnostics;
mod icons;
mod routes;
mod scope;
mod security_groups;
pub(super) use scope::scope;

pub(crate) fn classify(resource_type: &str) -> crate::model::ResourceRole {
    classification::classify(resource_type)
}
pub(crate) fn icon_for(resource_type: &str) -> Option<crate::icons::ResourceIcon> {
    icons::icon_for(resource_type)
}
pub(super) fn transform(raw: crate::semantic::Input) -> crate::model::Graph {
    let graph = security_groups::infer(&raw, containment::infer(&raw));
    let contained = crate::semantic::Input { graph, ..raw };
    let mut graph = data::visible(associations::lower(&contained));
    graph.components = components::infer(&contained, &graph);
    graph
}
