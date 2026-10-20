use crate::model::{Node, Placement, architecture::Architecture};
use std::collections::BTreeSet;

pub(crate) const TYPES: [&str; 7] = [
    "aws_ssm_parameter",
    "aws_cloudwatch_metric_alarm",
    "aws_iam_role",
    "aws_iam_user",
    "aws_iam_group",
    "aws_iam_openid_connect_provider",
    "aws_iam_saml_provider",
];

pub(crate) fn eligible(node: &Node) -> bool {
    node.provider.is_aws() && TYPES.contains(&node.resource_type.as_str())
}

pub(crate) fn validate(types: &[String]) -> Result<(), String> {
    if types.iter().any(|name| !TYPES.contains(&name.as_str())) {
        return Err("unsupported spatial type; see docs/configuration.md".into());
    }
    Ok(())
}

pub(crate) fn apply(original: &Architecture, projected: &mut Architecture, types: &[String]) {
    if types.is_empty() {
        return;
    }
    let targets: BTreeSet<_> = original
        .states
        .iter()
        .flat_map(|(state, graph)| {
            graph
                .nodes
                .iter()
                .filter(|node| eligible(node) && types.contains(&node.resource_type))
                .flat_map(move |node| node.entity.provenance.iter().map(move |id| (state, id)))
        })
        .collect();
    for (state, graph) in &mut projected.states {
        for node in &mut graph.0.nodes {
            let promoted = eligible(node) && types.contains(&node.resource_type);
            if promoted {
                node.placement = Placement::Spatial;
            }
            for reference in &mut node.references {
                reference.preserve_spatial |= promoted;
                for target in &mut reference.targets {
                    if targets.contains(&(target.state.as_ref().unwrap_or(state), &target.id)) {
                        target.placement = Placement::Spatial;
                        reference.preserve_spatial = true;
                    }
                }
            }
        }
    }
}
