use crate::model::{DiagnosticReason, Graph, ReferenceTarget, ResourceReference};
use crate::semantic::Input;

#[cfg(test)]
#[path = "../../../tests/unit/semantic/monitoring.rs"]
mod tests;

pub(super) fn present(raw: &Input, graph: &mut Graph) {
    for (index, alarm) in raw.nodes.iter().enumerate() {
        if !alarm.provider.is_aws()
            || alarm.resource_type != "aws_cloudwatch_metric_alarm"
            || alarm.deposed_key.is_some()
        {
            continue;
        }
        let verified: Vec<_> = raw
            .attributes
            .iter()
            .filter(|r| r.target == index && r.monitoring_verified)
            .collect();
        for reference in &mut graph.nodes[index].references {
            if reference.dependency
                || !(reference.attribute == "dimensions"
                    || reference.attribute.starts_with("dimensions."))
            {
                continue;
            }
            reference.label = "Monitors";
            if !verified.iter().any(|r| r.attribute == reference.attribute) {
                reference.complete = false;
                reference
                    .issues
                    .insert(DiagnosticReason::UnverifiedMonitoring);
            }
        }
        if verified.is_empty()
            && !graph.nodes[index]
                .references
                .iter()
                .any(|r| r.label == "Monitors")
        {
            graph.nodes[index].references.push(ResourceReference {
                kind: crate::model::ReferenceKind::Configured,
                preserve_spatial: false,
                show_spatial: false,
                label: "Monitors",
                attribute: "dimensions".into(),
                targets: Vec::new(),
                complete: false,
                dependency: false,
                issues: [DiagnosticReason::UnverifiedMonitoring].into(),
            });
        }
        for reference in verified {
            for &source in &reference.sources {
                let inverse = ResourceReference {
                    kind: crate::model::ReferenceKind::Configured,
                    preserve_spatial: false,
                    show_spatial: false,
                    label: "Monitored by",
                    attribute: reference.attribute.clone(),
                    targets: vec![ReferenceTarget {
                        state: None,
                        id: alarm.entity.provenance[0].clone(),
                        placement: alarm.placement,
                    }],
                    complete: true,
                    dependency: false,
                    issues: Default::default(),
                };
                if !graph.nodes[source].references.contains(&inverse) {
                    graph.nodes[source].references.push(inverse);
                }
            }
        }
    }
}
